use crate::i18n::Language;
use adw::prelude::*;
use adw::{
    ActionRow, Clamp, HeaderBar, NavigationPage, NavigationView, PasswordEntryRow,
    PreferencesGroup, Toast, ToastOverlay, ToolbarView,
};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Orientation, PolicyType, ScrolledWindow};
use libadwaita as adw;
use std::rc::Rc;
use turnike_client::PortalClient;
use turnike_secret::SecretStore;
use turnike_types::{Credentials, check_password_rules, validate_password};

pub type RuleUpdater = Rc<dyn Fn(&str, Option<&str>)>;

pub fn build_live_rules_group(lang: Language) -> (PreferencesGroup, RuleUpdater) {
    let t = lang.texts();
    let rules_group = PreferencesGroup::builder()
        .title(t.password.rules_title)
        .build();

    let rule_texts = [
        t.password.rule_length,
        t.password.rule_chars,
        t.password.rule_forbidden_chars,
        t.password.rule_forbidden_letters,
        t.password.rule_no_username,
    ];

    let mut icons = Vec::with_capacity(5);

    for text in rule_texts {
        let row = ActionRow::builder().title(text).build();
        let icon = Image::from_icon_name("checkbox-symbolic");
        icon.add_css_class("dim-label");
        row.add_prefix(&icon);
        rules_group.add(&row);
        icons.push(icon);
    }

    let update_rules: RuleUpdater = Rc::new(move |password: &str, username: Option<&str>| {
        let status = check_password_rules(password, username);
        let checks = [
            status.length_ok,
            status.chars_ok,
            status.no_forbidden_chars,
            status.no_forbidden_letters,
            status.no_username,
        ];

        for (i, passed) in checks.into_iter().enumerate() {
            let icon = &icons[i];
            if password.is_empty() {
                icon.set_icon_name(Some("checkbox-symbolic"));
                icon.remove_css_class("success");
                icon.remove_css_class("error");
                icon.add_css_class("dim-label");
            } else if passed {
                icon.set_icon_name(Some("object-select-symbolic"));
                icon.remove_css_class("dim-label");
                icon.remove_css_class("error");
                icon.add_css_class("success");
            } else {
                icon.set_icon_name(Some("window-close-symbolic"));
                icon.remove_css_class("dim-label");
                icon.remove_css_class("success");
                icon.add_css_class("error");
            }
        }
    });

    update_rules("", None);
    (rules_group, update_rules)
}

pub fn push_password_page(
    nav_view: &NavigationView,
    toast_overlay: &ToastOverlay,
    client: PortalClient,
    username: Option<String>,
    initial_current_password: Option<String>,
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

    let form_group = PreferencesGroup::new();

    let current_pass_row = PasswordEntryRow::builder()
        .title(t.password.current_label)
        .build();
    if let Some(ref cp) = initial_current_password.filter(|s| !s.is_empty()) {
        current_pass_row.set_text(cp);
    }
    form_group.add(&current_pass_row);

    let new_pass_row = PasswordEntryRow::builder()
        .title(t.password.new_label)
        .build();
    form_group.add(&new_pass_row);

    let confirm_pass_row = PasswordEntryRow::builder()
        .title(t.password.confirm_label)
        .build();
    form_group.add(&confirm_pass_row);

    let (rules_group, update_rules) = build_live_rules_group(lang);

    let u_for_live = username.clone();
    let updater = update_rules.clone();
    new_pass_row.connect_changed(move |entry| {
        updater(&entry.text(), u_for_live.as_deref());
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
        .title(t.password.change_title)
        .child(&toolbar_view)
        .build();

    let nav_clone = nav_view.clone();
    let toast_clone = toast_overlay.clone();
    let save_btn_clone = save_btn.clone();

    save_btn.connect_clicked(move |_| {
        let current_p = current_pass_row.text().trim().to_string();
        let new_p = new_pass_row.text().trim().to_string();
        let confirm_p = confirm_pass_row.text().trim().to_string();

        if current_p.is_empty() {
            current_pass_row.grab_focus();
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

        if let Err(violation) = validate_password(&new_p, username.as_deref()) {
            toast_clone.add_toast(Toast::new(&lang.violation_message(&violation)));
            return;
        }

        save_btn_clone.set_sensitive(false);

        let c = client.clone();
        let b_done = save_btn_clone.clone();
        let t_done = toast_clone.clone();
        let nav_done = nav_clone.clone();

        glib::spawn_future_local(async move {
            match c.change_password(&current_p, &new_p).await {
                Ok(_) => {
                    if let Ok(Some(existing)) = SecretStore::load_async().await {
                        let updated = Credentials::new(existing.username, new_p);
                        let _ = SecretStore::save_async(updated).await;
                    }
                    t_done.add_toast(Toast::new(t.password.updated_toast));
                    nav_done.pop();
                }
                Err(err) => {
                    b_done.set_sensitive(true);
                    let err_msg = lang.portal_error_message(&err);
                    t_done.add_toast(Toast::new(&err_msg));
                }
            }
        });
    });

    nav_view.push(&page);
}
