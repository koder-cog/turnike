use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaPackage {
    pub package_type: String,
    pub total_mb: f64,
    pub remaining_mb: f64,
    pub start_date: String,
    pub expiration_date: String,
    pub remaining_time: String,
}

impl QuotaPackage {
    pub fn used_mb(&self) -> f64 {
        (self.total_mb - self.remaining_mb).max(0.0)
    }

    pub fn remaining_ratio(&self) -> f64 {
        if self.total_mb <= 0.0 {
            0.0
        } else {
            (self.remaining_mb / self.total_mb).clamp(0.0, 1.0)
        }
    }

    pub fn is_social(&self) -> bool {
        let lower = self.package_type.to_lowercase();
        lower.contains("social") || lower.contains("sosyal")
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub packages: Vec<QuotaPackage>,
    pub session_time: Option<String>,
    pub login_time: Option<String>,
    pub full_name: Option<String>,
}

impl QuotaInfo {
    pub fn total_mb(&self) -> f64 {
        self.packages.iter().map(|p| p.total_mb).sum()
    }

    pub fn remaining_mb(&self) -> f64 {
        self.packages.iter().map(|p| p.remaining_mb).sum()
    }

    pub fn used_mb(&self) -> f64 {
        (self.total_mb() - self.remaining_mb()).max(0.0)
    }

    pub fn remaining_ratio(&self) -> f64 {
        let total = self.total_mb();
        if total <= 0.0 {
            0.0
        } else {
            (self.remaining_mb() / total).clamp(0.0, 1.0)
        }
    }

    pub fn remaining_gb(&self) -> f64 {
        self.remaining_mb() / 1024.0
    }

    pub fn total_gb(&self) -> f64 {
        self.total_mb() / 1024.0
    }

