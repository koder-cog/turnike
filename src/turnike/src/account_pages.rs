use crate::config::{load_gui_config, save_gui_config};
use crate::i18n::{Language, RowTextExt};
use crate::password_dialog::{build_live_rules_group, push_password_page};
use adw::prelude::*;
use adw::{
    ActionRow, Clamp, ColorScheme, ComboRow, EntryRow, HeaderBar, NavigationPage, NavigationView,
    PasswordEntryRow, PreferencesDialog, PreferencesGroup, PreferencesPage, StyleManager,
    SwitchRow, Toast, ToastOverlay, ToolbarView,
};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, Label, Orientation, PolicyType, Scale, ScrolledWindow, ShortcutsGroup,
    ShortcutsSection, ShortcutsShortcut, ShortcutsWindow, StringList,
};
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use turnike_client::PortalClient;
use turnike_daemon::{load_config, save_config};
use turnike_types::{
    Nationality, PasswordResetRequest, QuotaThresholdMode, RegistrationRequest, UserType,
    validate_password,
};

fn format_threshold_label(mode: QuotaThresholdMode, value: f64, is_tr: bool) -> String {
    match mode {
        QuotaThresholdMode::Percentage => {
            if is_tr {
                format!("%{value:.0}")
            } else {
                format!("{value:.0}%")
            }
        }
        QuotaThresholdMode::Gigabytes => format!("{value:.1} GB"),
    }
}

pub fn push_register_page(
    nav_view: &NavigationView,
    toast_overlay: &ToastOverlay,
    client: PortalClient,
    initial_username: Option<String>,
    lang: Language,
) {
    let toolbar_view = ToolbarView::new();
    let header_bar = HeaderBar::new();
    toolbar_view.add_top_bar(&header_bar);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .build();

    let clamp = Clamp::builder()
        .maximum_size(540)
        .tightening_threshold(400)
        .build();

    let content_box = Box::new(Orientation::Vertical, 14);
    content_box.set_margin_top(16);
    let t = lang.texts();
    content_box.set_margin_bottom(18);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);

    let form_group = PreferencesGroup::new();

    let nationality_model =
        StringList::new(&[t.register.nationality_tc, t.register.nationality_foreigner]);
    let nationality_row = ComboRow::builder()
        .title(t.register.nationality_label)
        .model(&nationality_model)
        .selected(0)
        .build();
    form_group.add(&nationality_row);

    let user_type_labels = t.register.user_types;
    let user_type_model = StringList::new(&user_type_labels);
    let user_type_row = ComboRow::builder()
        .title(t.register.user_type_label)
        .model(&user_type_model)
        .selected(0)
        .build();
    form_group.add(&user_type_row);

    let id_row = EntryRow::builder()
        .title(t.auth.username_label)
        .show_apply_button(false)
        .build();
    if let Some(u) = initial_username.filter(|s| !s.is_empty()) {
        id_row.set_text(&u);
    }
    form_group.add(&id_row);

    let first_pass_row = PasswordEntryRow::builder()
        .title(t.password.first_label)
        .build();
    form_group.add(&first_pass_row);

    content_box.append(&form_group);

    let submit_btn = Button::with_label(t.register.verify_btn);
    submit_btn.add_css_class("suggested-action");
    submit_btn.add_css_class("pill");
    submit_btn.set_halign(Align::Center);
    submit_btn.set_size_request(200, 44);

    content_box.append(&submit_btn);

    clamp.set_child(Some(&content_box));
    scroll.set_child(Some(&clamp));
    toolbar_view.set_content(Some(&scroll));

    let page = NavigationPage::builder()
        .title(t.register.title)
        .child(&toolbar_view)
        .build();

    let nav_clone = nav_view.clone();
    let toast_clone = toast_overlay.clone();
    let btn_clone = submit_btn.clone();

    submit_btn.connect_clicked(move |_| {
        let identity = id_row.text().trim().to_string();
        let first_pass = first_pass_row.text().trim().to_string();

        if identity.is_empty() {
            id_row.grab_focus();
            return;
        }
        if first_pass.is_empty() {
            first_pass_row.grab_focus();
            return;
        }

        let nationality = if nationality_row.selected() == 0 {
            Nationality::Tc
        } else {
            Nationality::Foreigner
        };
        let user_type = UserType::from_index(user_type_row.selected());

        let req = RegistrationRequest {
            nationality,
            user_type,
            identity_number: identity.clone(),
            first_password: first_pass,
        };

        btn_clone.set_sensitive(false);

        let c = client.clone();
        let b_done = btn_clone.clone();
        let t_done = toast_clone.clone();
        let nav_done = nav_clone.clone();

        glib::spawn_future_local(async move {
            let first_pass_saved = req.first_password.clone();
            match c.register_first_user(&req).await {
                Ok(_) => {
                    b_done.set_sensitive(true);
                    push_password_page(
                        &nav_done,
                        &t_done,
                        c,
                        Some(identity),
                        Some(first_pass_saved),
                        lang,
                    );
                }
                Err(e) => {
                    b_done.set_sensitive(true);
                    let err_msg = lang.portal_error_message(&e);
                    t_done.add_toast(Toast::new(&err_msg));
                }
            }
        });
    });

    nav_view.push(&page);
}

