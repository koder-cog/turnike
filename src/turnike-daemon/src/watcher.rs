use crate::hooks::{HookContext, HookExecutor};
use crate::latency::{DEFAULT_TARGET_SECONDARY, measure_tcp_rtt_with_fallback};
use crate::nm::NetworkManagerClient;
use crate::notify::send_desktop_notification;
use crate::preemption::PreemptionTracker;
use crate::quota_tracker::QuotaNotificationTracker;
use crate::radar::BssidRadar;
use crate::screensaver::ScreenSaverWatcher;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use turnike_client::PortalClient;
use turnike_secret::SecretStore;
use turnike_types::{DaemonConfig, PortalError, SessionState};

use chrono::Timelike;

fn get_local_hour() -> u8 {
    chrono::Local::now().hour() as u8
}

pub struct NetworkWatcher {
    client: PortalClient,
    config: DaemonConfig,
    preemption: PreemptionTracker,
    quota_tracker: QuotaNotificationTracker,
}

impl NetworkWatcher {
    pub fn new(client: PortalClient, config: DaemonConfig) -> Self {
        Self {
            client,
            config,
            preemption: PreemptionTracker::default(),
            quota_tracker: QuotaNotificationTracker::new(),
        }
    }

    pub fn update_config(&mut self, config: DaemonConfig) {
        self.config = config;
    }

