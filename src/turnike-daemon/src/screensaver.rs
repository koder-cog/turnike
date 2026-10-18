use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::watch;
use tracing::{debug, info, warn};
use zbus::connection::Connection;
use zbus::proxy;

#[proxy(
    default_service = "org.freedesktop.ScreenSaver",
    default_path = "/org/freedesktop/ScreenSaver",
    interface = "org.freedesktop.ScreenSaver"
)]
trait ScreenSaver {
    #[zbus(signal)]
    fn active_changed(&self, new_value: bool) -> zbus::Result<()>;

    fn get_active(&self) -> zbus::Result<bool>;
}

pub struct ScreenSaverWatcher {
    is_locked: Arc<AtomicBool>,
}

impl ScreenSaverWatcher {
    pub async fn start() -> (Self, watch::Receiver<bool>) {
        let (tx, rx) = watch::channel(false);
        let is_locked = Arc::new(AtomicBool::new(false));
        let is_locked_clone = Arc::clone(&is_locked);

        tokio::spawn(async move {
            let session_conn = match Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    warn!(
                        "Session D-Bus bağlantısı kurulamadı, ekran kilidi kalkanı devre dışı: {e}"
                    );
                    return;
                }
            };

            let proxy = match ScreenSaverProxy::new(&session_conn).await {
                Ok(p) => p,
                Err(e) => {
                    warn!("ScreenSaver D-Bus proxy oluşturulamadı: {e}");
                    return;
                }
            };

            if let Ok(active) = proxy.get_active().await {
                is_locked_clone.store(active, Ordering::SeqCst);
                let _ = tx.send(active);
            }

            match proxy.receive_active_changed().await {
                Ok(mut stream) => {
                    info!("Ekran koruyucu D-Bus dinleyicisi aktif");
                    use zbus::export::ordered_stream::OrderedStreamExt;
                    while let Some(signal) = stream.next().await {
                        match signal.args() {
                            Ok(args) => {
                                let new_state = args.new_value;
                                debug!("Ekran kilidi sinyali alındı: {new_state}");
                                is_locked_clone.store(new_state, Ordering::SeqCst);
                                let _ = tx.send(new_state);
                            }
                            Err(e) => {
                                warn!("Ekran kilidi sinyal argümanı ayrıştırılamadı: {e}");
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("ActiveChanged sinyal akışı başlatılamadı: {e}");
                }
            }
        });

        (Self { is_locked }, rx)
    }

    pub fn is_locked(&self) -> bool {
        self.is_locked.load(Ordering::SeqCst)
    }
}
