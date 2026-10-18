use clap::{Parser, Subcommand};
use turnike_client::PortalClient;
use turnike_secret::SecretStore;
use turnike_types::{
    Credentials, QuotaInfo, QuotaPackageJson, QuotaProjectionJson, SessionState, StatusJsonOutput,
};

#[derive(Parser)]
#[command(
    name = "turnike-cli",
    about = "Turnike: GSB yurt ağları için komut satırı yöneticisi"
)]
struct Cli {
    #[arg(long, global = true)]
    lang: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Status {
        #[arg(short, long, default_value = "text")]
        format: String,

        #[arg(long)]
        json: bool,
    },

    Quota,

    Login {
        #[arg(short, long)]
        username: Option<String>,

        #[arg(short, long)]
        password: Option<String>,
    },

    Logout,

    Save {
        #[arg(short, long)]
        username: String,

        #[arg(short, long)]
        password: String,
    },

    Ping {
        #[arg(short, long, default_value = "1.1.1.1:443")]
        target: String,
    },

    Wifi {
        #[command(subcommand)]
        subcommand: WifiCommands,
    },

    #[command(alias = "hook")]
    Hooks {
        #[command(subcommand)]
        subcommand: HookCommands,
    },

    Clear,
}

#[derive(Subcommand)]
enum WifiCommands {
    Radar {
        #[arg(short, long, default_value = "GSBWIFI")]
        ssid: String,

        #[arg(short, long)]
        all: bool,

        #[arg(long)]
        json: bool,
    },

    Switch {
        #[arg(short, long)]
        bssid: String,

        #[arg(short, long, default_value = "GSBWIFI")]
        ssid: String,
    },

    Optimize {
        #[arg(short, long, default_value = "GSBWIFI")]
        ssid: String,
    },
}

