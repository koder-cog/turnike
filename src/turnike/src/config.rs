use std::fs::{File, create_dir_all};
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::{debug, warn};
use turnike_types::GuiConfig;

const CONFIG_DIR_NAME: &str = "turnike";
const GUI_CONFIG_FILE_NAME: &str = "gui.json";

pub fn gui_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join(CONFIG_DIR_NAME).join(GUI_CONFIG_FILE_NAME)
}

pub fn load_gui_config() -> GuiConfig {
    let path = gui_config_path();
    if !path.exists() {
        return GuiConfig::default();
    }

    match File::open(&path) {
        Ok(mut file) => {
            let mut contents = String::new();
            if let Err(e) = file.read_to_string(&mut contents) {
                warn!("GUI yapılandırma dosyası okunamadı: {e}");
                return GuiConfig::default();
            }

            serde_json::from_str(&contents).unwrap_or_else(|e| {
                warn!("GUI JSON ayrıştırılamadı: {e}");
                GuiConfig::default()
            })
        }
        Err(_) => GuiConfig::default(),
    }
}

pub fn save_gui_config(config: &GuiConfig) -> Result<(), std::io::Error> {
    let path = gui_config_path();
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(config)?;
    let mut file = File::create(&path)?;
    file.write_all(json.as_bytes())?;
    debug!("GUI yapılandırması kaydedildi: {:?}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_config_serialization() {
        let config = GuiConfig {
            language: Some("tr".to_string()),
            color_scheme: 2,
        };
        let json = serde_json::to_string(&config).expect("Serileştirme başarılı olmalı");
        let parsed: GuiConfig = serde_json::from_str(&json).expect("Ayrıştırma başarılı olmalı");
        assert_eq!(parsed, config);
    }

    #[test]
    fn test_gui_config_defaults() {
        let empty_json = "{}";
        let parsed: GuiConfig =
            serde_json::from_str(empty_json).expect("Varsayılanlar ayrıştırılabilmeli");
        assert_eq!(parsed.language, None);
        assert_eq!(parsed.color_scheme, 0);
    }
}
