use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use zbus::connection::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

const NM_SERVICE: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const NM_INTERFACE: &str = "org.freedesktop.NetworkManager";
const NM_ACTIVE_CONN_INTERFACE: &str = "org.freedesktop.NetworkManager.Connection.Active";
const NM_SETTINGS_CONN_INTERFACE: &str = "org.freedesktop.NetworkManager.Settings.Connection";

const NM_SETTINGS_UPDATE2_FLAG_IN_MEMORY: u32 = 2;

#[derive(Clone)]
pub struct NetworkManagerClient {
    conn: Connection,
}

// NetworkManager nesneleri (Devices/X, ActiveConnection/Y, AccessPoint/Z) dinamik D-Bus
// yollarına sahiptir. zbus derive makroları ile onlarca statik proxy nesnesi ve asenkron builder
// üretip derleme süresini uzatmak yerine doğrudan call_method ile dinamik nesneler yönetilir.
impl NetworkManagerClient {
    pub async fn new() -> Result<Self, zbus::Error> {
        let conn = Connection::system().await?;
        Ok(Self { conn })
    }

    pub async fn get_active_connections(&self) -> Result<Vec<OwnedObjectPath>, zbus::Error> {
        let reply: OwnedValue = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                NM_PATH,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(NM_INTERFACE, "ActiveConnections"),
            )
            .await?
            .body()
            .deserialize()?;

        let paths: Vec<OwnedObjectPath> = reply.try_into().unwrap_or_default();