pub fn push_forgot_password_page(
    nav_view: &NavigationView,
    toast_overlay: &ToastOverlay,
    client: PortalClient,
    initial_username: Option<String>,
    lang: Language,
) {
    let toolbar_view = ToolbarView::new();
    let header_bar = HeaderBar::new();
    toolbar_view.add_top_bar(&header_bar);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .build();

    let clamp = Clamp::builder()
        .maximum_size(540)
        .tightening_threshold(400)
        .build();

    let t = lang.texts();
    let content_box = Box::new(Orientation::Vertical, 14);
    content_box.set_margin_top(16);
    content_box.set_margin_bottom(18);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);

    let form_group = PreferencesGroup::builder()
        .description(t.register.edevlet_info)
        .build();

    let username_row = EntryRow::builder()
        .title(t.auth.username_label)
        .show_apply_button(false)
        .build();
    if let Some(u) = initial_username.filter(|s| !s.is_empty()) {
        username_row.set_text(&u);
    }
    form_group.add(&username_row);

    let temp_pass_row = PasswordEntryRow::builder()
        .title(t.password.temporary_label)
        .build();
    form_group.add(&temp_pass_row);

    let new_pass_row = PasswordEntryRow::builder()
        .title(t.password.new_label)
        .build();
    form_group.add(&new_pass_row);

    let confirm_pass_row = PasswordEntryRow::builder()
        .title(t.password.confirm_label)
        .build();
    form_group.add(&confirm_pass_row);

    let (rules_group, update_rules) = build_live_rules_group(lang);

    let u_row_for_pass = username_row.clone();
    let updater_pass = update_rules.clone();
    new_pass_row.connect_changed(move |entry| {
        let user_text = u_row_for_pass.text();
        updater_pass(&entry.text(), Some(user_text.as_str()));
    });

    let p_row_for_user = new_pass_row.clone();
    let updater_user = update_rules.clone();
    username_row.connect_changed(move |entry| {
        let pass_text = p_row_for_user.text();
        if !pass_text.is_empty() {
            updater_user(&pass_text, Some(entry.text().as_str()));
        }
    });

    let save_btn = Button::with_label(t.password.save_btn);
    save_btn.add_css_class("suggested-action");
    save_btn.add_css_class("pill");
    save_btn.set_halign(Align::Center);
    save_btn.set_size_request(200, 44);

    content_box.append(&form_group);
    content_box.append(&rules_group);
    content_box.append(&save_btn);

    clamp.set_child(Some(&content_box));
    scroll.set_child(Some(&clamp));
    toolbar_view.set_content(Some(&scroll));

    let page = NavigationPage::builder()
        .title(t.auth.forgot_password_link)
        .child(&toolbar_view)
        .build();

    let nav_clone = nav_view.clone();
    let toast_clone = toast_overlay.clone();
    let btn_clone = save_btn.clone();

    save_btn.connect_clicked(move |_| {
        let username = username_row.text().trim().to_string();
        let temp_p = temp_pass_row.text().trim().to_string();
        let new_p = new_pass_row.text().trim().to_string();
        let confirm_p = confirm_pass_row.text().trim().to_string();

        if username.is_empty() {
            username_row.grab_focus();
            return;
        }
        if temp_p.is_empty() {
            temp_pass_row.grab_focus();
            return;
        }
        if new_p.is_empty() {
            new_pass_row.grab_focus();
            return;
        }
        if confirm_p.is_empty() {
            confirm_pass_row.grab_focus();
            return;
        }

        if new_p != confirm_p {
            toast_clone.add_toast(Toast::new(t.password.mismatch_toast));
            return;
        }

        if let Err(violation) = validate_password(&new_p, Some(&username)) {
            toast_clone.add_toast(Toast::new(&lang.violation_message(&violation)));
            return;
        }

        let req = PasswordResetRequest {
            username,
            temporary_password: temp_p,
            new_password: new_p,
        };

        btn_clone.set_sensitive(false);

        let c = client.clone();
        let b_done = btn_clone.clone();
        let t_done = toast_clone.clone();
        let nav_done = nav_clone.clone();

        glib::spawn_future_local(async move {
            match c.reset_password_edevlet(&req).await {
                Ok(_) => {
                    t_done.add_toast(Toast::new(t.password.updated_toast));
                    nav_done.pop();
                }
                Err(e) => {
                    b_done.set_sensitive(true);
                    let err_msg = lang.portal_error_message(&e);
                    t_done.add_toast(Toast::new(&err_msg));
                }
            }
        });
    });

    nav_view.push(&page);
}

