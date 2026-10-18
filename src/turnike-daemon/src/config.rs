use std::fs::{File, create_dir_all};
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::{debug, warn};
use turnike_types::DaemonConfig;

const CONFIG_DIR_NAME: &str = "turnike";
const CONFIG_FILE_NAME: &str = "daemon.json";

pub fn config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME)
}

pub fn load_config() -> DaemonConfig {
    let path = config_path();
    if !path.exists() {
        debug!(
            "Yapılandırma dosyası bulunamadı, varsayılan ayarlar kullanılıyor: {:?}",
            path
        );
        return DaemonConfig::default();
    }

    match File::open(&path) {
        Ok(mut file) => {
            let mut contents = String::new();
            if let Err(e) = file.read_to_string(&mut contents) {
                warn!("Yapılandırma dosyası okunamadı: {e}, varsayılanlara dönülüyor");
                return DaemonConfig::default();
            }

            serde_json::from_str(&contents).unwrap_or_else(|e| {
                warn!("Yapılandırma JSON verisi ayrıştırılamadı: {e}, varsayılanlara dönülüyor");
                DaemonConfig::default()
            })
        }
        Err(e) => {
            warn!("Yapılandırma dosyası açılamadı: {e}");
            DaemonConfig::default()
        }
    }
}

pub fn save_config(config: &DaemonConfig) -> Result<(), std::io::Error> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(config)?;
    let mut file = File::create(&path)?;
    file.write_all(json.as_bytes())?;
    debug!("Yapılandırma kaydedildi: {:?}", path);
    Ok(())
}
