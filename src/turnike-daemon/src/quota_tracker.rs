use crate::hooks::{HookContext, HookExecutor};
use crate::notify::send_desktop_notification_full;
use tracing::info;
use turnike_types::{DaemonConfig, QuotaInfo, QuotaThresholdMode};

#[derive(Debug, Clone, Default)]
pub struct QuotaNotificationTracker {
    last_notification_id: u32,
    threshold_notified: bool,
    critical_notified: bool,
    throttled_notified: bool,
    last_remaining_mb: f64,
}

impl QuotaNotificationTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.threshold_notified = false;
        self.critical_notified = false;
        self.throttled_notified = false;
        self.last_remaining_mb = 0.0;
    }

    pub async fn check_and_notify(&mut self, quota: &QuotaInfo, config: &DaemonConfig) {
        if !config.notify_on_quota {
            return;
        }

        let total_mb = quota.total_mb();
        let remaining_mb = quota.remaining_mb();

        if total_mb <= 0.0 {
            return;
        }

        if remaining_mb > self.last_remaining_mb + 200.0 && self.last_remaining_mb > 0.0 {
            self.threshold_notified = false;
            self.critical_notified = false;
            self.throttled_notified = false;
        }
        self.last_remaining_mb = remaining_mb;

        let remaining_ratio = quota.remaining_ratio();
        let remaining_percent = remaining_ratio * 100.0;
        let remaining_gb = quota.remaining_gb();

        let is_below_threshold = match config.quota_threshold_mode {
            QuotaThresholdMode::Percentage => remaining_percent <= config.quota_threshold_value,
            QuotaThresholdMode::Gigabytes => remaining_gb <= config.quota_threshold_value,
        };

        let is_tr = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_MESSAGES"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default()
            .to_lowercase()
            .starts_with("tr");

        if is_below_threshold && !self.threshold_notified {
            let percent_val = remaining_percent.round() as u64;
            let (summary, body) = if is_tr {
                let s = "Düşük Kota";
                let b = match config.quota_threshold_mode {
                    QuotaThresholdMode::Percentage => {
                        format!("%{percent_val} kaldı ({remaining_gb:.1} GB)")
                    }
                    QuotaThresholdMode::Gigabytes => {
                        format!("{remaining_gb:.1} GB kaldı")
                    }
                };
                (s, b)
            } else {
                let s = "Low Quota";
                let b = match config.quota_threshold_mode {
                    QuotaThresholdMode::Percentage => {
                        format!("%{percent_val} remaining ({remaining_gb:.1} GB)")
                    }
                    QuotaThresholdMode::Gigabytes => {
                        format!("{remaining_gb:.1} GB remaining")
                    }
                };
                (s, b)
            };

            info!("Düşük kota bildirimi gönderiliyor: {body}");
            self.last_notification_id = send_desktop_notification_full(
                summary,
                &body,
                "dialog-warning-symbolic",
                self.last_notification_id,
                1,
            )
            .await;
            self.threshold_notified = true;
        }

        let is_critical = remaining_ratio <= 0.05 || remaining_gb <= 1.0;
        if is_critical && !self.critical_notified {
            let (summary, body) = if is_tr {
                (
                    "Kritik Kota",
                    format!("{remaining_gb:.1} GB kaldı, yakında hız düşürülecek"),
                )
            } else {
                (
                    "Critical Quota",
                    format!("{remaining_gb:.1} GB remaining, speed will drop soon"),
                )
            };

            info!("Kritik kota bildirimi gönderiliyor: {body}");
            self.last_notification_id = send_desktop_notification_full(
                summary,
                &body,
                "dialog-error-symbolic",
                self.last_notification_id,
                2,
            )
            .await;
            self.critical_notified = true;

            HookExecutor::trigger(
                &config.hooks,
                HookContext {
                    event: "quota_critical",
                    remaining_mb: Some(remaining_mb),
                    total_mb: Some(total_mb),
                    ..Default::default()
                },
            );
        }

        let is_throttled = remaining_mb <= 0.01;
        if is_throttled && !self.throttled_notified {
            let (summary, body) = if is_tr {
                ("Kota Tükendi", "Kota tükendi, hız düşürüldü".to_string())
            } else {
                (
                    "Quota Exhausted",
                    "Quota exhausted, speed reduced".to_string(),
                )
            };

            info!("Kota tükendi bildirimi gönderiliyor: {body}");
            self.last_notification_id = send_desktop_notification_full(
                summary,
                &body,
                "dialog-error-symbolic",
                self.last_notification_id,
                2,
            )
            .await;
            self.throttled_notified = true;

            HookExecutor::trigger(
                &config.hooks,
                HookContext {
                    event: "throttled",
                    remaining_mb: Some(remaining_mb),
                    total_mb: Some(total_mb),
                    ..Default::default()
                },
            );
        }
    }

    pub fn is_threshold_notified(&self) -> bool {
        self.threshold_notified
    }

    pub fn is_critical_notified(&self) -> bool {
        self.critical_notified
    }

    pub fn is_throttled_notified(&self) -> bool {
        self.throttled_notified
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turnike_types::QuotaPackage;

    fn sample_quota(remaining_mb: f64, total_mb: f64) -> QuotaInfo {
        QuotaInfo {
            packages: vec![QuotaPackage {
                package_type: "Genel İnternet".to_string(),
                total_mb,
                remaining_mb,
                start_date: "2026-10-01".to_string(),
                expiration_date: "2026-10-31".to_string(),
                remaining_time: "10 gün".to_string(),
            }],
            session_time: None,
            login_time: None,
            full_name: None,
        }
    }

    #[test]
    fn test_tracker_hysteresis_logic() {
        let mut tracker = QuotaNotificationTracker::new();
        let config = DaemonConfig {
            notify_on_quota: true,
            quota_threshold_mode: QuotaThresholdMode::Percentage,
            quota_threshold_value: 15.0,
            ..Default::default()
        };

        let q_safe = sample_quota(16384.0, 32768.0);
        let remaining_ratio = q_safe.remaining_ratio();
        let remaining_percent = remaining_ratio * 100.0;
        let is_below = remaining_percent <= config.quota_threshold_value;
        assert!(!is_below);
        assert!(!tracker.is_threshold_notified());

        let q_low = sample_quota(3276.0, 32768.0);
        let ratio_low = q_low.remaining_ratio();
        assert!(ratio_low * 100.0 <= config.quota_threshold_value);

        tracker.threshold_notified = true;
        assert!(tracker.is_threshold_notified());

        tracker.reset();
        assert!(!tracker.is_threshold_notified());
        assert!(!tracker.is_critical_notified());
    }
}
