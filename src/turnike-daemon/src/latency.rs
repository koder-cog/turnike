use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, warn};

#[derive(Debug, Error)]
pub enum LatencyError {
    #[error("Gecikme ölçümü zaman aşımına uğradı ({0:?})")]
    Timeout(Duration),

    #[error("Hedefe TCP bağlantısı kurulamadı ({target}): {source}")]
    ConnectionFailed {
        target: String,
        source: std::io::Error,
    },
}

pub const DEFAULT_TARGET_PRIMARY: &str = "1.1.1.1:443";
pub const DEFAULT_TARGET_SECONDARY: &str = "8.8.8.8:443";

// Linux'ta ham ICMP ping soketi (SOCK_RAW) açmak root yetkisi gerektirir.
// Kullanıcı yetkisiyle sıfır veri transferi yaparak gecikme ölçmek için port 443 TCP SYN-ACK
// süresi ölçülür. Port 443 seçimi KYK ve kurumsal ağlardaki DNS/DPI filtrelerini de baypas eder.
pub async fn measure_tcp_rtt(
    target: &str,
    timeout_dur: Duration,
) -> Result<Duration, LatencyError> {
    let start = Instant::now();

    match timeout(timeout_dur, TcpStream::connect(target)).await {
        Ok(Ok(stream)) => {
            let elapsed = start.elapsed();

            drop(stream);
            debug!("TCP RTT ölçümü ({target}): {elapsed:?}");
            Ok(elapsed)
        }
        Ok(Err(e)) => {
            warn!("TCP RTT bağlantı hatası ({target}): {e}");
            Err(LatencyError::ConnectionFailed {
                target: target.to_string(),
                source: e,
            })
        }
        Err(_) => {
            warn!("TCP RTT ölçümü zaman aşımına uğradı ({target}, {timeout_dur:?})");
            Err(LatencyError::Timeout(timeout_dur))
        }
    }
}

pub async fn measure_tcp_rtt_with_fallback(
    target: &str,
    fallback: &str,
    timeout_dur: Duration,
) -> Result<Duration, LatencyError> {
    match measure_tcp_rtt(target, timeout_dur).await {
        Ok(rtt) => Ok(rtt),
        Err(first_err) => {
            debug!("Birincil hedef ({target}) başarısız, ikincil deneniyor ({fallback})");
            measure_tcp_rtt(fallback, timeout_dur)
                .await
                .map_err(|_| first_err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_measure_tcp_rtt_local_listener() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Yerel soket açılmalı");
        let local_addr = listener.local_addr().expect("Adres alınmalı");

        let handle = tokio::spawn(async move {
            if let Ok((socket, _)) = listener.accept().await {
                drop(socket);
            }
        });

        let target = local_addr.to_string();
        let rtt = measure_tcp_rtt(&target, Duration::from_millis(500))
            .await
            .expect("RTT ölçümü başarılı olmalı");

        assert!(rtt.as_millis() < 200, "Yerel RTT çok düşük olmalı: {rtt:?}");
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_measure_tcp_rtt_timeout() {
        let non_routable = "192.0.2.1:80";
        let res = measure_tcp_rtt(non_routable, Duration::from_millis(50)).await;
        assert!(res.is_err());
    }
}
