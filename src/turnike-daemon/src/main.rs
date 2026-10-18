use std::error::Error;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use turnike_client::PortalClient;
use turnike_daemon::{NetworkWatcher, load_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Turnike arka plan servisi başlatılıyor");

    let config = load_config();
    info!(
        "Yapılandırma yüklendi: SSID={}, oto-giriş={}, devralma={}, tarifeli={}",
        config.target_ssid, config.auto_login, config.auto_preempt, config.auto_metered
    );

    let client = PortalClient::new()?;
    let watcher = NetworkWatcher::new(client, config);
    let mut rx = watcher.start();

    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let state = rx.borrow().clone();
            info!("Ağ ve portal durumu güncellendi: {:?}", state);
        }
    });

    info!("Turnike servisi çalışıyor, sonlandırmak için Ctrl+C veya SIGTERM bekleniyor");

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Kapatma sinyali alındı, Turnike servisi sonlandırılıyor");
        }
        Err(err) => {
            warn!("Sinyal dinleyicisi hatası: {err}");
        }
    }

    Ok(())
}
