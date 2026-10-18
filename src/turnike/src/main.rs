mod account_pages;
mod actions_dialog;
mod config;
mod i18n;
mod password_dialog;
mod window;

use adw::prelude::*;
use adw::{Application, ColorScheme, StyleManager};
use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::{IconTheme, Window};
use libadwaita as adw;
use std::path::Path;
use window::{AppMode, MainWindow};

pub const APP_ID: &str = "io.github.koder_cog.Turnike";

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt::init();

    let mut mode = AppMode::Normal;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--demo" | "-d" => mode = AppMode::Demo,
            "--smoke-test" => mode = AppMode::SmokeTest,
            "--help" | "-h" => {
                println!("Turnike - GSB yurt ağları için oturum ve kota istemcisi\n");
                println!("Kullanım: turnike [SEÇENEKLER]\n");
                println!("Seçenekler:");
                println!(
                    "  -d, --demo        Temsili kota verileriyle demo kipi (gizlilik/ekran görüntüsü için)"
                );
                println!(
                    "      --smoke-test  Tüm arayüz geçişlerini otomatik test edip sonlanan duman testi"
                );
                println!("  -h, --help        Bu yardım iletisini görüntüler");
                return glib::ExitCode::SUCCESS;
            }
            _ => {}
        }
    }

    let rt = tokio::runtime::Runtime::new().expect("Tokio çalışma zamanı başlatılamadı");
    let _guard = rt.enter();

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| {
        StyleManager::default().set_color_scheme(ColorScheme::Default);

        let provider = gtk4::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));

        if let Some(display) = Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );

            let icon_theme = IconTheme::for_display(&display);
            let local_icons = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/icons");
            if local_icons.exists() {
                icon_theme.add_search_path(&local_icons);
            }
        }
        Window::set_default_icon_name(APP_ID);
    });

    app.connect_activate(move |app| {
        let win = MainWindow::new(app, mode);
        win.window.present();
    });

    app.run_with_args::<&str>(&[])
}