        Ok(paths)
    }

    pub async fn find_active_connection_by_ssid(
        &self,
        target_ssid: &str,
    ) -> Result<Option<OwnedObjectPath>, zbus::Error> {
        let active_paths = self.get_active_connections().await?;

        for path in active_paths {
            debug!("Aktif bağlantı inceleniyor: {}", path.as_str());
            let id_reply = self
                .conn
                .call_method(
                    Some(NM_SERVICE),
                    path.as_str(),
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &(NM_ACTIVE_CONN_INTERFACE, "Id"),
                )
                .await;

            if let Ok(msg) = id_reply
                && let Ok(id_variant) = msg.body().deserialize::<OwnedValue>()
                && let Ok(id_str) = String::try_from(id_variant)
                && id_str == target_ssid
            {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    pub async fn get_connection_id(
        &self,
        active_conn_path: &OwnedObjectPath,
    ) -> Result<Option<String>, zbus::Error> {
        let id_reply = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                active_conn_path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(NM_ACTIVE_CONN_INTERFACE, "Id"),
            )
            .await?;

        let id_variant: OwnedValue = id_reply.body().deserialize()?;
        Ok(String::try_from(id_variant).ok())
    }

    pub async fn get_connection_timestamp(
        &self,
        active_conn_path: &OwnedObjectPath,
    ) -> Result<Option<u64>, zbus::Error> {
        let conn_reply = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                active_conn_path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(NM_ACTIVE_CONN_INTERFACE, "Connection"),
            )
            .await?;

        let settings_path_val: OwnedValue = conn_reply.body().deserialize()?;
        let settings_path: OwnedObjectPath = match settings_path_val.try_into() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        let get_settings_reply = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                settings_path.as_str(),
                Some(NM_SETTINGS_CONN_INTERFACE),
                "GetSettings",
                &(),
            )
            .await?;

        let settings_dict: HashMap<String, HashMap<String, OwnedValue>> =
            get_settings_reply.body().deserialize()?;

        if let Some(conn_sec) = settings_dict.get("connection")
            && let Some(val) = conn_sec.get("timestamp")
        {
            let ts = u64::try_from(val.clone()).ok();
            return Ok(ts);
        }

        Ok(None)
    }

    pub async fn get_active_connection_start_time(
        &self,
        target_ssid: &str,
    ) -> Result<Option<u64>, zbus::Error> {
        if let Some(path) = self.find_active_connection_by_ssid(target_ssid).await? {
            self.get_connection_timestamp(&path).await
        } else {
            Ok(None)
        }
    }

    pub async fn get_active_wifi_ssid(&self) -> Result<Option<String>, zbus::Error> {
        if let Some(ap) = self.get_current_ap_info().await?
            && !ap.ssid.is_empty()
        {
            return Ok(Some(ap.ssid));
        }

        let active_paths = self.get_active_connections().await?;
        for path in active_paths {
            let type_reply = self
                .conn
                .call_method(
                    Some(NM_SERVICE),
                    path.as_str(),
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &(NM_ACTIVE_CONN_INTERFACE, "Type"),
                )
                .await;

            if let Ok(msg) = type_reply
                && let Ok(type_variant) = msg.body().deserialize::<OwnedValue>()
                && let Ok(type_str) = String::try_from(type_variant)
                && type_str == "802-11-wireless"
            {
                return self.get_connection_id(&path).await;
            }
        }

        Ok(None)
    }

    pub async fn set_connection_metered(
        &self,
        active_conn_path: &OwnedObjectPath,
        metered: bool,
    ) -> Result<(), zbus::Error> {
        let conn_reply = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                active_conn_path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(NM_ACTIVE_CONN_INTERFACE, "Connection"),
            )
            .await?;

        let settings_path_val: OwnedValue = conn_reply.body().deserialize()?;
        let settings_path: OwnedObjectPath = match settings_path_val.try_into() {
            Ok(p) => p,
            Err(e) => {
                warn!("Aktif bağlantının profil yolu alınamadı: {e}");
                return Ok(());
            }
        };

        let get_settings_reply = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                settings_path.as_str(),
                Some(NM_SETTINGS_CONN_INTERFACE),
                "GetSettings",
                &(),
            )
            .await?;

        let mut settings_dict: HashMap<String, HashMap<String, OwnedValue>> =
            get_settings_reply.body().deserialize()?;

        let metered_val: u32 = if metered { 1 } else { 0 };
        let connection_section = settings_dict.entry("connection".to_string()).or_default();
        connection_section.insert("metered".to_string(), OwnedValue::from(metered_val));

        let empty_args: HashMap<String, Value<'_>> = HashMap::new();

        let update_result = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                settings_path.as_str(),
                Some(NM_SETTINGS_CONN_INTERFACE),
                "Update2",
                &(
                    &settings_dict,
                    NM_SETTINGS_UPDATE2_FLAG_IN_MEMORY,
                    empty_args,
                ),
            )
            .await;

        match update_result {
            Ok(_) => {
                info!(
                    "Tarifeli bağlantı bayrağı uygulandı (metered={metered}): {}",
                    settings_path.as_str()
                );
                Ok(())
            }
            Err(e) => {
                error!("Tarifeli bağlantı ayarı uygulanamadı: {e}");
                Err(e)
            }
        }
    }

    pub async fn get_wireless_devices(&self) -> Result<Vec<OwnedObjectPath>, zbus::Error> {
        let all_devices: Vec<OwnedObjectPath> = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                NM_PATH,
                Some(NM_INTERFACE),
                "GetDevices",
                &(),
            )
            .await?
            .body()
            .deserialize()?;

        let mut wifi_devices = Vec::new();

        for dev_path in all_devices {
            let type_reply = self
                .conn
                .call_method(
                    Some(NM_SERVICE),
                    dev_path.as_str(),
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.NetworkManager.Device", "DeviceType"),
                )
                .await;

            if let Ok(msg) = type_reply
                && let Ok(val) = msg.body().deserialize::<OwnedValue>()
                && let Ok(dev_type) = u32::try_from(val)
                && dev_type == 2
            {
                wifi_devices.push(dev_path);
            }
        }

        Ok(wifi_devices)
    }

    // Dahili Wi-Fi kartı arızalı olup harici USB adaptör kullanan sistemlerde
    // körü körüne ilk cihaza bağlanmak pasif aygıta kilitlenmeye sebep olur.
    // Bu yüzden öncelikle aktif bir erişim noktasına bağlı olan aygıt tespit edilir.
    pub async fn get_primary_wireless_device(
        &self,
    ) -> Result<Option<OwnedObjectPath>, zbus::Error> {
        let wifi_devs = self.get_wireless_devices().await?;
        if wifi_devs.is_empty() {
            return Ok(None);
        }

        for dev in &wifi_devs {
            if let Ok(Some(_)) = self.get_active_access_point(dev).await {
                return Ok(Some(dev.clone()));
            }
        }

        Ok(wifi_devs.first().cloned())
    }

    pub async fn get_access_points(
        &self,
        device_path: &OwnedObjectPath,
    ) -> Result<Vec<OwnedObjectPath>, zbus::Error> {
        let paths: Vec<OwnedObjectPath> = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                device_path.as_str(),
                Some("org.freedesktop.NetworkManager.Device.Wireless"),
                "GetAllAccessPoints",
                &(),
            )
            .await?
            .body()
            .deserialize()?;

        Ok(paths)
    }

    pub async fn get_active_access_point(
        &self,
        device_path: &OwnedObjectPath,
    ) -> Result<Option<OwnedObjectPath>, zbus::Error> {
        let reply = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                device_path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(
                    "org.freedesktop.NetworkManager.Device.Wireless",
                    "ActiveAccessPoint",
                ),
            )
            .await?;

        let val: OwnedValue = reply.body().deserialize()?;
        let path: OwnedObjectPath = val
            .try_into()
            .unwrap_or_else(|_| OwnedObjectPath::default());

        if path.as_str() == "/" || path.as_str().is_empty() {
            Ok(None)
        } else {
            Ok(Some(path))
        }
    }

    pub async fn get_ap_info(
        &self,
        ap_path: &OwnedObjectPath,
        is_active: bool,
    ) -> Result<turnike_types::AccessPointInfo, zbus::Error> {
        let reply = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                ap_path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &("org.freedesktop.NetworkManager.AccessPoint",),
            )
            .await?;

        let props: HashMap<String, OwnedValue> = reply.body().deserialize()?;

        let bssid = props
            .get("HwAddress")
            .and_then(|v| String::try_from(v.clone()).ok())
            .unwrap_or_default();

        let ssid = props
            .get("Ssid")
            .and_then(|v| <Vec<u8>>::try_from(v.clone()).ok())
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();

        let frequency_mhz = props
            .get("Frequency")
            .and_then(|v| u32::try_from(v.clone()).ok())
            .unwrap_or(0);

        let strength = props
            .get("Strength")
            .and_then(|v| u8::try_from(v.clone()).ok())
            .unwrap_or(0);

        let max_bitrate_kbps = props
            .get("MaxBitrate")
            .and_then(|v| u32::try_from(v.clone()).ok())
            .unwrap_or(0);

        let is_5ghz = frequency_mhz >= 4900;
        let channel = turnike_types::AccessPointInfo::freq_to_channel(frequency_mhz);
        let score = turnike_types::AccessPointInfo::calculate_score(frequency_mhz, strength);

        Ok(turnike_types::AccessPointInfo {
            bssid,
            ssid,
            frequency_mhz,
            is_5ghz,
            channel,
            strength,
            max_bitrate_kbps,
            is_active,
            score,
        })
    }

    pub async fn get_current_ap_info(
        &self,
    ) -> Result<Option<turnike_types::AccessPointInfo>, zbus::Error> {
        let dev_path = match self.get_primary_wireless_device().await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let ap_path = match self.get_active_access_point(&dev_path).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let info = self.get_ap_info(&ap_path, true).await?;
        Ok(Some(info))
    }

    pub async fn activate_connection_specific_ap(
        &self,
        active_conn_path: &OwnedObjectPath,
        device_path: &OwnedObjectPath,
        target_ap_path: &OwnedObjectPath,
    ) -> Result<OwnedObjectPath, zbus::Error> {
        let conn_reply = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                active_conn_path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(NM_ACTIVE_CONN_INTERFACE, "Connection"),
            )
            .await?;

        let settings_path_val: OwnedValue = conn_reply.body().deserialize()?;
        let settings_path: OwnedObjectPath = settings_path_val.try_into()?;

        info!(
            "Erişim noktası geçişi başlatılıyor: Cihaz={}, Hedef AP={}",
            device_path.as_str(),
            target_ap_path.as_str()
        );

        let new_active_conn: OwnedObjectPath = self
            .conn
            .call_method(
                Some(NM_SERVICE),
                NM_PATH,
                Some(NM_INTERFACE),
                "ActivateConnection",
                &(settings_path, device_path, target_ap_path),
            )
            .await?
            .body()
            .deserialize()?;

        info!(
            "Erişim noktasına başarıyla geçildi: {}",
            new_active_conn.as_str()
        );
        Ok(new_active_conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nm_live_active_connection() {
        if let Ok(nm) = NetworkManagerClient::new().await {
            let ssid_res = nm.get_active_wifi_ssid().await;
            assert!(
                ssid_res.is_ok(),
                "D-Bus üzerinden Wi-Fi SSID sorgusu hata üretmemeli"
            );
            if let Ok(Some(ref s)) = ssid_res {
                let ts_res = nm.get_active_connection_start_time(s).await;
                assert!(
                    ts_res.is_ok(),
                    "Aktif bağlantı başlangıç zamanı sorgusu hata üretmemeli"
                );
                if let Ok(Some(ts)) = ts_res {
                    assert!(
                        ts > 0,
                        "Bağlantı zaman damgası pozitif bir Unix zamanı olmalı"
                    );
                }
            }
        }
    }
}
