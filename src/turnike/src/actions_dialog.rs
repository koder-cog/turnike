use crate::i18n::Language;
use adw::prelude::*;
use adw::{
    ActionRow, Clamp, HeaderBar, NavigationPage, NavigationView, PreferencesGroup, StatusPage,
    ToolbarView, ViewStack,
};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Label, Orientation, PolicyType, ScrolledWindow, Spinner};
use libadwaita as adw;
use std::rc::Rc;
use turnike_client::PortalClient;

pub fn push_actions_page(nav_view: &NavigationView, client: PortalClient, lang: Language) {
    let t = lang.texts();
    let toolbar_view = ToolbarView::new();
    let header_bar = HeaderBar::new();

    let refresh_btn = Button::from_icon_name("view-refresh-symbolic");
    refresh_btn.set_tooltip_text(Some(t.actions.refresh_tooltip));
    header_bar.pack_end(&refresh_btn);
    toolbar_view.add_top_bar(&header_bar);

    let stack = ViewStack::new();

    let loading_box = Box::new(Orientation::Vertical, 16);
    loading_box.set_valign(Align::Center);
    loading_box.set_halign(Align::Center);
    loading_box.set_vexpand(true);

    let spinner = Spinner::new();
    spinner.set_size_request(36, 36);
    spinner.set_halign(Align::Center);
    spinner.start();

    let loading_title = Label::new(Some(t.actions.loading));
    loading_title.add_css_class("title-4");
    loading_title.set_halign(Align::Center);

    loading_box.append(&spinner);
    loading_box.append(&loading_title);
    stack.add_named(&loading_box, Some("loading"));

    let empty_page = StatusPage::builder()
        .icon_name("network-wireless-symbolic")
        .title(t.actions.no_records)
        .build();
    stack.add_named(&empty_page, Some("empty"));

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .build();

    let clamp = Clamp::builder()
        .maximum_size(540)
        .tightening_threshold(400)
        .build();

    let list_container = Box::new(Orientation::Vertical, 14);
    list_container.set_margin_top(16);
    list_container.set_margin_bottom(18);
    list_container.set_margin_start(16);
    list_container.set_margin_end(16);

    clamp.set_child(Some(&list_container));
    scroll.set_child(Some(&clamp));
    stack.add_named(&scroll, Some("content"));

    toolbar_view.set_content(Some(&stack));

    let page = NavigationPage::builder()
        .title(t.actions.title)
        .child(&toolbar_view)
        .build();

    let load_data: Rc<dyn Fn()> = {
        let stack = stack.clone();
        let empty_page = empty_page.clone();
        let list_container = list_container.clone();
        let refresh_btn = refresh_btn.clone();

        Rc::new(move || {
            stack.set_visible_child_name("loading");
            refresh_btn.set_sensitive(false);

            while let Some(child) = list_container.first_child() {
                list_container.remove(&child);
            }

            let c = client.clone();
            let st = stack.clone();
            let ep = empty_page.clone();
            let lc = list_container.clone();
            let rb = refresh_btn.clone();

            glib::spawn_future_local(async move {
                let t = lang.texts();
                match c.get_last_actions().await {
                    Ok(actions) => {
                        rb.set_sensitive(true);
                        if actions.is_empty() {
                            ep.set_title(t.actions.no_records);
                            ep.set_description(None);
                            st.set_visible_child_name("empty");
                            return;
                        }

                        let total_down: f64 = actions.iter().map(|a| a.download_mb).sum();
                        let total_up: f64 = actions.iter().map(|a| a.upload_mb).sum();

                        let summary_group = PreferencesGroup::builder()
                            .title(t.actions.summary_title)
                            .build();

                        let summary_row = ActionRow::builder()
                            .title(format!(
                                "↓ {:.1} MB {}  •  ↑ {:.1} MB {}",
                                total_down, t.actions.download, total_up, t.actions.upload
                            ))
                            .subtitle(lang.format_recent_sessions(actions.len()))
                            .build();
                        summary_group.add(&summary_row);
                        lc.append(&summary_group);

                        let records_group =
                            PreferencesGroup::builder().title(t.actions.title).build();

                        for item in actions {
                            let safe_dorm = glib::markup_escape_text(&item.dormitory_name);
                            let safe_time = glib::markup_escape_text(&item.check_in_time);
                            let row = ActionRow::builder()
                                .title(safe_dorm.as_str())
                                .subtitle(format!(
                                    "{}  •  ↓ {:.1} MB {}  ↑ {:.1} MB {}",
                                    safe_time,
                                    item.download_mb,
                                    t.actions.download,
                                    item.upload_mb,
                                    t.actions.upload
                                ))
                                .build();

                            let dur_label =
                                Label::new(Some(&item.formatted_duration(lang.is_tr())));
                            dur_label.add_css_class("dim-label");
                            dur_label.add_css_class("numeric");
                            dur_label.add_css_class("caption");
                            dur_label.set_valign(Align::Center);
                            row.add_suffix(&dur_label);

                            records_group.add(&row);
                        }

                        lc.append(&records_group);
                        st.set_visible_child_name("content");
                    }
                    Err(e) => {
                        rb.set_sensitive(true);
                        ep.set_title(t.status.no_connection);
                        let err_msg = lang.portal_error_message(&e);
                        ep.set_description(Some(&err_msg));
                        st.set_visible_child_name("empty");
                    }
                }
            });
        })
    };

    let load_click = load_data.clone();
    refresh_btn.connect_clicked(move |_| {
        load_click();
    });

    load_data();
    nav_view.push(&page);
}
