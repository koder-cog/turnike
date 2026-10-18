use crate::account_pages::{
    push_forgot_password_page, push_help_page, push_register_page, show_preferences_dialog,
    show_shortcuts_dialog,
};
use crate::actions_dialog::push_actions_page;
use crate::i18n::Language;
use crate::password_dialog::push_password_page;
use adw::prelude::*;
use adw::{
    AboutDialog, ActionRow, AlertDialog, Application, ApplicationWindow, Banner, Clamp, EntryRow,
    HeaderBar, NavigationPage, NavigationView, PasswordEntryRow, PreferencesGroup,
    ResponseAppearance, SwitchRow, Toast, ToastOverlay, ToolbarView, ViewStack, WindowTitle,
};
use gtk4::gdk::Key;
use gtk4::gio::{self, SimpleAction};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, EventControllerKey, Image, InputPurpose, Label, MenuButton, Orientation,
    PolicyType, ProgressBar, ScrolledWindow, Spinner,
};
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use turnike_client::PortalClient;
use turnike_secret::SecretStore;
use turnike_types::{Credentials, PortalError, QuotaInfo, SessionState};

#[derive(Clone, Debug, PartialEq)]
pub enum NetworkEnvironment {
    GsbWifi {
        ssid: String,
        connection_timestamp: Option<u64>,
    },
    ForeignWifi {
        ssid: String,
    },
    Disconnected,
    Unknown,
}

pub enum ActionResponse {
    StatusUpdated(SessionState),
    NetworkEnvironmentUpdated(NetworkEnvironment),
    LoginSuccess(QuotaInfo),
    LoginFailed(PortalError),
    LogoutSuccess,
    LogoutFailed(PortalError),
    CredentialsLoaded(Option<Credentials>),
    WifiApUpdated(Option<turnike_types::AccessPointInfo>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Normal,

    Demo,

    SmokeTest,
}

pub struct MainWindow {
    pub window: ApplicationWindow,
}

async fn check_network_environment() -> NetworkEnvironment {
    let Ok(nm) = turnike_daemon::NetworkManagerClient::new().await else {
        return NetworkEnvironment::Unknown;
    };

    if let Ok(Some(ssid)) = nm.get_active_wifi_ssid().await {
        let target_ssid = turnike_daemon::load_config().target_ssid;
        if ssid.eq_ignore_ascii_case(&target_ssid) || ssid.eq_ignore_ascii_case("GSBWIFI") {
            let ts = nm
                .get_active_connection_start_time(&ssid)
                .await
                .ok()
                .flatten();
            return NetworkEnvironment::GsbWifi {
                ssid,
                connection_timestamp: ts,
            };
        } else if !ssid.is_empty() {
            return NetworkEnvironment::ForeignWifi { ssid };
        }
    }

    if let Ok(active_conns) = nm.get_active_connections().await
        && !active_conns.is_empty()
    {
        return NetworkEnvironment::Unknown;
    }

    NetworkEnvironment::Disconnected
}

fn apply_quota_bar_style(bar: &ProgressBar, ratio: f64) {
    bar.remove_css_class("warning");
    bar.remove_css_class("error");

    if ratio < 0.10 {
        bar.add_css_class("error");
    } else if ratio < 0.25 {
        bar.add_css_class("warning");
    }
}

#[derive(Clone, Debug)]
enum ActiveSessionTracker {
    NetworkTimestamp {
        started_instant: std::time::Instant,
        base_seconds: u64,
    },
    LoginTime {
        login_dt: glib::DateTime,
    },
    Duration {
        initial_seconds: u64,
        started_at: std::time::Instant,
    },
}

fn parse_login_time_to_datetime(raw: &str) -> Option<glib::DateTime> {
    let parts: Vec<&str> = raw
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() < 6 {
        return None;
    }

    let nums: Vec<i32> = parts.iter().filter_map(|s| s.parse().ok()).collect();
    if nums.len() < 6 {
        return None;
    }

    let (year, month, day, hour, minute, second) = if nums[0] >= 1000 {
        (nums[0], nums[1], nums[2], nums[3], nums[4], nums[5])
    } else if nums[2] >= 1000 {
        (nums[2], nums[1], nums[0], nums[3], nums[4], nums[5])
    } else {
        return None;
    };

    glib::DateTime::from_local(year, month, day, hour, minute, second as f64).ok()
}

fn parse_duration_to_seconds(raw: &str) -> Option<u64> {
    let nums: Vec<u64> = raw
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse().ok())
        .collect();

    if nums.len() == 4 {
        let (d, h, m, s) = (nums[0], nums[1], nums[2], nums[3]);
        Some(d * 86400 + h * 3600 + m * 60 + s)
    } else {
        None
    }
}

// Kullanıcı Turnike'yi açtığında oturum saatler önce telefondan veya arka plan daemon'ı
// tarafından açılmış olabilir. Doğrudan yerel Instant::now() kullanılsaydı sayaç her açılışta
// sıfırlanır ve KYK'nın 8 saatlik oturum sonlandırma sınırına ne kadar kaldığı anlaşılamazdı.
// Bu yüzden NetworkManager bağlantı zamanı ve portal login_time verisiyle temel zaman kurulur.
impl ActiveSessionTracker {
    fn new(
        network_connected_epoch: Option<u64>,
        login_time_str: Option<&str>,
        session_time_str: Option<&str>,
    ) -> Option<Self> {
        if let Some(ts) = network_connected_epoch
            && ts > 0
            && let Ok(dur) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        {
            let now_epoch = dur.as_secs();
            if now_epoch >= ts {
                let base_secs = now_epoch - ts;

                if base_secs < 7 * 86400 {
                    return Some(Self::NetworkTimestamp {
                        started_instant: std::time::Instant::now(),
                        base_seconds: base_secs,
                    });
                }
            }
        }

        if let Some(raw_login) = login_time_str
            && let Some(login_dt) = parse_login_time_to_datetime(raw_login)
            && let Ok(now) = glib::DateTime::now_local()
        {
            let diff = now.difference(&login_dt).as_seconds();

            if (-300..=86400).contains(&diff) {
                return Some(Self::LoginTime { login_dt });
            }
        }

        if let Some(raw_session) = session_time_str
            && let Some(secs) = parse_duration_to_seconds(raw_session)
        {
            return Some(Self::Duration {
                initial_seconds: secs,
                started_at: std::time::Instant::now(),
            });
        }

        None
    }

    fn elapsed_seconds(&self) -> u64 {
        match self {
            Self::NetworkTimestamp {
                started_instant,
                base_seconds,
            } => base_seconds.saturating_add(started_instant.elapsed().as_secs()),
            Self::LoginTime { login_dt } => {
                if let Ok(now) = glib::DateTime::now_local() {
                    let diff = now.difference(login_dt).as_seconds();
                    if diff > 0 { diff as u64 } else { 0 }
                } else {
                    0
                }
            }
            Self::Duration {
                initial_seconds,
                started_at,
            } => initial_seconds.saturating_add(started_at.elapsed().as_secs()),
        }
    }
}

