use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessPointInfo {
    pub bssid: String,

    pub ssid: String,

    pub frequency_mhz: u32,

    pub is_5ghz: bool,

    pub channel: u32,

    pub strength: u8,

    pub max_bitrate_kbps: u32,

    pub is_active: bool,

    pub score: u32,
}

impl AccessPointInfo {
    pub fn calculate_score(freq_mhz: u32, strength: u8) -> u32 {
        if strength < 30 {
            return strength as u32;
        }
        let band_bonus = if freq_mhz >= 4900 { 35 } else { 0 };
        (strength as u32) + band_bonus
    }

    pub fn freq_to_channel(freq_mhz: u32) -> u32 {
        if (2412..=2484).contains(&freq_mhz) {
            if freq_mhz == 2484 {
                14
            } else {
                (freq_mhz - 2407) / 5
            }
        } else if (5170..=5825).contains(&freq_mhz) {
            (freq_mhz - 5000) / 5
        } else {
            0
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_min_score_diff() -> u32 {
    15
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadarConfig {
    pub enabled: bool,

    #[serde(default = "default_true")]
    pub prefer_5ghz: bool,

    #[serde(default = "default_min_score_diff")]
    pub min_score_diff: u32,
}

impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prefer_5ghz: true,
            min_score_diff: 15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_calculation_5ghz_bonus() {
        let score_5ghz = AccessPointInfo::calculate_score(5180, 60);
        let score_24ghz = AccessPointInfo::calculate_score(2437, 80);
        assert_eq!(score_5ghz, 95);
        assert_eq!(score_24ghz, 80);
        assert!(score_5ghz > score_24ghz);
    }

    #[test]
    fn test_weak_signal_no_bonus() {
        let score = AccessPointInfo::calculate_score(5180, 25);
        assert_eq!(score, 25);
    }

    #[test]
    fn test_channel_conversion() {
        assert_eq!(AccessPointInfo::freq_to_channel(2412), 1);
        assert_eq!(AccessPointInfo::freq_to_channel(2437), 6);
        assert_eq!(AccessPointInfo::freq_to_channel(2462), 11);
        assert_eq!(AccessPointInfo::freq_to_channel(5180), 36);
        assert_eq!(AccessPointInfo::freq_to_channel(5200), 40);
    }
}