    pub fn calculate_projection(
        &self,
        total_days_in_cycle: Option<f64>,
    ) -> Option<QuotaProjection> {
        let total_mb = self.total_mb();
        let remaining_mb = self.remaining_mb();
        if total_mb <= 0.0 {
            return None;
        }

        let days_remaining = self
            .packages
            .iter()
            .find_map(|p| parse_remaining_days(&p.remaining_time))?;

        let total_days = total_days_in_cycle.unwrap_or(31.0).max(1.0);
        let days_elapsed = (total_days - days_remaining).max(0.1);
        let used_mb = self.used_mb();

        let current_burn_rate_mb = used_mb / days_elapsed;
        let daily_allowance_mb = remaining_mb / days_remaining.max(0.1);

        let projected_days_until_depletion = if current_burn_rate_mb > 0.0 {
            Some(remaining_mb / current_burn_rate_mb)
        } else {
            None
        };

        let is_at_risk = match projected_days_until_depletion {
            Some(proj) => proj < days_remaining,
            None => false,
        };

        let remaining_ratio = self.remaining_ratio();
        let remaining_percent = remaining_ratio * 100.0;

        let risk_level = if !is_at_risk || remaining_percent > 85.0 {
            ProjectionRiskLevel::Safe
        } else if let Some(proj) = projected_days_until_depletion {
            if proj <= days_remaining * 0.6 || remaining_mb < 2048.0 {
                ProjectionRiskLevel::Critical
            } else {
                ProjectionRiskLevel::Moderate
            }
        } else {
            ProjectionRiskLevel::Safe
        };

        let projected_depletion_day = projected_days_until_depletion.map(|p| {
            let day = (1.0 + days_elapsed + p).round() as u32;
            day.clamp(1, 31)
        });

        Some(QuotaProjection {
            days_remaining,
            days_elapsed,
            total_days,
            daily_allowance_mb,
            current_burn_rate_mb,
            projected_days_until_depletion,
            projected_depletion_day,
            is_at_risk,
            risk_level,
            remaining_percent,
        })
    }
}

pub fn parse_remaining_days(raw: &str) -> Option<f64> {
    let mut nums = Vec::new();
    for token in raw.split(|c: char| !c.is_ascii_digit()) {
        if let Ok(n) = token.parse::<f64>() {
            nums.push(n);
        }
    }
    if nums.is_empty() {
        return None;
    }

    let days = nums[0];
    let hours = nums.get(1).copied().unwrap_or(0.0);
    let minutes = nums.get(2).copied().unwrap_or(0.0);
    let seconds = nums.get(3).copied().unwrap_or(0.0);

    Some(days + (hours / 24.0) + (minutes / 1440.0) + (seconds / 86400.0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProjectionRiskLevel {
    #[default]
    Safe,

    Moderate,

    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaProjection {
    pub days_remaining: f64,

    pub days_elapsed: f64,

    pub total_days: f64,

    pub daily_allowance_mb: f64,

    pub current_burn_rate_mb: f64,

    pub projected_days_until_depletion: Option<f64>,

    pub projected_depletion_day: Option<u32>,

    pub is_at_risk: bool,

    pub risk_level: ProjectionRiskLevel,

    pub remaining_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusJsonOutput {
    pub text: String,

    pub alt: String,

    pub tooltip: String,

    pub class: String,

    pub percentage: f64,

    pub online: bool,

    pub state: String,

    pub full_name: Option<String>,

    pub login_time: Option<String>,

    pub session_time: Option<String>,

    pub total_mb: f64,

    pub remaining_mb: f64,

    pub used_mb: f64,

    pub remaining_gb: f64,

    pub total_gb: f64,

    pub packages: Vec<QuotaPackageJson>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub projection: Option<QuotaProjectionJson>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaProjectionJson {
    pub days_remaining: f64,
    pub days_elapsed: f64,
    pub total_days: f64,
    pub daily_allowance_mb: f64,
    pub current_burn_rate_mb: f64,
    pub projected_days_until_depletion: Option<f64>,
    pub projected_depletion_day: Option<u32>,
    pub is_at_risk: bool,
    pub risk_level: String,
    pub remaining_percent: f64,
}

impl From<&QuotaProjection> for QuotaProjectionJson {
    fn from(p: &QuotaProjection) -> Self {
        let risk_str = match p.risk_level {
            ProjectionRiskLevel::Safe => "safe",
            ProjectionRiskLevel::Moderate => "moderate",
            ProjectionRiskLevel::Critical => "critical",
        };

        Self {
            days_remaining: p.days_remaining,
            days_elapsed: p.days_elapsed,
            total_days: p.total_days,
            daily_allowance_mb: p.daily_allowance_mb,
            current_burn_rate_mb: p.current_burn_rate_mb,
            projected_days_until_depletion: p.projected_days_until_depletion,
            projected_depletion_day: p.projected_depletion_day,
            is_at_risk: p.is_at_risk,
            risk_level: risk_str.to_string(),
            remaining_percent: p.remaining_percent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaPackageJson {
    pub package_type: String,
    pub total_mb: f64,
    pub remaining_mb: f64,
    pub used_mb: f64,
    pub remaining_ratio: f64,
    pub remaining_time: String,
    pub expiration_date: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_info_calculations() {
        let info = QuotaInfo {
            packages: vec![
                QuotaPackage {
                    package_type: "Sosyal Medya".to_string(),
                    total_mb: 16384.0,
                    remaining_mb: 8192.0,
                    start_date: "2026-10-01".to_string(),
                    expiration_date: "2026-10-31".to_string(),
                    remaining_time: "15 gün".to_string(),
                },
                QuotaPackage {
                    package_type: "Genel İnternet".to_string(),
                    total_mb: 16384.0,
                    remaining_mb: 4096.0,
                    start_date: "2026-10-01".to_string(),
                    expiration_date: "2026-10-31".to_string(),
                    remaining_time: "15 gün".to_string(),
                },
            ],
            session_time: Some("01:30:00".to_string()),
            login_time: Some("2026-10-15 08:00".to_string()),
            full_name: Some("Test Kullanıcı".to_string()),
        };

        assert_eq!(info.total_mb(), 32768.0);
        assert_eq!(info.remaining_mb(), 12288.0);
        assert_eq!(info.used_mb(), 20480.0);
        assert_eq!(info.total_gb(), 32.0);
        assert_eq!(info.remaining_gb(), 12.0);
        assert!((info.remaining_ratio() - 0.375).abs() < 0.001);
    }

    #[test]
    fn test_parse_remaining_days() {
        let days_turkish = parse_remaining_days("17 gn 0 sa 9 dk 52 sn").unwrap();
        assert!((days_turkish - 17.006).abs() < 0.01);

        let days_simple = parse_remaining_days("15 gün").unwrap();
        assert_eq!(days_simple, 15.0);

        let days_english = parse_remaining_days("10 days 12 hours").unwrap();
        assert_eq!(days_english, 10.5);

        assert!(parse_remaining_days("geçersiz").is_none());
    }

    #[test]
    fn test_calculate_projection_critical_and_safe() {
        let info_critical = QuotaInfo {
            packages: vec![QuotaPackage {
                package_type: "Toplam".to_string(),
                total_mb: 32768.0,
                remaining_mb: 12288.0,
                start_date: "01.10.2026".to_string(),
                expiration_date: "31.10.2026".to_string(),
                remaining_time: "17 gn 0 sa 0 dk 0 sn".to_string(),
            }],
            session_time: None,
            login_time: None,
            full_name: None,
        };

        let proj = info_critical.calculate_projection(Some(31.0)).unwrap();
        assert!(proj.is_at_risk);
        assert_eq!(proj.risk_level, ProjectionRiskLevel::Critical);
        assert!(proj.daily_allowance_mb < proj.current_burn_rate_mb);
        assert_eq!(proj.projected_depletion_day, Some(23));

        let info_safe = QuotaInfo {
            packages: vec![QuotaPackage {
                package_type: "Toplam".to_string(),
                total_mb: 32768.0,
                remaining_mb: 28000.0,
                start_date: "01.10.2026".to_string(),
                expiration_date: "31.10.2026".to_string(),
                remaining_time: "17 gn 0 sa 0 dk 0 sn".to_string(),
            }],
            session_time: None,
            login_time: None,
            full_name: None,
        };

        let proj_safe = info_safe.calculate_projection(Some(31.0)).unwrap();
        assert!(!proj_safe.is_at_risk);
        assert_eq!(proj_safe.risk_level, ProjectionRiskLevel::Safe);
    }
}