    pub fn start(mut self) -> watch::Receiver<SessionState> {
        let (tx, rx) = watch::channel(SessionState::Offline);

        tokio::spawn(async move {
            let nm_client = match NetworkManagerClient::new().await {
                Ok(nm) => Some(nm),
                Err(e) => {
                    warn!(
                        "NetworkManager D-Bus servisine erişilemedi, doğrudan portal yoklamasına geçiliyor: {e}"
                    );
                    None
                }
            };

            let mut radar = nm_client.as_ref().map(|nm| {
                BssidRadar::new(
                    nm.clone(),
                    self.config.radar.clone(),
                    self.config.target_ssid.clone(),
                )
            });

            let (_screensaver, mut lock_rx) = ScreenSaverWatcher::start().await;
            let mut logged_out_by_screen_lock = false;
            let mut in_offpeak = false;

            loop {
                let is_locked = *lock_rx.borrow_and_update();
                if is_locked {
                    if self.config.auto_logout_on_screen_lock && !logged_out_by_screen_lock {
                        info!(
                            "Ekran kilitlendi, kalkan devreye girerek portal oturumu kapatılıyor"
                        );
                        let _ = self.client.logout().await;
                        logged_out_by_screen_lock = true;
                        HookExecutor::trigger(&self.config.hooks, HookContext::new("logout"));
                        let _ = tx.send(SessionState::CaptiveLocked);
                    }

                    sleep(Duration::from_secs(5)).await;
                    continue;
                } else if logged_out_by_screen_lock {
                    info!("Ekran kilidi açıldı, oturum izleme yeniden başlatılıyor");
                    logged_out_by_screen_lock = false;
                }

                let mut target_network_active = true;

                if let Some(ref nm) = nm_client {
                    match nm
                        .find_active_connection_by_ssid(&self.config.target_ssid)
                        .await
                    {
                        Ok(Some(active_path)) => {
                            target_network_active = true;

                            if self.config.auto_metered {
                                let _ = nm.set_connection_metered(&active_path, true).await;
                            }
                        }
                        Ok(None) => {
                            target_network_active = false;
                        }
                        Err(e) => {
                            debug!("D-Bus üzerinden aktif bağlantı sorgulanamadı: {e}");
                        }
                    }
                }

                if !target_network_active {
                    debug!(
                        "Hedef kablosuz ağ '{}' bağlı değil, beklemede",
                        self.config.target_ssid
                    );
                    let _ = tx.send(SessionState::Offline);
                    sleep(Duration::from_secs(self.config.check_interval_secs.max(10))).await;
                    continue;
                }

                if let Some(ref mut r) = radar {
                    r.update_config(self.config.radar.clone());
                    match r.check_and_optimize().await {
                        Ok(Some(roam)) => {
                            info!(
                                "Radar daha temiz erişim noktasına geçiş yaptı: {} -> {} (5GHz={}, Skor={})",
                                roam.from_bssid, roam.to_bssid, roam.is_5ghz, roam.to_score
                            );
                        }
                        Ok(None) => {}
                        Err(e) => {
                            debug!("Radar optimizasyon denetimi atlandı: {e}");
                        }
                    }
                }

                let mut current_state = self
                    .client
                    .get_status()
                    .await
                    .unwrap_or(SessionState::Offline);

                match &current_state {
                    SessionState::Authenticated(info) => {
                        self.quota_tracker
                            .check_and_notify(info, &self.config)
                            .await;

                        if self.config.offpeak.enabled {
                            let current_hour = get_local_hour();
                            let in_hours =
                                if self.config.offpeak.start_hour <= self.config.offpeak.end_hour {
                                    current_hour >= self.config.offpeak.start_hour
                                        && current_hour < self.config.offpeak.end_hour
                                } else {
                                    current_hour >= self.config.offpeak.start_hour
                                        || current_hour < self.config.offpeak.end_hour
                                };

                            let rtt_res = measure_tcp_rtt_with_fallback(
                                &self.config.offpeak.target_host,
                                DEFAULT_TARGET_SECONDARY,
                                Duration::from_millis(1000),
                            )
                            .await;

                            let (rtt_ok, rtt_ms) = match rtt_res {
                                Ok(d) => {
                                    let ms = d.as_millis() as u32;
                                    (ms <= self.config.offpeak.max_rtt_ms, Some(ms as u64))
                                }
                                Err(_) => (false, None),
                            };

                            if in_hours && rtt_ok {
                                if !in_offpeak {
                                    in_offpeak = true;
                                    info!(
                                        "Yoğun olmayan saat penceresine girildi (RTT: {:?} ms)",
                                        rtt_ms
                                    );
                                    HookExecutor::trigger(
                                        &self.config.hooks,
                                        HookContext {
                                            event: "offpeak_enter",
                                            rtt_ms,
                                            remaining_mb: Some(info.remaining_mb()),
                                            total_mb: Some(info.total_mb()),
                                            ssid: Some(self.config.target_ssid.clone()),
                                            ..Default::default()
                                        },
                                    );
                                }
                            } else if in_offpeak {
                                in_offpeak = false;
                                info!("Yoğun olmayan saat penceresinden çıkıldı");
                                HookExecutor::trigger(
                                    &self.config.hooks,
                                    HookContext {
                                        event: "offpeak_exit",
                                        rtt_ms,
                                        remaining_mb: Some(info.remaining_mb()),
                                        total_mb: Some(info.total_mb()),
                                        ssid: Some(self.config.target_ssid.clone()),
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                    }
                    SessionState::Offline | SessionState::CaptiveLocked => {
                        self.quota_tracker.reset();
                        if in_offpeak {
                            in_offpeak = false;
                            HookExecutor::trigger(
                                &self.config.hooks,
                                HookContext::new("offpeak_exit"),
                            );
                        }
                    }
                }

                if current_state == SessionState::CaptiveLocked && self.config.auto_login {
                    info!(
                        "Captive portal kilitli tespit edildi, otomatik giriş akışı başlatılıyor"
                    );

                    match SecretStore::load_async().await {
                        Ok(Some(creds)) => {
                            let is_tr = std::env::var("LC_ALL")
                                .or_else(|_| std::env::var("LC_MESSAGES"))
                                .or_else(|_| std::env::var("LANG"))
                                .unwrap_or_default()
                                .to_lowercase()
                                .starts_with("tr");

                            match self.client.login(&creds).await {
                                Ok(info) => {
                                    info!("Otomatik portal girişi tamamlandı");
                                    let (summary, body) = if is_tr {
                                        ("Oturum Açıldı", "GSB WiFi portalında oturum açıldı")
                                    } else {
                                        ("Signed In", "Signed in to GSB WiFi portal")
                                    };
                                    send_desktop_notification(summary, body).await;
                                    self.quota_tracker
                                        .check_and_notify(&info, &self.config)
                                        .await;

                                    HookExecutor::trigger(
                                        &self.config.hooks,
                                        HookContext {
                                            event: "login",
                                            username: Some(creds.username.clone()),
                                            remaining_mb: Some(info.remaining_mb()),
                                            total_mb: Some(info.total_mb()),
                                            ssid: Some(self.config.target_ssid.clone()),
                                            ..Default::default()
                                        },
                                    );

                                    current_state = SessionState::Authenticated(info);
                                }
                                Err(PortalError::MaxSessionsReached(msg)) => {
                                    warn!("Portal maksimum oturum hatası verdi: {msg}");

                                    if self.config.auto_preempt {
                                        if self.preemption.try_acquire() {
                                            info!(
                                                "Askıda kalan oturumu devralma (preemption) deneniyor"
                                            );
                                            match self.client.preempt_session(&creds).await {
                                                Ok(info) => {
                                                    info!("Askıda kalan oturum devralındı");
                                                    let (summary, body) = if is_tr {
                                                        (
                                                            "Oturum Devralındı",
                                                            "Askıda kalan oturum devralındı",
                                                        )
                                                    } else {
                                                        (
                                                            "Session Preempted",
                                                            "Session preempted from previous device",
                                                        )
                                                    };
                                                    send_desktop_notification(summary, body).await;
                                                    self.quota_tracker
                                                        .check_and_notify(&info, &self.config)
                                                        .await;

                                                    HookExecutor::trigger(
                                                        &self.config.hooks,
                                                        HookContext {
                                                            event: "login",
                                                            username: Some(creds.username.clone()),
                                                            remaining_mb: Some(info.remaining_mb()),
                                                            total_mb: Some(info.total_mb()),
                                                            ssid: Some(
                                                                self.config.target_ssid.clone(),
                                                            ),
                                                            ..Default::default()
                                                        },
                                                    );

                                                    current_state =
                                                        SessionState::Authenticated(info);
                                                }
                                                Err(e) => {
                                                    error!(
                                                        "Oturum devralma girişimi başarısız: {e}"
                                                    );
                                                }
                                            }
                                        } else {
                                            error!(
                                                "Oturum devralma sınırı aşıldı, ping-pong koruması devreye girdi"
                                            );
                                            let (summary, body) = if is_tr {
                                                (
                                                    "Oturum Çakışması",
                                                    "Başka bir cihazla sürekli oturum çakışması yaşandığı için devralma durduruldu",
                                                )
                                            } else {
                                                (
                                                    "Session Conflict",
                                                    "Session preemption stopped due to repeated device conflicts",
                                                )
                                            };
                                            send_desktop_notification(summary, body).await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Otomatik giriş başarısız oldu: {e}");
                                }
                            }
                        }
                        Ok(None) => {
                            debug!(
                                "Anahtarlıkta kayıtlı kimlik bilgisi bulunamadı, otomatik giriş atlandı"
                            );
                        }
                        Err(e) => {
                            error!("Sistem anahtarlığından kimlik bilgisi okunamadı: {e}");
                        }
                    }
                }

                let _ = tx.send(current_state);
                sleep(Duration::from_secs(self.config.check_interval_secs.max(5))).await;
            }
        });

        rx
    }
}
