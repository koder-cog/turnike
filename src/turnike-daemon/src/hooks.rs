use std::collections::HashMap;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use turnike_types::HookConfig;

#[derive(Debug, Clone, Default)]
pub struct HookContext {
    pub event: &'static str,
    pub rtt_ms: Option<u64>,
    pub remaining_mb: Option<f64>,
    pub total_mb: Option<f64>,
    pub username: Option<String>,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
}

impl HookContext {
    pub fn new(event: &'static str) -> Self {
        Self {
            event,
            ..Default::default()
        }
    }

    pub fn to_env_vars(&self) -> HashMap<&'static str, String> {
        let mut map = HashMap::new();
        map.insert("TURNIKE_EVENT", self.event.to_string());

        if let Some(rtt) = self.rtt_ms {
            map.insert("TURNIKE_RTT_MS", rtt.to_string());
        }
        if let Some(rem) = self.remaining_mb {
            map.insert("TURNIKE_REMAINING_MB", format!("{rem:.2}"));
        }
        if let Some(total) = self.total_mb {
            map.insert("TURNIKE_TOTAL_MB", format!("{total:.2}"));
        }
        if let Some(ref u) = self.username {
            map.insert("TURNIKE_USERNAME", u.clone());
        }
        if let Some(ref s) = self.ssid {
            map.insert("TURNIKE_SSID", s.clone());
        }
        if let Some(ref b) = self.bssid {
            map.insert("TURNIKE_BSSID", b.clone());
        }

        map
    }
}

pub struct HookExecutor;

impl HookExecutor {
    pub fn trigger(hooks: &HookConfig, ctx: HookContext) {
        let cmd_opt = match ctx.event {
            "offpeak_enter" => hooks.on_offpeak_enter.clone(),
            "offpeak_exit" => hooks.on_offpeak_exit.clone(),
            "quota_critical" => hooks.on_quota_critical.clone(),
            "throttled" => hooks.on_throttled.clone(),
            "login" => hooks.on_login.clone(),
            "logout" => hooks.on_logout.clone(),
            _ => None,
        };

        if let Some(cmd) = cmd_opt {
            if cmd.trim().is_empty() {
                return;
            }
            tokio::spawn(async move {
                Self::execute_command(&cmd, &ctx).await;
            });
        }
    }

    pub async fn execute_command(cmd: &str, ctx: &HookContext) -> bool {
        let event = ctx.event;
        info!("Kanca tetikleniyor [{event}]: {cmd}");

        let envs = ctx.to_env_vars();
        let mut child = Command::new("sh");
        child.arg("-c").arg(cmd);

        for (k, v) in envs {
            child.env(k, v);
        }

        let timeout_dur = Duration::from_secs(15);
        match timeout(timeout_dur, child.status()).await {
            Ok(Ok(status)) => {
                if status.success() {
                    debug!("Kanca başarıyla tamamlandı [{event}]");
                    true
                } else {
                    warn!(
                        "Kanca başarısız çıkış koduyla bitti [{event}]: {:?}",
                        status.code()
                    );
                    false
                }
            }
            Ok(Err(e)) => {
                error!("Kanca süreci başlatılamadı [{event}]: {e}");
                false
            }
            Err(_) => {
                error!("Kanca zaman aşımına uğradı (15s), süreç sonlandırılıyor [{event}]");
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hook_execution_with_env() {
        let ctx = HookContext {
            event: "login",
            rtt_ms: Some(25),
            remaining_mb: Some(15000.0),
            total_mb: Some(32768.0),
            username: Some("12345678901".to_string()),
            ssid: Some("GSBWIFI".to_string()),
            bssid: Some("AA:BB:CC:DD:EE:FF".to_string()),
        };

        let cmd = "test \"$TURNIKE_EVENT\" = \"login\" && test \"$TURNIKE_RTT_MS\" = \"25\"";
        let success = HookExecutor::execute_command(cmd, &ctx).await;
        assert!(success);
    }

    #[tokio::test]
    async fn test_hook_failure_status() {
        let ctx = HookContext::new("test");
        let cmd = "exit 1";
        let success = HookExecutor::execute_command(cmd, &ctx).await;
        assert!(!success);
    }
}
