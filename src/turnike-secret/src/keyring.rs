use keyring::Entry;
use tracing::{debug, info, warn};
use turnike_types::{Credentials, PortalError};

const SERVICE_NAME: &str = "turnike";
// T.C. Kimlik Numarası KVKK kapsamında hassas kişisel veri olduğundan asla diske düz metin yazılamaz.
const CREDENTIALS_KEY: &str = "credentials";
const LEGACY_ACTIVE_USER_KEY: &str = "__active_user__";
const LEGACY_ACCOUNTS_INDEX_KEY: &str = "__accounts_index__";

pub struct SecretStore;

impl SecretStore {
    pub fn save(creds: &Credentials) -> Result<(), PortalError> {
        info!(
            "Kimlik bilgisi anahtarlığa kaydediliyor: {}",
            creds.masked_username()
        );

        let entry = Entry::new(SERVICE_NAME, CREDENTIALS_KEY)
            .map_err(|e| PortalError::Keyring(e.to_string()))?;
        let payload =
            serde_json::to_string(creds).map_err(|e| PortalError::Keyring(e.to_string()))?;
        entry
            .set_password(&payload)
            .map_err(|e| PortalError::Keyring(e.to_string()))?;

        if let Ok(legacy_active) = Entry::new(SERVICE_NAME, LEGACY_ACTIVE_USER_KEY) {
            if let Ok(old_user) = legacy_active.get_password()
                && let Ok(old_pass_entry) = Entry::new(SERVICE_NAME, &old_user)
            {
                let _ = old_pass_entry.delete_credential();
            }
            let _ = legacy_active.delete_credential();
        }
        if let Ok(legacy_index) = Entry::new(SERVICE_NAME, LEGACY_ACCOUNTS_INDEX_KEY) {
            let _ = legacy_index.delete_credential();
        }

        debug!("Kimlik bilgisi sistem anahtarlığına işlendi");
        Ok(())
    }

    pub fn load() -> Result<Option<Credentials>, PortalError> {
        let entry = Entry::new(SERVICE_NAME, CREDENTIALS_KEY)
            .map_err(|e| PortalError::Keyring(e.to_string()))?;

        match entry.get_password() {
            Ok(payload) => match serde_json::from_str::<Credentials>(&payload) {
                Ok(creds) => Ok(Some(creds)),
                Err(e) => {
                    warn!("Anahtarlıktaki kimlik verisi ayrıştırılamadı: {e}");
                    Ok(None)
                }
            },
            Err(keyring::Error::NoEntry) => Self::load_legacy(),
            Err(e) => Err(PortalError::Keyring(e.to_string())),
        }
    }

    fn load_legacy() -> Result<Option<Credentials>, PortalError> {
        let legacy_active = match Entry::new(SERVICE_NAME, LEGACY_ACTIVE_USER_KEY) {
            Ok(entry) => entry,
            Err(_) => return Ok(None),
        };

        let username = match legacy_active.get_password() {
            Ok(u) => u,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(e) => return Err(PortalError::Keyring(e.to_string())),
        };

        let pass_entry = match Entry::new(SERVICE_NAME, &username) {
            Ok(entry) => entry,
            Err(_) => return Ok(None),
        };

        let password = match pass_entry.get_password() {
            Ok(p) => p,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(e) => return Err(PortalError::Keyring(e.to_string())),
        };

        let creds = Credentials::new(username, password);
        let _ = Self::save(&creds);
        Ok(Some(creds))
    }

    pub fn clear() -> Result<(), PortalError> {
        info!("Kayıtlı kimlik bilgisi anahtarlıktan siliniyor");

        if let Ok(entry) = Entry::new(SERVICE_NAME, CREDENTIALS_KEY) {
            let _ = entry.delete_credential();
        }

        if let Ok(legacy_active) = Entry::new(SERVICE_NAME, LEGACY_ACTIVE_USER_KEY) {
            if let Ok(username) = legacy_active.get_password()
                && let Ok(pass_entry) = Entry::new(SERVICE_NAME, &username)
            {
                let _ = pass_entry.delete_credential();
            }
            let _ = legacy_active.delete_credential();
        }

        if let Ok(legacy_index) = Entry::new(SERVICE_NAME, LEGACY_ACCOUNTS_INDEX_KEY) {
            let _ = legacy_index.delete_credential();
        }

        Ok(())
    }

    pub async fn load_async() -> Result<Option<Credentials>, PortalError> {
        tokio::task::spawn_blocking(Self::load)
            .await
            .map_err(|e| PortalError::Keyring(e.to_string()))?
    }

    pub async fn save_async(creds: Credentials) -> Result<(), PortalError> {
        tokio::task::spawn_blocking(move || Self::save(&creds))
            .await
            .map_err(|e| PortalError::Keyring(e.to_string()))?
    }

    pub async fn clear_async() -> Result<(), PortalError> {
        tokio::task::spawn_blocking(Self::clear)
            .await
            .map_err(|e| PortalError::Keyring(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_json_payload_roundtrip() {
        let creds = Credentials::new("12345678901", "GuvenliParola!99");
        let serialized = serde_json::to_string(&creds).unwrap();
        let deserialized: Credentials = serde_json::from_str(&serialized).unwrap();
        assert_eq!(creds, deserialized);
        assert_eq!(deserialized.username, "12345678901");
        assert_eq!(deserialized.password, "GuvenliParola!99");
    }

    #[test]
    fn test_invalid_json_payload_handling() {
        let corrupt = "{ invalid json }";
        assert!(serde_json::from_str::<Credentials>(corrupt).is_err());
    }
}
