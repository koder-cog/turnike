use crate::nm::NetworkManagerClient;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info, warn};
use turnike_types::{AccessPointInfo, RadarConfig};
use zbus::zvariant::OwnedObjectPath;

#[derive(Debug, Error)]
pub enum RadarError {
    #[error("Sistemde kablosuz ağ cihazı (Wi-Fi) bulunamadı")]
    NoWirelessDevice,

    #[error("Aktif '{0}' bağlantısı bulunamadı")]
    NoActiveConnection(String),

    #[error("Belirtilen erişim noktası bulunamadı: {0}")]
    ApNotFound(String),

    #[error("NetworkManager D-Bus hatası: {0}")]
    Dbus(#[from] zbus::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoamDecision {
    pub from_bssid: String,
    pub to_bssid: String,
    pub from_score: u32,
    pub to_score: u32,
    pub is_5ghz: bool,
}

pub struct BssidRadar {
    nm: NetworkManagerClient,
    config: RadarConfig,
    target_ssid: String,
    last_switch_time: Option<Instant>,
    min_switch_interval: Duration,
}

impl BssidRadar {
    pub fn new(nm: NetworkManagerClient, config: RadarConfig, target_ssid: String) -> Self {
        Self {
            nm,
            config,
            target_ssid,
            last_switch_time: None,
            min_switch_interval: Duration::from_secs(120),
        }
    }

    pub fn update_config(&mut self, config: RadarConfig) {
        self.config = config;
    }

    pub async fn scan_all_aps(
        &self,
    ) -> Result<Vec<(AccessPointInfo, OwnedObjectPath, OwnedObjectPath)>, RadarError> {
        let dev_path = self
            .nm
            .get_primary_wireless_device()
            .await?
            .ok_or(RadarError::NoWirelessDevice)?;

        let active_ap_path = self.nm.get_active_access_point(&dev_path).await?;
        let ap_paths = self.nm.get_access_points(&dev_path).await?;

        let mut results = Vec::new();
        for ap_path in ap_paths {
            let is_active = active_ap_path
                .as_ref()
                .map(|p| p.as_str() == ap_path.as_str())
                .unwrap_or(false);

            if let Ok(info) = self.nm.get_ap_info(&ap_path, is_active).await
                && (info.ssid == self.target_ssid || self.target_ssid.is_empty())
            {
                results.push((info, ap_path, dev_path.clone()));
            }
        }

        results.sort_by_key(|a| std::cmp::Reverse(a.0.score));
        Ok(results)
    }

    pub async fn check_and_optimize(&mut self) -> Result<Option<RoamDecision>, RadarError> {
        if !self.config.enabled {
            return Ok(None);
        }

        if let Some(last) = self.last_switch_time
            && last.elapsed() < self.min_switch_interval
        {
            debug!(
                "Radar histerezis bekleme süresinde, geçiş atlandı ({:?} kaldı)",
                self.min_switch_interval - last.elapsed()
            );
            return Ok(None);
        }

        let active_conn = match self
            .nm
            .find_active_connection_by_ssid(&self.target_ssid)
            .await?
        {
            Some(p) => p,
            None => return Err(RadarError::NoActiveConnection(self.target_ssid.clone())),
        };

        let aps = self.scan_all_aps().await?;
        if aps.is_empty() {
            return Ok(None);
        }

        let active_entry = aps.iter().find(|(info, _, _)| info.is_active);
        let best_entry = aps.first();

        let (best_info, best_ap_path, dev_path) = match best_entry {
            Some(e) => e,
            None => return Ok(None),
        };

        if best_info.is_active {
            debug!(
                "Zaten en yüksek skorlu erişim noktasına ({}) bağlısınız",
                best_info.bssid
            );
            return Ok(None);
        }

        let active_score = active_entry.map(|(info, _, _)| info.score).unwrap_or(0);
        let active_bssid = active_entry
            .map(|(info, _, _)| info.bssid.clone())
            .unwrap_or_else(|| "Bilinmiyor".to_string());

        let score_diff = best_info.score.saturating_sub(active_score);

        if score_diff < self.config.min_score_diff {
            debug!(
                "Aday AP ({}) skor farkı yetersiz (Aday={}, Aktif={}, Fark={}, Baraj={})",
                best_info.bssid,
                best_info.score,
                active_score,
                score_diff,
                self.config.min_score_diff
            );
            return Ok(None);
        }

        info!(
            "Daha temiz erişim noktası tespit edildi: {} (Skor: {}) -> {} (Skor: {}, 5GHz: {})",
            active_bssid, active_score, best_info.bssid, best_info.score, best_info.is_5ghz
        );

        match self
            .nm
            .activate_connection_specific_ap(&active_conn, dev_path, best_ap_path)
            .await
        {
            Ok(_) => {
                self.last_switch_time = Some(Instant::now());
                Ok(Some(RoamDecision {
                    from_bssid: active_bssid,
                    to_bssid: best_info.bssid.clone(),
                    from_score: active_score,
                    to_score: best_info.score,
                    is_5ghz: best_info.is_5ghz,
                }))
            }
            Err(e) => {
                warn!("Erişim noktası geçişi başarısız oldu: {e}");
                Err(RadarError::Dbus(e))
            }
        }
    }

    pub async fn switch_to_bssid(
        &mut self,
        target_bssid: &str,
    ) -> Result<AccessPointInfo, RadarError> {
        let active_conn = match self
            .nm
            .find_active_connection_by_ssid(&self.target_ssid)
            .await?
        {
            Some(p) => p,
            None => return Err(RadarError::NoActiveConnection(self.target_ssid.clone())),
        };

        let aps = self.scan_all_aps().await?;
        let target_entry = aps
            .into_iter()
            .find(|(info, _, _)| info.bssid.eq_ignore_ascii_case(target_bssid))
            .ok_or_else(|| RadarError::ApNotFound(target_bssid.to_string()))?;

        let (target_info, ap_path, dev_path) = target_entry;
        self.nm
            .activate_connection_specific_ap(&active_conn, &dev_path, &ap_path)
            .await?;

        self.last_switch_time = Some(Instant::now());
        Ok(target_info)
    }
}