pub fn push_help_page(nav_view: &NavigationView, lang: Language) {
    let toolbar_view = ToolbarView::new();
    let header_bar = HeaderBar::new();
    toolbar_view.add_top_bar(&header_bar);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .build();

    let clamp = Clamp::builder()
        .maximum_size(540)
        .tightening_threshold(400)
        .build();

    let content_box = Box::new(Orientation::Vertical, 14);
    content_box.set_margin_top(16);
    content_box.set_margin_bottom(18);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);

    let t = lang.texts();
    for (title, body) in t.help.sections {
        let group = PreferencesGroup::builder()
            .title(title)
            .description(body)
            .build();
        content_box.append(&group);
    }

    clamp.set_child(Some(&content_box));
    scroll.set_child(Some(&clamp));
    toolbar_view.set_content(Some(&scroll));

    let page = NavigationPage::builder()
        .title(t.help.title)
        .child(&toolbar_view)
        .build();

    nav_view.push(&page);
}

fn is_systemd_service_enabled() -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "turnike-daemon.service"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim() == "enabled")
        .unwrap_or(false)
}

fn set_systemd_service_enabled(enabled: bool) {
    let action = if enabled { "enable" } else { "disable" };
    let _ = std::process::Command::new("systemctl")
        .args(["--user", action, "--now", "turnike-daemon.service"])
        .spawn();
}

