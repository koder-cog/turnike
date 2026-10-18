use crate::hook::{HookConfig, OffPeakConfig};
use crate::radar::RadarConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QuotaThresholdMode {
    #[default]
    Percentage,

    Gigabytes,
}

fn default_true() -> bool {
    true
}

fn default_threshold_value() -> f64 {
    15.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub auto_login: bool,

    pub auto_preempt: bool,

    pub auto_metered: bool,

    pub target_ssid: String,

    pub check_interval_secs: u64,

    #[serde(default = "default_true")]
    pub notify_on_quota: bool,

    #[serde(default)]
    pub quota_threshold_mode: QuotaThresholdMode,

    #[serde(default = "default_threshold_value")]
    pub quota_threshold_value: f64,

    #[serde(default)]
    pub auto_logout_on_screen_lock: bool,

    #[serde(default)]
    pub hooks: HookConfig,

    #[serde(default)]
    pub offpeak: OffPeakConfig,

    #[serde(default)]
    pub radar: RadarConfig,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            auto_login: true,
            auto_preempt: true,
            auto_metered: true,
            target_ssid: "GSBWIFI".to_string(),
            check_interval_secs: 30,
            notify_on_quota: true,
            quota_threshold_mode: QuotaThresholdMode::Percentage,
            quota_threshold_value: 15.0,
            auto_logout_on_screen_lock: false,
            hooks: HookConfig::default(),
            offpeak: OffPeakConfig::default(),
            radar: RadarConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GuiConfig {
    #[serde(default)]
    pub language: Option<String>,

    #[serde(default)]
    pub color_scheme: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backward_compatible_deserialization() {
        let legacy_json = r#"{
            "auto_login": true,
            "auto_preempt": false,
            "auto_metered": true,
            "target_ssid": "GSBWIFI",
            "check_interval_secs": 45
        }"#;

        let parsed: DaemonConfig =
            serde_json::from_str(legacy_json).expect("Ayrıştırma başarılı olmalı");
        assert!(parsed.notify_on_quota);
        assert_eq!(parsed.quota_threshold_mode, QuotaThresholdMode::Percentage);
        assert_eq!(parsed.quota_threshold_value, 15.0);
        assert_eq!(parsed.check_interval_secs, 45);
        assert!(!parsed.auto_preempt);
        assert!(!parsed.auto_logout_on_screen_lock);
        assert_eq!(parsed.hooks, HookConfig::default());
        assert!(!parsed.offpeak.enabled);
        assert_eq!(parsed.offpeak.start_hour, 2);
        assert_eq!(parsed.offpeak.end_hour, 6);
        assert_eq!(parsed.offpeak.max_rtt_ms, 50);
        assert_eq!(parsed.offpeak.target_host, "1.1.1.1:443");
        assert!(!parsed.radar.enabled);
        assert!(parsed.radar.prefer_5ghz);
        assert_eq!(parsed.radar.min_score_diff, 15);
    }
}