#[derive(Subcommand)]
enum HookCommands {
    Test {
        #[arg(value_name = "EVENT")]
        event: String,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CliLang {
    Tr,
    En,
}

impl CliLang {
    pub fn from_opt(opt: Option<&str>) -> Self {
        match opt {
            Some(s) if s.eq_ignore_ascii_case("en") => Self::En,
            Some(s) if s.eq_ignore_ascii_case("tr") => Self::Tr,
            _ => Self::autodetect(),
        }
    }

    pub fn autodetect() -> Self {
        let locale = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_MESSAGES"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default();
        if locale.starts_with("tr") || locale.contains("tr_TR") || locale.contains("tr-TR") {
            Self::Tr
        } else {
            Self::En
        }
    }

    pub fn is_tr(self) -> bool {
        matches!(self, Self::Tr)
    }

    fn texts(self) -> &'static CliTexts {
        match self {
            Self::Tr => &TR_TEXTS,
            Self::En => &EN_TEXTS,
        }
    }
}

struct CliTexts {
    checking_status: &'static str,
    internet_active: &'static str,
    internet_offline: &'static str,
    portal_offline: &'static str,
    portal_locked: &'static str,
    portal_authenticated: &'static str,
    not_logged_in_hint: &'static str,
    portal_unreachable: &'static str,
    no_credentials_saved: &'static str,
    signing_in: &'static str,
    sign_in_success: &'static str,
    sign_in_failed: &'static str,
    signing_out: &'static str,
    sign_out_success: &'static str,
    sign_out_failed: &'static str,
    credentials_saved: &'static str,
    credentials_cleared: &'static str,
    user_label: &'static str,
    login_time_label: &'static str,
    session_duration_label: &'static str,
    package_details_header: &'static str,
    projection_header: &'static str,
    remaining_time_label: &'static str,
    daily_allowance_label: &'static str,
    daily_burn_label: &'static str,
    col_active: &'static str,
    status_offline: &'static str,
    status_locked: &'static str,
    tooltip_offline: &'static str,
    tooltip_locked: &'static str,
    tooltip_status: &'static str,
    tooltip_total_remaining: &'static str,
    pinging: &'static str,
    ping_result: &'static str,
    ping_failed: &'static str,
    wifi_scanning: &'static str,
    wifi_no_aps: &'static str,
    wifi_switched: &'static str,
    wifi_switch_failed: &'static str,
    wifi_already_best: &'static str,
    wifi_optimized: &'static str,
    col_band: &'static str,
    col_channel: &'static str,
    col_signal: &'static str,
    col_score: &'static str,
    col_ssid: &'static str,
    hook_testing: &'static str,
    hook_not_configured: &'static str,
    hook_success: &'static str,
    hook_failed: &'static str,
}

const TR_TEXTS: CliTexts = CliTexts {
    checking_status: "Bağlantı durumu kontrol ediliyor",
    internet_active: "İnternet bağlantısı: Açık",
    internet_offline: "İnternet bağlantısı: Kapalı veya portal tarafından kısıtlı",
    portal_offline: "Portal durumu: Çevrimdışı (erişilemiyor)",
    portal_locked: "Portal durumu: Oturum kapalı (oturum açılması bekleniyor)",
    portal_authenticated: "Portal durumu: Oturum açık",
    not_logged_in_hint: "Oturum açık değil, önce oturum açın",
    portal_unreachable: "Portal sunucusuna erişilemedi",
    no_credentials_saved: "Kimlik bilgisi bulunamadı. Kullanıcı adı ve şifre girin veya 'turnike-cli save' ile kaydedin",
    signing_in: "Oturum açılıyor",
    sign_in_success: "Oturum açıldı",
    sign_in_failed: "Oturum açılamadı",
    signing_out: "Oturum kapatılıyor",
    sign_out_success: "Oturum kapatıldı",
    sign_out_failed: "Oturum kapatılamadı",
    credentials_saved: "Kimlik bilgileri sistem anahtarlığına kaydedildi",
    credentials_cleared: "Kayıtlı kimlik bilgileri silindi",
    user_label: "Kullanıcı",
    login_time_label: "Oturum açma zamanı",
    session_duration_label: "Oturum süresi",
    package_details_header: "\nPaket Detayları",
    projection_header: "\nKullanım Tahmini",
    remaining_time_label: "Kalan süre",
    daily_allowance_label: "Günlük bütçe",
    daily_burn_label: "Mevcut harcama",
    col_active: "Aktif",
    status_offline: "Çevrimdışı",
    status_locked: "Kilitli",
    tooltip_offline: "Turnike GSB WiFi: Ağ bağlantısı yok veya portal sunucusuna erişilemiyor",
    tooltip_locked: "Turnike GSB WiFi: Captive portal oturumu açılması bekleniyor",
    tooltip_status: "Durum: Oturum açık",
    tooltip_total_remaining: "Toplam kalan",
    pinging: "Ağ gecikmesi ölçülüyor (TCP RTT)",
    ping_result: "Gecikme süresi",
    ping_failed: "Gecikme ölçümü başarısız oldu",
    wifi_scanning: "Erişim noktaları taranıyor",
    wifi_no_aps: "Görüş alanında uygun erişim noktası bulunamadı",
    wifi_switched: "Erişim noktasına geçildi",
    wifi_switch_failed: "Erişim noktası geçişi başarısız oldu",
    wifi_already_best: "Zaten en yüksek skorlu erişim noktasına bağlı",
    wifi_optimized: "Daha temiz erişim noktasına geçildi",
    col_band: "Bant",
    col_channel: "Kanal",
    col_signal: "Sinyal",
    col_score: "Skor",
    col_ssid: "SSID",
    hook_testing: "Kanca komutu çalıştırılıyor",
    hook_not_configured: "Bu olay için kanca komutu tanımlanmamış",
    hook_success: "Kanca başarıyla tamamlandı",
    hook_failed: "Kanca hata verdi",
};

const EN_TEXTS: CliTexts = CliTexts {
    checking_status: "Checking connection status",
    internet_active: "Internet connection: Active",
    internet_offline: "Internet connection: Offline or captive locked",
    portal_offline: "Portal status: Offline (unreachable)",
    portal_locked: "Portal status: Not signed in",
    portal_authenticated: "Portal status: Signed in",
    not_logged_in_hint: "Not signed in, sign in first",
    portal_unreachable: "Cannot reach portal server",
    no_credentials_saved: "No credentials found. Provide username and password or save them with 'turnike-cli save'",
    signing_in: "Signing in",
    sign_in_success: "Signed in successfully",
    sign_in_failed: "Sign-in failed",
    signing_out: "Signing out",
    sign_out_success: "Signed out successfully",
    sign_out_failed: "Sign-out failed",
    credentials_saved: "Credentials saved to system keyring",
    credentials_cleared: "Saved credentials removed",
    user_label: "User",
    login_time_label: "Sign-in time",
    session_duration_label: "Session duration",
    package_details_header: "\nPackage Details",
    projection_header: "\nUsage Projection",
    remaining_time_label: "Remaining time",
    daily_allowance_label: "Daily budget",
    daily_burn_label: "Current burn rate",
    col_active: "Active",
    status_offline: "Offline",
    status_locked: "Locked",
    tooltip_offline: "Turnike GSB WiFi: No network connection or portal server unreachable",
    tooltip_locked: "Turnike GSB WiFi: Captive portal sign in required",
    tooltip_status: "Status: Signed in",
    tooltip_total_remaining: "Total remaining",
    pinging: "Measuring network latency (TCP RTT)",
    ping_result: "Latency",
    ping_failed: "Latency measurement failed",
    wifi_scanning: "Scanning access points",
    wifi_no_aps: "No suitable access points found",
    wifi_switched: "Switched to access point",
    wifi_switch_failed: "Access point switch failed",
    wifi_already_best: "Already connected to highest scoring access point",
    wifi_optimized: "Switched to cleaner access point",
    col_band: "Band",
    col_channel: "Channel",
    col_signal: "Signal",
    col_score: "Score",
    col_ssid: "SSID",
    hook_testing: "Executing hook command",
    hook_not_configured: "No hook command configured for this event",
    hook_success: "Hook completed successfully",
    hook_failed: "Hook exited with error",
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let lang = CliLang::from_opt(cli.lang.as_deref());
    let t = lang.texts();

    let is_json_requested = match &cli.command {
        Commands::Status { format, json } => *json || format.eq_ignore_ascii_case("json"),
        Commands::Wifi {
            subcommand: WifiCommands::Radar { json, .. },
        } => *json,
        _ => false,
    };

    if !is_json_requested {
        tracing_subscriber::fmt::init();
    }

    let client = PortalClient::new()?;

    match cli.command {
        Commands::Status { format, json } => {
            let use_json = json || format.eq_ignore_ascii_case("json");

            if use_json {
                let status = client.get_status().await.unwrap_or(SessionState::Offline);
                let is_online = if matches!(status, SessionState::Authenticated(_)) {
                    client.check_internet().await.unwrap_or(false)
                } else {
                    false
                };

                let output = build_status_json(&status, is_online, lang);
                println!("{}", serde_json::to_string(&output)?);
                return Ok(());
            }

            println!("{}", t.checking_status);
            let is_online = client.check_internet().await?;

            if is_online {
                println!("{}", t.internet_active);
            } else {
                println!("{}", t.internet_offline);
            }

            match client.get_status().await? {
                SessionState::Offline => println!("{}", t.portal_offline),
                SessionState::CaptiveLocked => println!("{}", t.portal_locked),
                SessionState::Authenticated(info) => {
                    println!("{}", t.portal_authenticated);
                    print_quota(&info, lang);
                }
            }
        }

        Commands::Quota => match client.get_status().await? {
            SessionState::Authenticated(info) => print_quota(&info, lang),
            SessionState::CaptiveLocked => {
                println!("{}", t.not_logged_in_hint);
            }
            SessionState::Offline => {
                println!("{}", t.portal_unreachable);
            }
        },

        Commands::Login { username, password } => {
            let creds = match (username, password) {
                (Some(u), Some(p)) => Credentials::new(u, p),
                _ => match SecretStore::load_async().await? {
                    Some(c) => c,
                    None => {
                        eprintln!("{}", t.no_credentials_saved);
                        std::process::exit(1);
                    }
                },
            };

            println!("{}: {}", t.signing_in, creds.masked_username());
            match client.login(&creds).await {
                Ok(info) => {
                    println!("{}", t.sign_in_success);
                    print_quota(&info, lang);
                }
                Err(e) => {
                    eprintln!(
                        "{}: {}",
                        t.sign_in_failed,
                        e.localized_message(lang.is_tr())
                    );
                    std::process::exit(1);
                }
            }
        }

        Commands::Logout => {
            println!("{}", t.signing_out);
            match client.logout().await {
                Ok(_) => println!("{}", t.sign_out_success),
                Err(e) => eprintln!(
                    "{}: {}",
                    t.sign_out_failed,
                    e.localized_message(lang.is_tr())
                ),
            }
        }

        Commands::Save { username, password } => {
            let creds = Credentials::new(username, password);
            SecretStore::save_async(creds).await?;
            println!("{}", t.credentials_saved);
        }

        Commands::Clear => {
            SecretStore::clear_async().await?;
            println!("{}", t.credentials_cleared);
        }

        Commands::Ping { target } => {
            println!("{}: {target}", t.pinging);
            match turnike_daemon::measure_tcp_rtt(&target, std::time::Duration::from_millis(3000))
                .await
            {
                Ok(rtt) => {
                    println!("{}: {} ms", t.ping_result, rtt.as_millis());
                }
                Err(e) => {
                    eprintln!("{}: {e}", t.ping_failed);
                    std::process::exit(1);
                }
            }
        }

        Commands::Wifi { subcommand } => {
            let nm = match turnike_daemon::NetworkManagerClient::new().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("NetworkManager D-Bus bağlantısı kurulamadı: {e}");
                    std::process::exit(1);
                }
            };

            match subcommand {
                WifiCommands::Radar { ssid, all, json } => {
                    let filter_ssid = if all { String::new() } else { ssid };
                    let radar = turnike_daemon::BssidRadar::new(
                        nm,
                        turnike_types::RadarConfig::default(),
                        filter_ssid,
                    );
                    if !json {
                        println!("{}", t.wifi_scanning);
                    }
                    match radar.scan_all_aps().await {
                        Ok(aps) => {
                            if aps.is_empty() {
                                if json {
                                    println!("[]");
                                } else {
                                    println!("{}", t.wifi_no_aps);
                                }
                                return Ok(());
                            }

                            if json {
                                let json_list: Vec<&turnike_types::AccessPointInfo> =
                                    aps.iter().map(|(info, _, _)| info).collect();
                                println!("{}", serde_json::to_string(&json_list)?);
                            } else {
                                println!(
                                    "{:<4} {:<18} {:<8} {:<6} {:<7} {:<6} {:<16}",
                                    t.col_active,
                                    "BSSID",
                                    t.col_band,
                                    t.col_channel,
                                    t.col_signal,
                                    t.col_score,
                                    t.col_ssid
                                );
                                println!("{:-<70}", "");
                                for (info, _, _) in aps {
                                    let mark = if info.is_active { "*" } else { " " };
                                    let band = if info.is_5ghz { "5 GHz" } else { "2.4 GHz" };
                                    let signal_str = if lang.is_tr() {
                                        format!("%{}", info.strength)
                                    } else {
                                        format!("{}%", info.strength)
                                    };
                                    println!(
                                        "{:<4} {:<18} {:<8} {:<6} {:<7} {:<6} {:<16}",
                                        mark,
                                        info.bssid,
                                        band,
                                        info.channel,
                                        signal_str,
                                        info.score,
                                        info.ssid
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("{}: {e}", t.wifi_switch_failed);
                            std::process::exit(1);
                        }
                    }
                }
                WifiCommands::Switch { bssid, ssid } => {
                    let mut radar = turnike_daemon::BssidRadar::new(
                        nm,
                        turnike_types::RadarConfig::default(),
                        ssid,
                    );
                    match radar.switch_to_bssid(&bssid).await {
                        Ok(info) => {
                            let band = if info.is_5ghz { "5 GHz" } else { "2.4 GHz" };
                            let strength_str = if lang.is_tr() {
                                format!("%{}", info.strength)
                            } else {
                                format!("{}%", info.strength)
                            };
                            println!(
                                "{}: {} ({band} / {strength_str})",
                                t.wifi_switched, info.bssid
                            );
                        }
                        Err(e) => {
                            eprintln!("{}: {e}", t.wifi_switch_failed);
                            std::process::exit(1);
                        }
                    }
                }
                WifiCommands::Optimize { ssid } => {
                    let mut radar = turnike_daemon::BssidRadar::new(
                        nm,
                        turnike_types::RadarConfig::default(),
                        ssid,
                    );
                    match radar.check_and_optimize().await {
                        Ok(Some(decision)) => {
                            let band = if decision.is_5ghz { "5 GHz" } else { "2.4 GHz" };
                            println!(
                                "{}: {} ({}, Skor: {}) -> {} (Skor: {})",
                                t.wifi_optimized,
                                decision.from_bssid,
                                band,
                                decision.from_score,
                                decision.to_bssid,
                                decision.to_score
                            );
                        }
                        Ok(None) => {
                            println!("{}", t.wifi_already_best);
                        }
                        Err(e) => {
                            eprintln!("{}: {e}", t.wifi_switch_failed);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        Commands::Hooks { subcommand } => match subcommand {
            HookCommands::Test { event } => {
                let cfg = turnike_daemon::load_config();
                let (event_str, cmd) = match event.to_lowercase().as_str() {
                    "login" | "on_login" => ("login", cfg.hooks.on_login.as_deref()),
                    "logout" | "on_logout" => ("logout", cfg.hooks.on_logout.as_deref()),
                    "quota_critical" | "on_quota_critical" => {
                        ("quota_critical", cfg.hooks.on_quota_critical.as_deref())
                    }
                    "throttled" | "on_throttled" => {
                        ("throttled", cfg.hooks.on_throttled.as_deref())
                    }
                    "offpeak_enter" | "on_offpeak_enter" | "offpeak_start" => {
                        ("offpeak_enter", cfg.hooks.on_offpeak_enter.as_deref())
                    }
                    "offpeak_exit" | "on_offpeak_exit" | "offpeak_end" => {
                        ("offpeak_exit", cfg.hooks.on_offpeak_exit.as_deref())
                    }
                    other => {
                        eprintln!(
                            "Bilinmeyen olay: {other}. Geçerli olaylar: login, logout, quota_critical, throttled, offpeak_enter, offpeak_exit"
                        );
                        std::process::exit(1);
                    }
                };

                match cmd {
                    Some(c) => {
                        println!("{}: [{event_str}] -> {c}", t.hook_testing);
                        let ctx = turnike_daemon::HookContext {
                            event: event_str,
                            username: Some("test_user".to_string()),
                            remaining_mb: Some(10240.0),
                            total_mb: Some(32768.0),
                            rtt_ms: Some(42),
                            ssid: Some("GSBWIFI".to_string()),
                            bssid: Some("AA:BB:CC:DD:EE:FF".to_string()),
                        };
                        let success = turnike_daemon::HookExecutor::execute_command(c, &ctx).await;
                        if success {
                            println!("{}", t.hook_success);
                        } else {
                            eprintln!("{}", t.hook_failed);
                            std::process::exit(1);
                        }
                    }
                    None => {
                        println!("{}: {event_str}", t.hook_not_configured);
                    }
                }
            }
        },
    }

    Ok(())
}

fn print_quota(info: &QuotaInfo, lang: CliLang) {
    let t = lang.texts();
    if let Some(name) = &info.full_name {
        println!("{}: {name}", t.user_label);
    }
    if let Some(login) = &info.login_time {
        println!("{}: {login}", t.login_time_label);
    }
    if let Some(session) = &info.session_time {
        println!("{}: {session}", t.session_duration_label);
    }

    println!("{}", t.package_details_header);
    for pkg in &info.packages {
        if lang.is_tr() {
            println!(
                "[{}] Kalan: {:.2} MB / Toplam: {:.2} MB (Kalan Süre: {})",
                pkg.package_type, pkg.remaining_mb, pkg.total_mb, pkg.remaining_time
            );
        } else {
            println!(
                "[{}] Remaining: {:.2} MB / Total: {:.2} MB (Remaining Time: {})",
                pkg.package_type, pkg.remaining_mb, pkg.total_mb, pkg.remaining_time
            );
        }
    }

    if let Some(proj) = info.calculate_projection(None) {
        println!("{}", t.projection_header);
        let days_unit = if lang.is_tr() { "gün" } else { "days" };
        let rate_unit = if lang.is_tr() { "MB/gün" } else { "MB/day" };
        println!(
            "{}: {:.1} {days_unit}",
            t.remaining_time_label, proj.days_remaining
        );
        println!(
            "{}: {:.0} {rate_unit}",
            t.daily_allowance_label, proj.daily_allowance_mb
        );
        println!(
            "{}: {:.0} {rate_unit}",
            t.daily_burn_label, proj.current_burn_rate_mb
        );
        if let Some(day) = proj.projected_depletion_day {
            if proj.is_at_risk {
                if lang.is_tr() {
                    println!("Bu hızla ayın {day}. gününde kota tükenir");
                } else {
                    println!("At this rate, quota will be depleted on day {day}");
                }
            } else if lang.is_tr() {
                println!("Mevcut kullanım hızı ay sonu için yeterli");
            } else {
                println!("Current usage rate is sufficient for month end");
            }
        }
    }
}

fn build_status_json(state: &SessionState, online: bool, lang: CliLang) -> StatusJsonOutput {
    let t = lang.texts();
    match state {
        SessionState::Offline => StatusJsonOutput {
            text: t.status_offline.to_string(),
            alt: "offline".to_string(),
            tooltip: t.tooltip_offline.to_string(),
            class: "offline".to_string(),
            percentage: 0.0,
            online: false,
            state: "offline".to_string(),
            full_name: None,
            login_time: None,
            session_time: None,
            total_mb: 0.0,
            remaining_mb: 0.0,
            used_mb: 0.0,
            remaining_gb: 0.0,
            total_gb: 0.0,
            packages: Vec::new(),
            projection: None,
        },
        SessionState::CaptiveLocked => StatusJsonOutput {
            text: t.status_locked.to_string(),
            alt: "locked".to_string(),
            tooltip: t.tooltip_locked.to_string(),
            class: "locked".to_string(),
            percentage: 0.0,
            online: false,
            state: "locked".to_string(),
            full_name: None,
            login_time: None,
            session_time: None,
            total_mb: 0.0,
            remaining_mb: 0.0,
            used_mb: 0.0,
            remaining_gb: 0.0,
            total_gb: 0.0,
            packages: Vec::new(),
            projection: None,
        },
        SessionState::Authenticated(info) => {
            let total_mb = info.total_mb();
            let remaining_mb = info.remaining_mb();
            let used_mb = info.used_mb();
            let remaining_gb = info.remaining_gb();
            let total_gb = info.total_gb();
            let remaining_ratio = info.remaining_ratio();
            let percentage = (remaining_ratio * 100.0).round();

            let class = if percentage <= 10.0 || remaining_gb <= 1.0 {
                "critical".to_string()
            } else if percentage <= 25.0 || remaining_gb <= 3.0 {
                "warning".to_string()
            } else {
                "connected".to_string()
            };

            let text = format!("{remaining_gb:.1} GB");

            let mut tooltip_lines =
                vec!["Turnike GSB WiFi".to_string(), t.tooltip_status.to_string()];
            if let Some(ref name) = info.full_name {
                tooltip_lines.push(format!("{}: {name}", t.user_label));
            }
            if lang.is_tr() {
                tooltip_lines.push(format!(
                    "{}: {remaining_gb:.2} GB / {total_gb:.2} GB (%{percentage:.0})",
                    t.tooltip_total_remaining
                ));
            } else {
                tooltip_lines.push(format!(
                    "{}: {remaining_gb:.2} GB / {total_gb:.2} GB ({percentage:.0}%)",
                    t.tooltip_total_remaining
                ));
            }
            if let Some(ref session) = info.session_time {
                tooltip_lines.push(format!("{}: {session}", t.session_duration_label));
            }
            if let Some(ref login) = info.login_time {
                tooltip_lines.push(format!("{}: {login}", t.login_time_label));
            }
            for pkg in &info.packages {
                tooltip_lines.push(format!(
                    "• {}: {:.2} GB / {:.2} GB ({})",
                    pkg.package_type,
                    pkg.remaining_mb / 1024.0,
                    pkg.total_mb / 1024.0,
                    pkg.remaining_time
                ));
            }

            let packages = info
                .packages
                .iter()
                .map(|p| QuotaPackageJson {
                    package_type: p.package_type.clone(),
                    total_mb: p.total_mb,
                    remaining_mb: p.remaining_mb,
                    used_mb: p.used_mb(),
                    remaining_ratio: p.remaining_ratio(),
                    remaining_time: p.remaining_time.clone(),
                    expiration_date: p.expiration_date.clone(),
                })
                .collect();

            let projection = info
                .calculate_projection(None)
                .as_ref()
                .map(QuotaProjectionJson::from);

            StatusJsonOutput {
                text,
                alt: "connected".to_string(),
                tooltip: tooltip_lines.join("\n"),
                class,
                percentage,
                online,
                state: "authenticated".to_string(),
                full_name: info.full_name.clone(),
                login_time: info.login_time.clone(),
                session_time: info.session_time.clone(),
                total_mb,
                remaining_mb,
                used_mb,
                remaining_gb,
                total_gb,
                packages,
                projection,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turnike_types::QuotaPackage;

    #[test]
    fn test_build_status_json_offline() {
        let output_tr = build_status_json(&SessionState::Offline, false, CliLang::Tr);
        assert_eq!(output_tr.class, "offline");
        assert_eq!(output_tr.text, "Çevrimdışı");
        assert!(!output_tr.online);

        let output_en = build_status_json(&SessionState::Offline, false, CliLang::En);
        assert_eq!(output_en.text, "Offline");
        assert_eq!(
            output_en.tooltip,
            "Turnike GSB WiFi: No network connection or portal server unreachable"
        );
    }

    #[test]
    fn test_build_status_json_locked() {
        let output_tr = build_status_json(&SessionState::CaptiveLocked, false, CliLang::Tr);
        assert_eq!(output_tr.class, "locked");
        assert_eq!(output_tr.text, "Kilitli");
        assert_eq!(output_tr.alt, "locked");
        assert!(!output_tr.online);

        let output_en = build_status_json(&SessionState::CaptiveLocked, false, CliLang::En);
        assert_eq!(output_en.text, "Locked");
        assert_eq!(
            output_en.tooltip,
            "Turnike GSB WiFi: Captive portal sign in required"
        );
    }

    #[test]
    fn test_build_status_json_authenticated() {
        let info = QuotaInfo {
            packages: vec![QuotaPackage {
                package_type: "Genel İnternet".to_string(),
                total_mb: 32768.0,
                remaining_mb: 16384.0,
                start_date: "2026-10-01".to_string(),
                expiration_date: "2026-10-31".to_string(),
                remaining_time: "10 gün".to_string(),
            }],
            session_time: Some("02:30:00".to_string()),
            login_time: Some("2026-10-15 09:00".to_string()),
            full_name: Some("Ali Veli".to_string()),
        };

        let output = build_status_json(
            &SessionState::Authenticated(info.clone()),
            true,
            CliLang::Tr,
        );
        assert_eq!(output.class, "connected");
        assert_eq!(output.text, "16.0 GB");
        assert_eq!(output.percentage, 50.0);
        assert!(output.online);
        assert!(output.tooltip.contains("Ali Veli"));
        assert!(output.tooltip.contains("02:30:00"));

        let output_en = build_status_json(&SessionState::Authenticated(info), true, CliLang::En);
        assert!(output_en.tooltip.contains("Status: Signed in"));
        assert!(output_en.tooltip.contains("Total remaining"));
    }

    #[test]
    fn test_build_status_json_warning_and_critical_classes() {
        let warn_info = QuotaInfo {
            packages: vec![QuotaPackage {
                package_type: "Genel".to_string(),
                total_mb: 10000.0,
                remaining_mb: 2000.0,
                start_date: "2026-10-01".to_string(),
                expiration_date: "2026-10-31".to_string(),
                remaining_time: "5 gün".to_string(),
            }],
            session_time: None,
            login_time: None,
            full_name: None,
        };
        let warn_out =
            build_status_json(&SessionState::Authenticated(warn_info), true, CliLang::Tr);
        assert_eq!(warn_out.class, "warning");

        let crit_info = QuotaInfo {
            packages: vec![QuotaPackage {
                package_type: "Genel".to_string(),
                total_mb: 10000.0,
                remaining_mb: 500.0,
                start_date: "2026-10-01".to_string(),
                expiration_date: "2026-10-31".to_string(),
                remaining_time: "1 gün".to_string(),
            }],
            session_time: None,
            login_time: None,
            full_name: None,
        };
        let crit_out =
            build_status_json(&SessionState::Authenticated(crit_info), true, CliLang::Tr);
        assert_eq!(crit_out.class, "critical");
    }

    #[test]
    fn test_cli_lang_from_opt() {
        assert_eq!(CliLang::from_opt(Some("en")), CliLang::En);
        assert_eq!(CliLang::from_opt(Some("EN")), CliLang::En);
        assert_eq!(CliLang::from_opt(Some("tr")), CliLang::Tr);
        assert_eq!(CliLang::from_opt(Some("TR")), CliLang::Tr);
    }
}