pub fn show_preferences_dialog(
    parent: &impl IsA<gtk4::Widget>,
    lang_cell: Rc<Cell<Language>>,
    on_lang_changed: Rc<dyn Fn(Language)>,
) {
    let current_lang = lang_cell.get();
    let t = current_lang.texts();
    let daemon_cfg = Rc::new(RefCell::new(load_config()));

    let dialog = PreferencesDialog::builder()
        .title(t.preferences.title)
        .build();

    let page = PreferencesPage::new();

    let appearance_group = PreferencesGroup::new();

    let sm = StyleManager::default();
    let current_scheme_idx = match sm.color_scheme() {
        ColorScheme::ForceLight | ColorScheme::PreferLight => 1,
        ColorScheme::ForceDark => 2,
        _ => 0,
    };

    let scheme_model = StringList::new(&t.preferences.appearance_options);
    let scheme_row = ComboRow::builder()
        .title(t.preferences.appearance_label)
        .model(&scheme_model)
        .selected(current_scheme_idx)
        .build();

    scheme_row.connect_selected_notify(|row| {
        let sm = StyleManager::default();
        match row.selected() {
            1 => sm.set_color_scheme(ColorScheme::ForceLight),
            2 => sm.set_color_scheme(ColorScheme::ForceDark),
            _ => sm.set_color_scheme(ColorScheme::Default),
        }
        let mut gc = load_gui_config();
        gc.color_scheme = row.selected();
        let _ = save_gui_config(&gc);
    });
    appearance_group.add(&scheme_row);

    let lang_model = StringList::new(&["Türkçe", "English"]);
    let lang_row = ComboRow::builder()
        .title(t.preferences.language_label)
        .model(&lang_model)
        .selected(current_lang.index())
        .build();

    appearance_group.add(&lang_row);
    page.add(&appearance_group);

    let t = current_lang.texts();

    let build_switch_row = |text: &crate::i18n::RowText, active: bool| -> SwitchRow {
        let row = SwitchRow::builder()
            .title(text.title)
            .active(active)
            .build();
        if !text.subtitle.is_empty() {
            row.set_subtitle(text.subtitle);
        }
        row
    };

    let service_group = PreferencesGroup::new();
    service_group.set_title(t.preferences.service_group);

    let autostart_row = build_switch_row(&t.preferences.autostart, is_systemd_service_enabled());

    autostart_row.connect_active_notify(|row| {
        set_systemd_service_enabled(row.is_active());
    });
    service_group.add(&autostart_row);

    let auto_login_row =
        build_switch_row(&t.preferences.auto_login, daemon_cfg.borrow().auto_login);

    let cfg_login = daemon_cfg.clone();
    auto_login_row.connect_active_notify(move |row| {
        let mut cfg = cfg_login.borrow_mut();
        cfg.auto_login = row.is_active();
        let _ = save_config(&cfg);
    });
    service_group.add(&auto_login_row);

    let auto_preempt_row = build_switch_row(
        &t.preferences.auto_preempt,
        daemon_cfg.borrow().auto_preempt,
    );

    let cfg_preempt = daemon_cfg.clone();
    auto_preempt_row.connect_active_notify(move |row| {
        let mut cfg = cfg_preempt.borrow_mut();
        cfg.auto_preempt = row.is_active();
        let _ = save_config(&cfg);
    });
    service_group.add(&auto_preempt_row);

    let auto_metered_row = build_switch_row(
        &t.preferences.auto_metered,
        daemon_cfg.borrow().auto_metered,
    );

    let cfg_metered = daemon_cfg.clone();
    auto_metered_row.connect_active_notify(move |row| {
        let mut cfg = cfg_metered.borrow_mut();
        cfg.auto_metered = row.is_active();
        let _ = save_config(&cfg);
    });
    service_group.add(&auto_metered_row);

    let screen_lock_row = build_switch_row(
        &t.preferences.auto_logout_on_screen_lock,
        daemon_cfg.borrow().auto_logout_on_screen_lock,
    );

    let cfg_screen_lock = daemon_cfg.clone();
    screen_lock_row.connect_active_notify(move |row| {
        let mut cfg = cfg_screen_lock.borrow_mut();
        cfg.auto_logout_on_screen_lock = row.is_active();
        let _ = save_config(&cfg);
    });
    service_group.add(&screen_lock_row);

    let radar_row = build_switch_row(&t.preferences.radar, daemon_cfg.borrow().radar.enabled);

    let cfg_radar = daemon_cfg.clone();
    radar_row.connect_active_notify(move |row| {
        let mut cfg = cfg_radar.borrow_mut();
        cfg.radar.enabled = row.is_active();
        let _ = save_config(&cfg);
    });
    service_group.add(&radar_row);

    page.add(&service_group);

    let quota_group = PreferencesGroup::new();
    quota_group.set_title(t.preferences.quota_group);

    let is_notify_enabled = daemon_cfg.borrow().notify_on_quota;
    let initial_mode = daemon_cfg.borrow().quota_threshold_mode;
    let initial_val = daemon_cfg.borrow().quota_threshold_value;

    let notify_quota_row = build_switch_row(&t.preferences.notify_quota, is_notify_enabled);

    let mode_model = StringList::new(&t.preferences.threshold_mode_options);
    let mode_row = ComboRow::builder()
        .title(t.preferences.threshold_mode_label)
        .model(&mode_model)
        .selected(match initial_mode {
            QuotaThresholdMode::Percentage => 0,
            QuotaThresholdMode::Gigabytes => 1,
        })
        .sensitive(is_notify_enabled)
        .build();

    let scale_row = ActionRow::builder()
        .title(t.preferences.threshold_value_label)
        .sensitive(is_notify_enabled)
        .build();

    let (min_val, max_val, step_val) = match initial_mode {
        QuotaThresholdMode::Percentage => (5.0, 50.0, 5.0),
        QuotaThresholdMode::Gigabytes => (1.0, 10.0, 0.5),
    };

    let scale = Scale::with_range(Orientation::Horizontal, min_val, max_val, step_val);
    scale.set_size_request(160, -1);
    scale.set_draw_value(false);
    scale.set_value(initial_val);
    scale.set_valign(Align::Center);

    let val_label = Label::new(Some(&format_threshold_label(
        initial_mode,
        initial_val,
        current_lang.is_tr(),
    )));
    val_label.add_css_class("numeric");
    val_label.set_valign(Align::Center);
    val_label.set_size_request(60, -1);

    scale_row.add_suffix(&scale);
    scale_row.add_suffix(&val_label);

    quota_group.add(&notify_quota_row);
    quota_group.add(&mode_row);
    quota_group.add(&scale_row);
    page.add(&quota_group);

    let cfg_notify = daemon_cfg.clone();
    let mode_row_notify = mode_row.clone();
    let scale_row_notify = scale_row.clone();
    notify_quota_row.connect_active_notify(move |row| {
        let is_active = row.is_active();
        mode_row_notify.set_sensitive(is_active);
        scale_row_notify.set_sensitive(is_active);
        let mut cfg = cfg_notify.borrow_mut();
        cfg.notify_on_quota = is_active;
        let _ = save_config(&cfg);
    });

    let cfg_mode = daemon_cfg.clone();
    let scale_mode = scale.clone();
    let val_label_mode = val_label.clone();
    let lang_for_mode = lang_cell.clone();
    mode_row.connect_selected_notify(move |row| {
        let new_mode = if row.selected() == 0 {
            QuotaThresholdMode::Percentage
        } else {
            QuotaThresholdMode::Gigabytes
        };

        let (new_min, new_max, new_step, default_val) = match new_mode {
            QuotaThresholdMode::Percentage => (5.0, 50.0, 5.0, 15.0),
            QuotaThresholdMode::Gigabytes => (1.0, 10.0, 0.5, 3.0),
        };

        scale_mode.set_range(new_min, new_max);
        scale_mode.set_increments(new_step, new_step * 2.0);
        scale_mode.set_value(default_val);
        val_label_mode.set_text(&format_threshold_label(
            new_mode,
            default_val,
            lang_for_mode.get().is_tr(),
        ));

        let mut cfg = cfg_mode.borrow_mut();
        cfg.quota_threshold_mode = new_mode;
        cfg.quota_threshold_value = default_val;
        let _ = save_config(&cfg);
    });

    let cfg_scale = daemon_cfg.clone();
    let mode_row_scale = mode_row.clone();
    let val_label_scale = val_label.clone();
    let lang_for_scale = lang_cell.clone();
    scale.connect_value_changed(move |s| {
        let mode = if mode_row_scale.selected() == 0 {
            QuotaThresholdMode::Percentage
        } else {
            QuotaThresholdMode::Gigabytes
        };
        let val = s.value();
        val_label_scale.set_text(&format_threshold_label(
            mode,
            val,
            lang_for_scale.get().is_tr(),
        ));

        let mut cfg = cfg_scale.borrow_mut();
        cfg.quota_threshold_value = val;
        let _ = save_config(&cfg);
    });

    let dialog_weak = dialog.downgrade();
    let scheme_row_weak = scheme_row.downgrade();
    let service_group_weak = service_group.downgrade();
    let autostart_row_weak = autostart_row.downgrade();
    let auto_login_row_weak = auto_login_row.downgrade();
    let auto_preempt_row_weak = auto_preempt_row.downgrade();
    let auto_metered_row_weak = auto_metered_row.downgrade();
    let screen_lock_row_weak = screen_lock_row.downgrade();
    let radar_row_weak = radar_row.downgrade();
    let quota_group_weak = quota_group.downgrade();
    let notify_quota_row_weak = notify_quota_row.downgrade();
    let mode_row_weak = mode_row.downgrade();
    let scale_row_weak = scale_row.downgrade();
    let scale_weak = scale.downgrade();
    let val_label_weak = val_label.downgrade();

    lang_row.connect_selected_notify(move |row| {
        let next_lang = Language::from_index(row.selected());
        if next_lang == lang_cell.get() {
            return;
        }
        lang_cell.set(next_lang);
        on_lang_changed(next_lang);

        let mut gc = load_gui_config();
        gc.language = Some(next_lang.code().to_string());
        let _ = save_gui_config(&gc);

        let t = next_lang.texts();

        if let Some(d) = dialog_weak.upgrade() {
            d.set_title(t.preferences.title);
        }
        row.set_title(t.preferences.language_label);
        if let Some(sr) = scheme_row_weak.upgrade() {
            let prev_sel = sr.selected();
            sr.set_title(t.preferences.appearance_label);
            sr.set_model(Some(&StringList::new(&t.preferences.appearance_options)));
            sr.set_selected(prev_sel);
        }
        if let Some(sg) = service_group_weak.upgrade() {
            sg.set_title(t.preferences.service_group);
        }
        if let Some(ar) = autostart_row_weak.upgrade() {
            ar.apply_text(&t.preferences.autostart);
        }
        if let Some(al) = auto_login_row_weak.upgrade() {
            al.apply_text(&t.preferences.auto_login);
        }
        if let Some(ap) = auto_preempt_row_weak.upgrade() {
            ap.apply_text(&t.preferences.auto_preempt);
        }
        if let Some(am) = auto_metered_row_weak.upgrade() {
            am.apply_text(&t.preferences.auto_metered);
        }
        if let Some(sl) = screen_lock_row_weak.upgrade() {
            sl.apply_text(&t.preferences.auto_logout_on_screen_lock);
        }
        if let Some(rr) = radar_row_weak.upgrade() {
            rr.apply_text(&t.preferences.radar);
        }
        if let Some(qg) = quota_group_weak.upgrade() {
            qg.set_title(t.preferences.quota_group);
        }
        if let Some(nq) = notify_quota_row_weak.upgrade() {
            nq.apply_text(&t.preferences.notify_quota);
        }
        if let Some(mr) = mode_row_weak.upgrade() {
            let prev_sel = mr.selected();
            mr.set_title(t.preferences.threshold_mode_label);
            mr.set_model(Some(&StringList::new(
                &t.preferences.threshold_mode_options,
            )));
            mr.set_selected(prev_sel);
        }
        if let Some(sr) = scale_row_weak.upgrade() {
            sr.set_title(t.preferences.threshold_value_label);
        }
        if let Some(vl) = val_label_weak.upgrade() {
            let mode = if let Some(mr) = mode_row_weak.upgrade() {
                if mr.selected() == 0 {
                    QuotaThresholdMode::Percentage
                } else {
                    QuotaThresholdMode::Gigabytes
                }
            } else {
                QuotaThresholdMode::Percentage
            };
            let val = if let Some(sc) = scale_weak.upgrade() {
                sc.value()
            } else {
                15.0
            };
            vl.set_text(&format_threshold_label(mode, val, next_lang.is_tr()));
        }
    });

    dialog.add(&page);
    dialog.present(Some(parent));
}

pub fn show_shortcuts_dialog(parent: &impl IsA<gtk4::Widget>, lang: Language) {
    let t = lang.texts();
    let shortcuts_win = ShortcutsWindow::builder().modal(true).build();

    let section = ShortcutsSection::builder().build();
    let group = ShortcutsGroup::builder().title(t.app.shortcuts).build();

    for (label_text, accel_text) in t.help.shortcuts_list {
        let shortcut = ShortcutsShortcut::builder()
            .title(label_text)
            .accelerator(accel_text)
            .build();
        group.append(&shortcut);
    }

    section.append(&group);
    shortcuts_win.set_child(Some(&section));

    if let Some(w) = parent
        .root()
        .and_then(|r| r.downcast::<gtk4::Window>().ok())
    {
        shortcuts_win.set_transient_for(Some(&w));
    }
    shortcuts_win.present();
}
