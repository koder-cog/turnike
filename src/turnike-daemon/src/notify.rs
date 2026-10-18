use std::collections::HashMap;
use tracing::{debug, warn};
use zbus::connection::Connection;
use zbus::zvariant::Value;

pub async fn send_desktop_notification(summary: &str, body: &str) -> u32 {
    send_desktop_notification_full(summary, body, "network-wireless-symbolic", 0, 1).await
}

pub async fn send_desktop_notification_full(
    summary: &str,
    body: &str,
    icon: &str,
    replaces_id: u32,
    urgency: u8,
) -> u32 {
    let conn = match Connection::session().await {
        Ok(c) => c,
        Err(e) => {
            warn!("D-Bus oturum veri yoluna bağlanılamadı, bildirim atlandı: {e}");
            return 0;
        }
    };

    let app_name = "Turnike";
    let actions: Vec<&str> = Vec::new();
    let mut hints: HashMap<&str, Value<'_>> = HashMap::new();
    hints.insert("urgency", Value::from(urgency));

    let expire_timeout: i32 = if urgency >= 2 { 10000 } else { 5000 };

    let result = conn
        .call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &(
                app_name,
                replaces_id,
                icon,
                summary,
                body,
                actions,
                hints,
                expire_timeout,
            ),
        )
        .await;

    match result {
        Ok(reply) => {
            let notif_id: u32 = reply.body().deserialize().unwrap_or(0);
            debug!("Masaüstü bildirimi iletildi (ID: {notif_id}): {summary}");
            notif_id
        }
        Err(e) => {
            warn!("Masaüstü bildirimi gönderilemedi: {e}");
            0
        }
    }
}