impl MainWindow {
    pub fn new(app: &Application, mode: AppMode) -> Self {
        let gui_cfg = crate::config::load_gui_config();
        let initial_lang = gui_cfg
            .language
            .as_deref()
            .and_then(Language::from_code)
            .unwrap_or_else(Language::autodetect);
        let lang = Rc::new(Cell::new(initial_lang));

        let sm = libadwaita::StyleManager::default();
        match gui_cfg.color_scheme {
            1 => sm.set_color_scheme(libadwaita::ColorScheme::ForceLight),
            2 => sm.set_color_scheme(libadwaita::ColorScheme::ForceDark),
            _ => sm.set_color_scheme(libadwaita::ColorScheme::Default),
        }

        // Ağ ve D-Bus operasyonları GTK ana olay döngüsünü donduramaz.
        // Tokio görevleri ile arayüz arasındaki durum senkronizasyonu async_channel
        // mesaj kuyruğu üzerinden yürütülür.
        let (sender, receiver) = async_channel::unbounded::<ActionResponse>();
        let client = PortalClient::new().expect("Portal istemcisi başlatılamadı");

        let toast_overlay = ToastOverlay::new();
        let nav_view = NavigationView::new();

        let t = lang.get().texts();

        let toolbar_view = ToolbarView::new();
        let header_bar = HeaderBar::new();

        let window_title = WindowTitle::new(t.app.title, "");
        header_bar.set_title_widget(Some(&window_title));

        let refresh_btn = Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_tooltip_text(Some(t.actions.refresh_tooltip));
        header_bar.pack_start(&refresh_btn);

        let menu_btn = MenuButton::new();
        menu_btn.set_icon_name("open-menu-symbolic");
        menu_btn.set_tooltip_text(Some(t.app.main_menu_tooltip));
        header_bar.pack_end(&menu_btn);

        toolbar_view.add_top_bar(&header_bar);

        let view_stack = ViewStack::new();

        let loading_box = Box::new(Orientation::Vertical, 16);
        loading_box.set_valign(Align::Center);
        loading_box.set_halign(Align::Center);
        loading_box.set_vexpand(true);

        let loading_spinner = Spinner::new();
        loading_spinner.set_size_request(36, 36);
        loading_spinner.set_halign(Align::Center);
        loading_spinner.start();

        let loading_title = Label::new(Some(t.status.checking_connection));
        loading_title.add_css_class("title-4");
        loading_title.set_halign(Align::Center);

        loading_box.append(&loading_spinner);
        loading_box.append(&loading_title);
        view_stack.add_named(&loading_box, Some("loading"));

        let login_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .build();

        let login_clamp = Clamp::builder()
            .maximum_size(540)
            .tightening_threshold(400)
            .build();

        let login_box = Box::new(Orientation::Vertical, 14);
        login_box.set_margin_top(16);
        login_box.set_margin_bottom(18);
        login_box.set_margin_start(16);
        login_box.set_margin_end(16);

        let auth_group = PreferencesGroup::new();

        let username_row = EntryRow::builder()
            .title(t.auth.username_label)
            .input_purpose(InputPurpose::Digits)
            .show_apply_button(false)
            .build();

        let password_row = PasswordEntryRow::builder()
            .title(t.auth.password_label)
            .build();

        let remember_row = SwitchRow::builder()
            .title(t.auth.remember_me)
            .active(true)
            .build();

        auth_group.add(&username_row);
        auth_group.add(&password_row);
        auth_group.add(&remember_row);
        login_box.append(&auth_group);

        let login_action_btn = Button::with_label(t.auth.login_btn);
        login_action_btn.add_css_class("suggested-action");
        login_action_btn.add_css_class("pill");
        login_action_btn.set_halign(Align::Center);
        login_action_btn.set_size_request(190, 44);
        login_box.append(&login_action_btn);

        let account_group = PreferencesGroup::new();

        let register_row = ActionRow::builder()
            .title(t.auth.register_link)
            .activatable(true)
            .build();
        register_row.add_suffix(&Image::from_icon_name("go-next-symbolic"));

        let forgot_row = ActionRow::builder()
            .title(t.auth.forgot_password_link)
            .activatable(true)
            .build();
        forgot_row.add_suffix(&Image::from_icon_name("go-next-symbolic"));

        let login_help_row = ActionRow::builder()
            .title(t.help.title)
            .activatable(true)
            .build();
        login_help_row.add_suffix(&Image::from_icon_name("go-next-symbolic"));

        account_group.add(&register_row);
        account_group.add(&forgot_row);
        account_group.add(&login_help_row);
        login_box.append(&account_group);

        login_clamp.set_child(Some(&login_box));
        login_scroll.set_child(Some(&login_clamp));

        let network_banner = Banner::new("");
        network_banner.set_revealed(false);

        let login_container = Box::new(Orientation::Vertical, 0);
        login_container.append(&network_banner);
        login_container.append(&login_scroll);

        view_stack.add_named(&login_container, Some("login"));

        let dash_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .build();

        let dash_clamp = Clamp::builder()
            .maximum_size(540)
            .tightening_threshold(400)
            .build();

        let dash_box = Box::new(Orientation::Vertical, 14);
        dash_box.set_margin_top(16);
        dash_box.set_margin_bottom(18);
        dash_box.set_margin_start(16);
        dash_box.set_margin_end(16);

        let user_group = PreferencesGroup::builder().title(t.app.title).build();

        let session_row = ActionRow::builder().title(t.quota.session_time).build();
        let session_time_val = Label::new(Some("-"));
        session_time_val.add_css_class("dim-label");
        session_time_val.add_css_class("numeric");
        session_time_val.set_valign(Align::Center);
        session_row.add_suffix(&session_time_val);

        let wifi_ap_row = ActionRow::builder().title(t.quota.wifi_ap).build();
        let wifi_ap_val = Label::new(Some("-"));
        wifi_ap_val.add_css_class("dim-label");
        wifi_ap_val.add_css_class("numeric");
        wifi_ap_val.set_valign(Align::Center);
        wifi_ap_row.add_suffix(&wifi_ap_val);

        user_group.add(&session_row);
        user_group.add(&wifi_ap_row);
        dash_box.append(&user_group);

        let quotas_group = PreferencesGroup::builder().title(t.quota.title).build();

        let quotas_stack_box = Box::new(Orientation::Vertical, 12);

        let general_card = Box::new(Orientation::Vertical, 10);
        general_card.add_css_class("card");
        general_card.add_css_class("quota-card");

        let general_top = Box::new(Orientation::Horizontal, 8);
        let general_title = Label::new(Some(t.quota.general));
        general_title.set_xalign(0.0);
        general_title.set_hexpand(true);
        general_title.add_css_class("heading");

        let general_val = Label::new(Some("0.00 GB / 0.00 GB"));
        general_val.set_xalign(1.0);
        general_val.add_css_class("heading");
        general_val.add_css_class("numeric");

        general_top.append(&general_title);
        general_top.append(&general_val);

        let general_progress = ProgressBar::new();
        general_progress.add_css_class("quota-bar");
        general_progress.set_fraction(0.0);

        let general_reset_countdown = Label::new(Some("-"));
        general_reset_countdown.set_xalign(0.0);
        general_reset_countdown.add_css_class("caption");
        general_reset_countdown.add_css_class("dim-label");

        general_card.append(&general_top);
        general_card.append(&general_progress);
        general_card.append(&general_reset_countdown);

        let social_card = Box::new(Orientation::Vertical, 10);
        social_card.add_css_class("card");
        social_card.add_css_class("quota-card");

        let social_top = Box::new(Orientation::Horizontal, 8);
        let social_title = Label::new(Some(t.quota.social));
        social_title.set_xalign(0.0);
        social_title.set_hexpand(true);
        social_title.add_css_class("heading");

        let social_val = Label::new(Some("0.00 GB / 0.00 GB"));
        social_val.set_xalign(1.0);
        social_val.add_css_class("heading");
        social_val.add_css_class("numeric");

        social_top.append(&social_title);
        social_top.append(&social_val);

        let social_progress = ProgressBar::new();
        social_progress.add_css_class("quota-bar");
        social_progress.set_fraction(0.0);

        let social_reset_countdown = Label::new(Some("-"));
        social_reset_countdown.set_xalign(0.0);
        social_reset_countdown.add_css_class("caption");
        social_reset_countdown.add_css_class("dim-label");

        social_card.append(&social_top);
        social_card.append(&social_progress);
        social_card.append(&social_reset_countdown);

        quotas_stack_box.append(&general_card);
        quotas_stack_box.append(&social_card);
        quotas_group.add(&quotas_stack_box);
        dash_box.append(&quotas_group);

        let projection_group = PreferencesGroup::builder()
            .title(t.projection.title)
            .build();

        let projection_card = Box::new(Orientation::Vertical, 10);
        projection_card.add_css_class("card");
        projection_card.add_css_class("quota-card");

        let proj_metrics_box = Box::new(Orientation::Horizontal, 16);
        proj_metrics_box.set_homogeneous(true);

        let budget_box = Box::new(Orientation::Vertical, 4);
        let budget_title = Label::new(Some(t.projection.daily_safe_budget));
        budget_title.set_xalign(0.0);
        budget_title.add_css_class("caption");
        budget_title.add_css_class("dim-label");

        let budget_val = Label::new(Some("-"));
        budget_val.set_xalign(0.0);
        budget_val.add_css_class("heading");
        budget_val.add_css_class("numeric");
        budget_box.append(&budget_title);
        budget_box.append(&budget_val);

        let burn_box = Box::new(Orientation::Vertical, 4);
        let burn_title = Label::new(Some(t.projection.current_burn_rate));
        burn_title.set_xalign(0.0);
        burn_title.add_css_class("caption");
        burn_title.add_css_class("dim-label");

        let burn_val = Label::new(Some("-"));
        burn_val.set_xalign(0.0);
        burn_val.add_css_class("heading");
        burn_val.add_css_class("numeric");
        burn_box.append(&burn_title);
        burn_box.append(&burn_val);

        proj_metrics_box.append(&budget_box);
        proj_metrics_box.append(&burn_box);

        let proj_advice = Label::new(Some("-"));
        proj_advice.set_wrap(true);
        proj_advice.set_xalign(0.0);
        proj_advice.add_css_class("caption");
        proj_advice.add_css_class("dim-label");

        projection_card.append(&proj_metrics_box);
        projection_card.append(&proj_advice);

        projection_group.add(&projection_card);
        dash_box.append(&projection_group);

        let actions_group = PreferencesGroup::new();

        let change_pass_row = ActionRow::builder()
            .title(t.password.change_title)
            .activatable(true)
            .build();
        change_pass_row.add_suffix(&Image::from_icon_name("go-next-symbolic"));

        let last_actions_row = ActionRow::builder()
            .title(t.actions.title)
            .activatable(true)
            .build();
        last_actions_row.add_suffix(&Image::from_icon_name("go-next-symbolic"));

        let dash_help_row = ActionRow::builder()
            .title(t.help.title)
            .activatable(true)
            .build();
        dash_help_row.add_suffix(&Image::from_icon_name("go-next-symbolic"));

        actions_group.add(&change_pass_row);
        actions_group.add(&last_actions_row);
        actions_group.add(&dash_help_row);
        dash_box.append(&actions_group);

        let logout_btn = Button::with_label(t.auth.logout_btn);
        logout_btn.add_css_class("destructive-action");
        logout_btn.add_css_class("pill");
        logout_btn.set_halign(Align::Center);
        logout_btn.set_size_request(190, 44);
        dash_box.append(&logout_btn);

        dash_clamp.set_child(Some(&dash_box));
        dash_scroll.set_child(Some(&dash_clamp));
        view_stack.add_named(&dash_scroll, Some("dashboard"));

        toolbar_view.set_content(Some(&view_stack));

        let root_page = NavigationPage::builder()
            .title(t.app.title)
            .tag("home")
            .child(&toolbar_view)
            .build();
        nav_view.add(&root_page);

        toast_overlay.set_child(Some(&nav_view));

        let window = ApplicationWindow::builder()
            .application(app)
            .title(t.app.title)
            .default_width(500)
            .default_height(620)
            .width_request(420)
            .height_request(294)
            .content(&toast_overlay)
            .build();

        let rebuild_menu: Rc<dyn Fn(Language)> = {
            let menu_btn = menu_btn.clone();
            Rc::new(move |l: Language| {
                let t = l.texts();
                let menu_model = gtk4::gio::Menu::new();

                let prefs_section = gtk4::gio::Menu::new();
                prefs_section.append(Some(t.app.preferences_menu), Some("app.preferences"));
                menu_model.append_section(None, &prefs_section);

                let help_section = gtk4::gio::Menu::new();
                help_section.append(Some(t.help.title), Some("app.help"));
                help_section.append(Some(t.app.shortcuts), Some("app.shortcuts"));
                help_section.append(Some(t.app.about), Some("app.about"));
                menu_model.append_section(None, &help_section);

                menu_btn.set_menu_model(Some(&menu_model));
                menu_btn.set_tooltip_text(Some(t.app.main_menu_tooltip));
            })
        };
        rebuild_menu(lang.get());

        let current_username = Rc::new(RefCell::new(None::<String>));
        let last_quota_info = Rc::new(RefCell::new(None::<QuotaInfo>));
        let active_session = Rc::new(RefCell::new(None::<ActiveSessionTracker>));
        let network_conn_ts = Rc::new(RefCell::new(None::<u64>));
        let current_network_env = Rc::new(RefCell::new(NetworkEnvironment::Unknown));

        {
            let nav = nav_view.clone();
            let toast = toast_overlay.clone();
            let c = client.clone();
            let u_row = username_row.clone();
            let l = lang.clone();
            register_row.connect_activated(move |_| {
                let initial_u = Some(u_row.text().trim().to_string());
                push_register_page(&nav, &toast, c.clone(), initial_u, l.get());
            });
        }

        {
            let nav = nav_view.clone();
            let toast = toast_overlay.clone();
            let c = client.clone();
            let u_row = username_row.clone();
            let l = lang.clone();
            forgot_row.connect_activated(move |_| {
                let initial_u = Some(u_row.text().trim().to_string());
                push_forgot_password_page(&nav, &toast, c.clone(), initial_u, l.get());
            });
        }

        {
            let nav = nav_view.clone();
            let l = lang.clone();
            login_help_row.connect_activated(move |_| {
                push_help_page(&nav, l.get());
            });
        }

        {
            let nav = nav_view.clone();
            let l = lang.clone();
            dash_help_row.connect_activated(move |_| {
                push_help_page(&nav, l.get());
            });
        }

        {
            let nav = nav_view.clone();
            let toast = toast_overlay.clone();
            let c = client.clone();
            let u_row = username_row.clone();
            let c_user = current_username.clone();
            let l = lang.clone();
            change_pass_row.connect_activated(move |_| {
                let typed = u_row.text().trim().to_string();
                let u = if !typed.is_empty() {
                    Some(typed)
                } else {
                    c_user.borrow().clone()
                };
                push_password_page(&nav, &toast, c.clone(), u, None, l.get());
            });
        }

        {
            let nav = nav_view.clone();
            let c = client.clone();
            let l = lang.clone();
            last_actions_row.connect_activated(move |_| {
                push_actions_page(&nav, c.clone(), l.get());
            });
        }

        let update_dashboard_fn: Rc<dyn Fn(&QuotaInfo)> = {
            let l_dash = lang.clone();
            let user_group = user_group.clone();
            let session_row = session_row.clone();
            let session_time_val = session_time_val.clone();
            let wifi_ap_row = wifi_ap_row.clone();
            let general_title = general_title.clone();
            let general_val = general_val.clone();
            let general_progress = general_progress.clone();
            let general_reset_countdown = general_reset_countdown.clone();
            let social_title = social_title.clone();
            let social_val = social_val.clone();
            let social_progress = social_progress.clone();
            let social_reset_countdown = social_reset_countdown.clone();
            let projection_group = projection_group.clone();
            let budget_val = budget_val.clone();
            let burn_val = burn_val.clone();
            let proj_advice = proj_advice.clone();
            let c_user_for_dialog = current_username.clone();
            let last_info_store = last_quota_info.clone();
            let active_session_store = active_session.clone();
            let net_ts_dash = network_conn_ts.clone();

            Rc::new(move |info: &QuotaInfo| {
                if let Ok(mut store) = last_info_store.try_borrow_mut() {
                    *store = Some(info.clone());
                }
                let l = l_dash.get();
                let t = l.texts();

                session_row.set_title(t.quota.session_time);
                wifi_ap_row.set_title(t.quota.wifi_ap);
                general_title.set_text(t.quota.general);
                social_title.set_text(t.quota.social);

                if let Some(name) = &info.full_name {
                    let safe_name = glib::markup_escape_text(name);
                    user_group.set_title(safe_name.as_str());
                    *c_user_for_dialog.borrow_mut() = Some(name.clone());
                }

                let conn_ts = *net_ts_dash.borrow();
                let tracker = ActiveSessionTracker::new(
                    conn_ts,
                    info.login_time.as_deref(),
                    info.session_time.as_deref(),
                );
                if let Some(t) = &tracker {
                    let secs = t.elapsed_seconds();
                    session_time_val.set_text(&l.format_duration_seconds(secs));
                } else if let Some(session) = &info.session_time {
                    session_time_val.set_text(&l.format_portal_duration(session));
                }
                *active_session_store.borrow_mut() = tracker;

                for pkg in &info.packages {
                    let remaining_gb = pkg.remaining_mb / 1024.0;
                    let total_gb = pkg.total_mb / 1024.0;
                    let ratio = pkg.remaining_ratio();

                    let val_str = format!("{remaining_gb:.2} GB / {total_gb:.2} GB");
                    let countdown_str = l.format_quota_reset_countdown(&pkg.remaining_time);

                    if pkg.is_social() {
                        social_val.set_text(&val_str);
                        social_progress.set_fraction(ratio);
                        apply_quota_bar_style(&social_progress, ratio);
                        social_reset_countdown.set_text(&countdown_str);
                    } else {
                        general_val.set_text(&val_str);
                        general_progress.set_fraction(ratio);
                        apply_quota_bar_style(&general_progress, ratio);
                        general_reset_countdown.set_text(&countdown_str);
                    }
                }

                if let Some(proj) = info.calculate_projection(None) {
                    projection_group.set_visible(true);
                    budget_val.set_text(&l.format_mb_or_gb(proj.daily_allowance_mb));
                    let burn_unit = if l.is_tr() { "gün" } else { "day" };
                    burn_val.set_text(&format!(
                        "{}/{}",
                        l.format_mb_or_gb(proj.current_burn_rate_mb),
                        burn_unit
                    ));

                    let advice = l.format_burn_rate_advice(&proj);
                    proj_advice.set_text(&advice);
                } else {
                    projection_group.set_visible(false);
                }
            })
        };

        let apply_language_fn: Rc<dyn Fn(Language)> = {
            let rebuild_menu = rebuild_menu.clone();
            let refresh_btn = refresh_btn.clone();
            let loading_title = loading_title.clone();
            let username_row = username_row.clone();
            let password_row = password_row.clone();
            let remember_row = remember_row.clone();
            let login_action_btn = login_action_btn.clone();
            let register_row = register_row.clone();
            let forgot_row = forgot_row.clone();
            let login_help_row = login_help_row.clone();
            let quotas_group = quotas_group.clone();
            let projection_group = projection_group.clone();
            let budget_title = budget_title.clone();
            let burn_title = burn_title.clone();
            let general_title = general_title.clone();
            let social_title = social_title.clone();
            let change_pass_row = change_pass_row.clone();
            let last_actions_row = last_actions_row.clone();
            let dash_help_row = dash_help_row.clone();
            let logout_btn = logout_btn.clone();
            let window_ref = window.clone();
            let root_page_ref = root_page.clone();
            let window_title = window_title.clone();
            let view_stack = view_stack.clone();
            let last_info = last_quota_info.clone();
            let update_dash = update_dashboard_fn.clone();
            let net_banner_lang = network_banner.clone();
            let net_env_lang = current_network_env.clone();

            Rc::new(move |next: Language| {
                rebuild_menu(next);
                let t = next.texts();

                window_ref.set_title(Some(t.app.title));
                root_page_ref.set_title(t.app.title);
                window_title.set_title(t.app.title);

                refresh_btn.set_tooltip_text(Some(t.actions.refresh_tooltip));
                loading_title.set_text(t.status.checking_connection);

                username_row.set_title(t.auth.username_label);
                password_row.set_title(t.auth.password_label);
                remember_row.set_title(t.auth.remember_me);
                login_action_btn.set_label(t.auth.login_btn);

                register_row.set_title(t.auth.register_link);
                forgot_row.set_title(t.auth.forgot_password_link);
                login_help_row.set_title(t.help.title);

                wifi_ap_row.set_title(t.quota.wifi_ap);
                general_title.set_text(t.quota.general);
                social_title.set_text(t.quota.social);
                quotas_group.set_title(t.quota.title);
                projection_group.set_title(t.projection.title);
                budget_title.set_text(t.projection.daily_safe_budget);
                burn_title.set_text(t.projection.current_burn_rate);
                change_pass_row.set_title(t.password.change_title);
                last_actions_row.set_title(t.actions.title);
                dash_help_row.set_title(t.help.title);
                logout_btn.set_label(t.auth.logout_btn);

                if view_stack.visible_child_name().as_deref() == Some("dashboard") {
                    window_title.set_subtitle(t.status.connected);
                } else {
                    let cur_env = net_env_lang.borrow().clone();
                    match cur_env {
                        NetworkEnvironment::ForeignWifi { ref ssid } => {
                            window_title.set_subtitle(t.status.foreign_network);
                            net_banner_lang.set_title(&next.format_foreign_wifi_banner(ssid));
                        }
                        NetworkEnvironment::Disconnected => {
                            window_title.set_subtitle(t.status.no_connection);
                            net_banner_lang.set_title(t.network.disconnected_banner);
                        }
                        _ => {
                            window_title.set_subtitle(t.status.not_logged_in);
                        }
                    }
                }

                let maybe_cached = last_info.borrow().as_ref().cloned();
                if let Some(info) = maybe_cached {
                    update_dash(&info);
                }
            })
        };

        {
            let win_prefs = window.clone();
            let l_cell = lang.clone();
            let apply_lang = apply_language_fn.clone();
            let app_prefs = SimpleAction::new("preferences", None);
            app_prefs.connect_activate(move |_, _| {
                show_preferences_dialog(&win_prefs, l_cell.clone(), apply_lang.clone());
            });
            app.add_action(&app_prefs);
        }

        {
            let nav = nav_view.clone();
            let l = lang.clone();
            let app_help = SimpleAction::new("help", None);
            app_help.connect_activate(move |_, _| {
                push_help_page(&nav, l.get());
            });
            app.add_action(&app_help);
        }

        {
            let win_sc = window.clone();
            let l = lang.clone();
            let app_shortcuts = SimpleAction::new("shortcuts", None);
            app_shortcuts.connect_activate(move |_, _| {
                show_shortcuts_dialog(&win_sc, l.get());
            });
            app.add_action(&app_shortcuts);
        }

        {
            let win_about = window.clone();
            let l = lang.clone();
            let app_about = SimpleAction::new("about", None);
            app_about.connect_activate(move |_, _| {
                let cur = l.get();
                let t = cur.texts();
                let about = AboutDialog::builder()
                    .application_name(t.app.title)
                    .application_icon(crate::APP_ID)
                    .version(env!("CARGO_PKG_VERSION"))
                    .developer_name(t.app.about_developer)
                    .designers(["koder-cog"])
                    .license_type(gtk4::License::Agpl30)
                    .comments(t.app.about_comments)
                    .issue_url("https://github.com/koder-cog/turnike/issues")
                    .build();
                about.add_link(
                    t.app.about_source_code,
                    "https://github.com/koder-cog/turnike",
                );

                about.present(Some(&win_about));
            });

            app.add_action(&app_about);
        }

        let trigger_wifi_ap_query: Rc<dyn Fn()> = {
            let sender = sender.clone();
            Rc::new(move || {
                let s = sender.clone();
                glib::spawn_future_local(async move {
                    if let Ok(nm) = turnike_daemon::NetworkManagerClient::new().await {
                        let ap_info = nm.get_current_ap_info().await.ok().flatten();
                        let _ = s.send(ActionResponse::WifiApUpdated(ap_info)).await;
                    }
                });
            })
        };

        {
            let view_stack_clone = view_stack.clone();
            let window_title_clone = window_title.clone();
            let login_action_btn_clone = login_action_btn.clone();
            let toast_overlay_clone = toast_overlay.clone();
            let username_row_clone = username_row.clone();
            let password_row_clone = password_row.clone();
            let logout_btn_clone = logout_btn.clone();
            let update_dash = update_dashboard_fn.clone();
            let l_recv = lang.clone();
            let active_session_recv = active_session.clone();
            let session_time_val_recv = session_time_val.clone();
            let wifi_ap_val_recv = wifi_ap_val.clone();
            let trigger_wifi_clone = trigger_wifi_ap_query.clone();
            let network_banner_recv = network_banner.clone();
            let current_network_env_recv = current_network_env.clone();
            let network_conn_ts_recv = network_conn_ts.clone();
            let last_info_recv = last_quota_info.clone();

            glib::spawn_future_local(async move {
                while let Ok(msg) = receiver.recv().await {
                    let l = l_recv.get();
                    let t = l.texts();
                    match msg {
                        ActionResponse::NetworkEnvironmentUpdated(env) => {
                            *current_network_env_recv.borrow_mut() = env.clone();
                            match env {
                                NetworkEnvironment::ForeignWifi { ssid } => {
                                    network_banner_recv
                                        .set_title(&l.format_foreign_wifi_banner(&ssid));
                                    network_banner_recv.set_revealed(true);
                                    login_action_btn_clone.set_sensitive(false);
                                    if view_stack_clone.visible_child_name().as_deref()
                                        != Some("dashboard")
                                    {
                                        window_title_clone.set_subtitle(t.status.foreign_network);
                                    }
                                }
                                NetworkEnvironment::Disconnected => {
                                    network_banner_recv.set_title(t.network.disconnected_banner);
                                    network_banner_recv.set_revealed(true);
                                    login_action_btn_clone.set_sensitive(false);
                                    if view_stack_clone.visible_child_name().as_deref()
                                        != Some("dashboard")
                                    {
                                        window_title_clone.set_subtitle(t.status.no_connection);
                                    }
                                }
                                NetworkEnvironment::GsbWifi {
                                    connection_timestamp,
                                    ..
                                } => {
                                    network_banner_recv.set_revealed(false);
                                    login_action_btn_clone.set_sensitive(true);
                                    *network_conn_ts_recv.borrow_mut() = connection_timestamp;
                                    let cached_info = last_info_recv.borrow().as_ref().cloned();
                                    if view_stack_clone.visible_child_name().as_deref()
                                        == Some("login")
                                    {
                                        window_title_clone.set_subtitle(t.status.not_logged_in);
                                    } else if view_stack_clone.visible_child_name().as_deref()
                                        == Some("dashboard")
                                        && let Some(info) = cached_info
                                    {
                                        update_dash(&info);
                                    }
                                }
                                NetworkEnvironment::Unknown => {
                                    network_banner_recv.set_revealed(false);
                                    login_action_btn_clone.set_sensitive(true);
                                }
                            }
                        }
                        ActionResponse::StatusUpdated(state) => match state {
                            SessionState::Authenticated(info) => {
                                window_title_clone.set_subtitle(t.status.connected);
                                update_dash(&info);
                                trigger_wifi_clone();
                                view_stack_clone.set_visible_child_name("dashboard");
                            }
                            SessionState::CaptiveLocked => {
                                let env = current_network_env_recv.borrow().clone();
                                if let NetworkEnvironment::ForeignWifi { ref ssid } = env {
                                    window_title_clone.set_subtitle(t.status.foreign_network);
                                    network_banner_recv
                                        .set_title(&l.format_foreign_wifi_banner(ssid));
                                    network_banner_recv.set_revealed(true);
                                    login_action_btn_clone.set_sensitive(false);
                                } else if let NetworkEnvironment::Disconnected = env {
                                    window_title_clone.set_subtitle(t.status.no_connection);
                                    network_banner_recv.set_title(t.network.disconnected_banner);
                                    network_banner_recv.set_revealed(true);
                                    login_action_btn_clone.set_sensitive(false);
                                } else {
                                    window_title_clone.set_subtitle(t.status.not_logged_in);
                                    login_action_btn_clone.set_sensitive(true);
                                    network_banner_recv.set_revealed(false);
                                }
                                *active_session_recv.borrow_mut() = None;
                                session_time_val_recv.set_text("-");
                                wifi_ap_val_recv.set_text("-");
                                view_stack_clone.set_visible_child_name("login");
                            }
                            SessionState::Offline => {
                                let env = current_network_env_recv.borrow().clone();
                                if let NetworkEnvironment::ForeignWifi { ref ssid } = env {
                                    window_title_clone.set_subtitle(t.status.foreign_network);
                                    network_banner_recv
                                        .set_title(&l.format_foreign_wifi_banner(ssid));
                                    network_banner_recv.set_revealed(true);
                                    login_action_btn_clone.set_sensitive(false);
                                } else if let NetworkEnvironment::Disconnected = env {
                                    window_title_clone.set_subtitle(t.status.no_connection);
                                    network_banner_recv.set_title(t.network.disconnected_banner);
                                    network_banner_recv.set_revealed(true);
                                    login_action_btn_clone.set_sensitive(false);
                                } else {
                                    window_title_clone.set_subtitle(t.status.no_connection);
                                    login_action_btn_clone.set_sensitive(true);
                                    network_banner_recv.set_revealed(false);
                                }
                                *active_session_recv.borrow_mut() = None;
                                session_time_val_recv.set_text("-");
                                wifi_ap_val_recv.set_text("-");
                                view_stack_clone.set_visible_child_name("login");
                            }
                        },
                        ActionResponse::LoginSuccess(info) => {
                            window_title_clone.set_subtitle(t.status.connected);
                            update_dash(&info);
                            trigger_wifi_clone();
                            view_stack_clone.set_visible_child_name("dashboard");
                            login_action_btn_clone.set_sensitive(true);
                        }
                        ActionResponse::LoginFailed(err) => {
                            let msg = l.portal_error_message(&err);
                            toast_overlay_clone.add_toast(Toast::new(&msg));
                            window_title_clone.set_subtitle(t.status.login_failed);
                            login_action_btn_clone.set_sensitive(true);
                            view_stack_clone.set_visible_child_name("login");
                        }
                        ActionResponse::LogoutSuccess => {
                            let env = current_network_env_recv.borrow().clone();
                            if let NetworkEnvironment::ForeignWifi { ref ssid } = env {
                                window_title_clone.set_subtitle(t.status.foreign_network);
                                network_banner_recv.set_title(&l.format_foreign_wifi_banner(ssid));
                                network_banner_recv.set_revealed(true);
                                login_action_btn_clone.set_sensitive(false);
                            } else if let NetworkEnvironment::Disconnected = env {
                                window_title_clone.set_subtitle(t.status.no_connection);
                                network_banner_recv.set_title(t.network.disconnected_banner);
                                network_banner_recv.set_revealed(true);
                                login_action_btn_clone.set_sensitive(false);
                            } else {
                                window_title_clone.set_subtitle(t.status.not_logged_in);
                                login_action_btn_clone.set_sensitive(true);
                                network_banner_recv.set_revealed(false);
                            }
                            logout_btn_clone.set_sensitive(true);
                            *active_session_recv.borrow_mut() = None;
                            session_time_val_recv.set_text("-");
                            wifi_ap_val_recv.set_text("-");
                            view_stack_clone.set_visible_child_name("login");
                        }
                        ActionResponse::LogoutFailed(err) => {
                            let msg = l.portal_error_message(&err);
                            toast_overlay_clone.add_toast(Toast::new(&msg));
                            logout_btn_clone.set_sensitive(true);
                            view_stack_clone.set_visible_child_name("dashboard");
                        }
                        ActionResponse::CredentialsLoaded(maybe_creds) => {
                            if let Some(creds) = maybe_creds {
                                username_row_clone.set_text(&creds.username);

                                let pw_len = creds.password.chars().count();
                                if (6..=10).contains(&pw_len) {
                                    password_row_clone.set_text(&creds.password);
                                }
                            }
                        }
                        ActionResponse::WifiApUpdated(opt_ap) => {
                            if let Some(ap) = opt_ap {
                                let band = if ap.is_5ghz { "5 GHz" } else { "2.4 GHz" };
                                let strength_str = if l.is_tr() {
                                    format!("%{}", ap.strength)
                                } else {
                                    format!("{}%", ap.strength)
                                };
                                wifi_ap_val_recv.set_text(&format!("{band} ({strength_str})"));
                            } else {
                                wifi_ap_val_recv.set_text("-");
                            }
                        }
                    }
                }
            });
        }

        if mode == AppMode::Normal {
            let s_load = sender.clone();
            tokio::spawn(async move {
                let loaded = SecretStore::load_async().await.ok().flatten();
                let _ = s_load.send(ActionResponse::CredentialsLoaded(loaded)).await;
            });

            let s_init = sender.clone();
            let c_init = client.clone();
            tokio::spawn(async move {
                let env = check_network_environment().await;
                let _ = s_init
                    .send(ActionResponse::NetworkEnvironmentUpdated(env.clone()))
                    .await;
                if !matches!(
                    env,
                    NetworkEnvironment::ForeignWifi { .. } | NetworkEnvironment::Disconnected
                ) {
                    let state = c_init.get_status().await.unwrap_or(SessionState::Offline);
                    let _ = s_init.send(ActionResponse::StatusUpdated(state)).await;
                } else {
                    let _ = s_init
                        .send(ActionResponse::StatusUpdated(SessionState::Offline))
                        .await;
                }
            });
            trigger_wifi_ap_query();
        }

        let do_login: Rc<dyn Fn()> = {
            let sender = sender.clone();
            let client = client.clone();
            let u_row = username_row.clone();
            let p_row = password_row.clone();
            let rem_row = remember_row.clone();
            let l_btn = login_action_btn.clone();
            let v_stack = view_stack.clone();
            let net_env = current_network_env.clone();
            let toast = toast_overlay.clone();
            let l_login = lang.clone();

            Rc::new(move || {
                let env = net_env.borrow().clone();
                let l = l_login.get();
                let t = l.texts();

                if let NetworkEnvironment::ForeignWifi { ssid } = env {
                    let msg = format!(
                        "{}. ({}: {})",
                        t.network.foreign_wifi_toast, t.quota.wifi_ap, ssid
                    );
                    toast.add_toast(Toast::new(&msg));
                    return;
                }
                if let NetworkEnvironment::Disconnected = env {
                    toast.add_toast(Toast::new(t.network.disconnected_banner));
                    return;
                }

                let username = u_row.text().trim().to_string();
                let password = p_row.text().trim().to_string();

                if username.is_empty() {
                    u_row.grab_focus();
                    return;
                }
                if password.is_empty() {
                    p_row.grab_focus();
                    return;
                }

                let creds = Credentials::new(username, password);
                let remember = rem_row.is_active();

                l_btn.set_sensitive(false);
                v_stack.set_visible_child_name("loading");

                let s = sender.clone();
                let c = client.clone();
                tokio::spawn(async move {
                    if remember {
                        let _ = SecretStore::save_async(creds.clone()).await;
                    } else {
                        let _ = SecretStore::clear_async().await;
                    }

                    match c.login(&creds).await {
                        Ok(info) => {
                            let _ = s.send(ActionResponse::LoginSuccess(info)).await;
                        }
                        Err(e) => {
                            let _ = s.send(ActionResponse::LoginFailed(e)).await;
                        }
                    }
                });
            })
        };

        let do_login_click = do_login.clone();
        login_action_btn.connect_clicked(move |_| {
            do_login_click();
        });

        let do_login_enter = do_login.clone();
        password_row.connect_entry_activated(move |_| {
            do_login_enter();
        });

        let do_login_user_enter = do_login.clone();
        username_row.connect_entry_activated(move |_| {
            do_login_user_enter();
        });

        remember_row.connect_active_notify(|sw| {
            if !sw.is_active() {
                tokio::spawn(async {
                    let _ = SecretStore::clear_async().await;
                });
            }
        });

        let s_logout = sender.clone();
        let c_logout = client.clone();
        let lo_btn = logout_btn.clone();
        let v_stack_lo = view_stack.clone();
        let win_lo = window.clone();
        let lang_lo = lang.clone();
        let current_user_lo = current_username.clone();

        logout_btn.connect_clicked(move |_| {
            let t = lang_lo.get().texts();
            let heading = if let Some(name) = current_user_lo.borrow().as_deref() {
                if lang_lo.get().is_tr() {
                    format!("{name} Oturumunu Kapat")
                } else {
                    format!("Log Out {name}")
                }
            } else {
                t.auth.logout_dialog_heading.to_string()
            };

            let alert = AlertDialog::builder()
                .heading(&heading)
                .body(t.auth.logout_dialog_body)
                .prefer_wide_layout(true)
                .build();
            alert.add_response("cancel", t.auth.logout_dialog_cancel);
            alert.add_response("logout", t.auth.logout_dialog_confirm);
            alert.set_response_appearance("logout", ResponseAppearance::Destructive);
            alert.set_default_response(Some("cancel"));
            alert.set_close_response("cancel");

            let s = s_logout.clone();
            let c = c_logout.clone();
            let lo = lo_btn.clone();
            let v_stack = v_stack_lo.clone();

            alert.choose(&win_lo, None::<&gio::Cancellable>, move |response| {
                if response == "logout" {
                    lo.set_sensitive(false);
                    v_stack.set_visible_child_name("loading");

                    tokio::spawn(async move {
                        match c.logout().await {
                            Ok(_) => {
                                let _ = s.send(ActionResponse::LogoutSuccess).await;
                            }
                            Err(e) => {
                                let _ = s.send(ActionResponse::LogoutFailed(e)).await;
                            }
                        }
                    });
                }
            });
        });

        let trigger_refresh: Rc<dyn Fn()> = {
            let sender = sender.clone();
            let client = client.clone();
            let v_stack = view_stack.clone();
            let nav = nav_view.clone();
            let trigger_wifi = trigger_wifi_ap_query.clone();

            Rc::new(move || {
                let _ = nav.pop_to_tag("home");
                v_stack.set_visible_child_name("loading");
                trigger_wifi();
                let s = sender.clone();
                let c = client.clone();
                tokio::spawn(async move {
                    let env = check_network_environment().await;
                    let _ = s
                        .send(ActionResponse::NetworkEnvironmentUpdated(env.clone()))
                        .await;
                    if !matches!(
                        env,
                        NetworkEnvironment::ForeignWifi { .. } | NetworkEnvironment::Disconnected
                    ) {
                        let state = c.get_status().await.unwrap_or(SessionState::Offline);
                        let _ = s.send(ActionResponse::StatusUpdated(state)).await;
                    } else {
                        let _ = s
                            .send(ActionResponse::StatusUpdated(SessionState::Offline))
                            .await;
                    }
                });
            })
        };

        let tr_click = trigger_refresh.clone();
        refresh_btn.connect_clicked(move |_| {
            tr_click();
        });

        {
            let tr_act = trigger_refresh.clone();
            let action_refresh = SimpleAction::new("refresh", None);
            action_refresh.connect_activate(move |_, _| {
                tr_act();
            });
            window.add_action(&action_refresh);
        }

        {
            let nav_close = nav_view.clone();
            let win_close = window.clone();
            let action_close = SimpleAction::new("close", None);
            action_close.connect_activate(move |_, _| {
                if !nav_close.pop() {
                    win_close.close();
                }
            });
            window.add_action(&action_close);
        }

        {
            let app_c = app.clone();
            let app_quit = SimpleAction::new("quit", None);
            app_quit.connect_activate(move |_, _| {
                app_c.quit();
            });
            app.add_action(&app_quit);
        }

        app.set_accels_for_action("app.preferences", &["<Control>comma"]);
        app.set_accels_for_action("app.help", &["F1"]);
        app.set_accels_for_action("app.shortcuts", &["<Control>question", "<Control>slash"]);
        app.set_accels_for_action("app.quit", &["<Control>q"]);
        app.set_accels_for_action("win.refresh", &["<Control>r"]);
        app.set_accels_for_action("win.close", &["<Control>w"]);

        let key_controller = EventControllerKey::new();
        let nav_esc = nav_view.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == Key::Escape && nav_esc.pop() {
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_controller);

        let s_poll = sender.clone();
        let c_poll = client.clone();
        glib::spawn_future_local(async move {
            loop {
                glib::timeout_future(std::time::Duration::from_secs(60)).await;
                let c = c_poll.clone();
                let s = s_poll.clone();
                tokio::spawn(async move {
                    let env = check_network_environment().await;
                    let _ = s
                        .send(ActionResponse::NetworkEnvironmentUpdated(env.clone()))
                        .await;
                    if !matches!(
                        env,
                        NetworkEnvironment::ForeignWifi { .. } | NetworkEnvironment::Disconnected
                    ) {
                        let state = c.get_status().await.unwrap_or(SessionState::Offline);
                        let _ = s.send(ActionResponse::StatusUpdated(state)).await;
                    } else {
                        let _ = s
                            .send(ActionResponse::StatusUpdated(SessionState::Offline))
                            .await;
                    }
                });
            }
        });

        if mode == AppMode::Normal {
            let session_tracker = active_session.clone();
            let session_val = session_time_val.clone();
            let l_cell = lang.clone();
            let view_stack = view_stack.clone();
            let s_ticker = sender.clone();
            let tick_counter = Rc::new(Cell::new(0u32));

            glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
                let count = tick_counter.get() + 1;
                tick_counter.set(count);

                if view_stack.visible_child_name().as_deref() == Some("dashboard")
                    && let Some(tracker) = session_tracker.borrow().as_ref()
                {
                    let secs = tracker.elapsed_seconds();
                    let l = l_cell.get();
                    session_val.set_text(&l.format_duration_seconds(secs));
                } else if view_stack.visible_child_name().as_deref() == Some("login")
                    && count.is_multiple_of(4)
                {
                    let s = s_ticker.clone();
                    glib::spawn_future_local(async move {
                        let env = check_network_environment().await;
                        let _ = s.send(ActionResponse::NetworkEnvironmentUpdated(env)).await;
                    });
                }
                glib::ControlFlow::Continue
            });
        } else {
            let demo_info = QuotaInfo {
                full_name: Some("Ali Yılmaz".to_string()),
                packages: vec![
                    turnike_types::QuotaPackage {
                        package_type: "Genel İnternet".to_string(),
                        total_mb: 32768.0,
                        remaining_mb: 13803.52,
                        start_date: "01/10/2026".to_string(),
                        expiration_date: "31/10/2026 23:59:59".to_string(),
                        remaining_time: "15 23 5".to_string(),
                    },
                    turnike_types::QuotaPackage {
                        package_type: "Sosyal Medya".to_string(),
                        total_mb: 5120.0,
                        remaining_mb: 0.0,
                        start_date: "01/10/2026".to_string(),
                        expiration_date: "31/10/2026 23:59:59".to_string(),
                        remaining_time: "15 23 5".to_string(),
                    },
                ],
                login_time: Some("16/10/2026 12:00:00".to_string()),
                session_time: Some("0 1 24 10".to_string()),
            };

            update_dashboard_fn(&demo_info);
            window_title.set_subtitle(t.status.connected);
            wifi_ap_val.set_text("5 GHz (%84)");
            budget_val.set_text("850 MB");
            burn_val.set_text("1.20 GB");
            proj_advice.set_text("Bu hızla ayın 22. gününde kota biter");
            view_stack.set_visible_child_name("dashboard");

            if mode == AppMode::SmokeTest {
                let app_clone = app.clone();
                let v_stack = view_stack.clone();
                let apply_lang = apply_language_fn.clone();
                let ref_btn = refresh_btn.clone();

                glib::timeout_add_local_once(std::time::Duration::from_millis(80), move || {
                    apply_lang(Language::En);
                    v_stack.set_visible_child_name("login");

                    let app_c2 = app_clone.clone();
                    let v_stack2 = v_stack.clone();
                    let apply_lang2 = apply_lang.clone();
                    let ref_btn2 = ref_btn.clone();
                    glib::timeout_add_local_once(std::time::Duration::from_millis(80), move || {
                        apply_lang2(Language::Tr);
                        v_stack2.set_visible_child_name("dashboard");
                        ref_btn2.emit_clicked();
                        println!("Turnike UI duman testi (smoke test) başarıyla tamamlandı.");
                        app_c2.quit();
                    });
                });
            }
        }

        Self { window }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_login_time_to_datetime() {
        let dt = parse_login_time_to_datetime("14/10/2026 21:45:57").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 10);
        assert_eq!(dt.day_of_month(), 14);
        assert_eq!(dt.hour(), 21);
        assert_eq!(dt.minute(), 45);
        assert_eq!(dt.seconds(), 57.0);

        let dt_iso = parse_login_time_to_datetime("2026-10-14 21:45:57").unwrap();
        assert_eq!(dt_iso.year(), 2026);
        assert_eq!(dt_iso.day_of_month(), 14);

        assert!(parse_login_time_to_datetime("invalid date").is_none());
    }

    #[test]
    fn test_parse_duration_to_seconds() {
        assert_eq!(parse_duration_to_seconds("0 gn 0 sa 12 dk 5 sn"), Some(725));
        assert_eq!(parse_duration_to_seconds("0 Day 1 h 0 m 0 s"), Some(3600));
        assert_eq!(parse_duration_to_seconds("invalid"), None);
    }

    #[test]
    fn test_active_session_tracker_network_timestamp() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let conn_ts = now - 3600;
        let tracker = ActiveSessionTracker::new(Some(conn_ts), None, None).unwrap();
        let elapsed = tracker.elapsed_seconds();
        assert!((3600..=3605).contains(&elapsed));
    }
}
