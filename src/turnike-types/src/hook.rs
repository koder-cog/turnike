use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HookConfig {
    pub on_offpeak_enter: Option<String>,

    pub on_offpeak_exit: Option<String>,

    pub on_quota_critical: Option<String>,

    pub on_throttled: Option<String>,

    pub on_login: Option<String>,

    pub on_logout: Option<String>,
}

fn default_target_host() -> String {
    "1.1.1.1:443".to_string()
}

fn default_start_hour() -> u8 {
    2
}

fn default_end_hour() -> u8 {
    6
}

fn default_max_rtt() -> u32 {
    50
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffPeakConfig {
    pub enabled: bool,

    #[serde(default = "default_start_hour")]
    pub start_hour: u8,

    #[serde(default = "default_end_hour")]
    pub end_hour: u8,

    #[serde(default = "default_max_rtt")]
    pub max_rtt_ms: u32,

    #[serde(default = "default_target_host")]
    pub target_host: String,
}

impl Default for OffPeakConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            start_hour: 2,
            end_hour: 6,
            max_rtt_ms: 50,
            target_host: "1.1.1.1:443".to_string(),
        }
    }
}
