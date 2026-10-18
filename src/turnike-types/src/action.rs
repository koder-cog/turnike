use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LastActionItem {
    pub check_in_time: String,
    pub dormitory_name: String,
    pub download_mb: f64,
    pub upload_mb: f64,
    pub duration_seconds: u64,
    pub service: String,
}

impl LastActionItem {
    pub fn formatted_duration(&self, is_tr: bool) -> String {
        let secs = self.duration_seconds;
        if secs == 0 {
            return if is_tr {
                "< 1 dk".to_string()
            } else {
                "< 1 min".to_string()
            };
        }
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let rem_secs = secs % 60;

        if is_tr {
            if hours > 0 {
                format!("{hours} sa {mins} dk")
            } else if mins > 0 {
                format!("{mins} dk {rem_secs} sn")
            } else {
                format!("{rem_secs} sn")
            }
        } else if hours > 0 {
            format!("{hours}h {mins}m")
        } else if mins > 0 {
            format!("{mins}m {rem_secs}s")
        } else {
            format!("{rem_secs}s")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaqItem {
    pub question: String,
    pub answer: String,
}
