use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortalError {
    #[error("Ağ bağlantısı kurulamadı: {0}")]
    Network(String),

    #[error("Captive portal kilitli, oturum açılması gerekiyor")]
    CaptiveLocked,

    #[error("Kimlik doğrulama başarısız: {0}")]
    AuthenticationFailed(String),

    #[error("Aktif bir portal oturumu bulunamadı")]
    SessionNotFound,

    #[error("Maksimum oturum sayısına ulaşıldı: {0}")]
    MaxSessionsReached(String),

    #[error("Portal yanıtı ayrıştırılamadı: {0}")]
    ParseError(String),

    #[error("Anahtarlık erişim hatası: {0}")]
    Keyring(String),

    #[error("Bilinmeyen bir hata oluştu: {0}")]
    Unknown(String),
}

impl PortalError {
    pub fn localized_message(&self, is_tr: bool) -> String {
        let is_gsb_unreachable = |s: &str| {
            s.contains("wifi.gsb.gov.tr")
                || s.contains("dns")
                || s.contains("error sending request")
                || s.contains("connection refused")
        };

        if is_tr {
            match self {
                Self::Network(s) if is_gsb_unreachable(s) => {
                    "GSBWIFI portalına ulaşılamadı. Lütfen GSBWIFI ağına bağlı olduğunuzdan emin olun.".to_string()
                }
                Self::Keyring(s) => {
                    let hint = if s.contains("ServiceUnknown")
                        || s.contains("No such secret service")
                        || s.contains("DBus")
                        || s.contains("Cannot autolaunch")
                    {
                        " (çalışan bir org.freedesktop.secrets sağlayıcısı -ör. gnome-keyring veya keepassxc- bulunamadı)"
                    } else {
                        ""
                    };
                    format!("Anahtarlık erişim hatası: {s}{hint}")
                }
                _ => self.to_string(),
            }
        } else {
            match self {
                Self::Network(s) if is_gsb_unreachable(s) => {
                    "Could not connect to GSBWIFI portal. Please make sure you are connected to the GSBWIFI network.".to_string()
                }
                Self::Network(s) => format!("Network connection failed: {s}"),
                Self::CaptiveLocked => "Captive portal locked, sign-in required".to_string(),
                Self::AuthenticationFailed(s) => format!("Authentication failed: {s}"),
                Self::SessionNotFound => "No active portal session found".to_string(),
                Self::MaxSessionsReached(s) => format!("Maximum active sessions reached: {s}"),
                Self::ParseError(s) => format!("Failed to parse portal response: {s}"),
                Self::Keyring(s) => {
                    let hint = if s.contains("ServiceUnknown")
                        || s.contains("No such secret service")
                        || s.contains("DBus")
                        || s.contains("Cannot autolaunch")
                    {
                        " (no running org.freedesktop.secrets provider -e.g., gnome-keyring or keepassxc- found)"
                    } else {
                        ""
                    };
                    format!("Keyring access error: {s}{hint}")
                }
                Self::Unknown(s) => format!("An unknown error occurred: {s}"),
            }
        }
    }
}
