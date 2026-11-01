use crate::{
    config::Config,
    services::{
        applications::{self, Application},
        system,
    },
};
use gtk::{glib, prelude::*};
use std::{cell::RefCell, rc::Rc};

pub fn build(
    apps: &[Application],
    config: Rc<RefCell<Config>>,
    parent: &gtk::Window,
    on_pins: Rc<dyn Fn()>,
) -> gtk::Popover {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_size_request(560, -1);
    content.append(&super::label("MONOLITO", "brand"));
    let search = gtk::SearchEntry::builder()
        .placeholder_text(gettextrs::gettext("Search installed applications"))
        .tooltip_text(gettextrs::gettext("Search installed applications"))
        .build();
    content.append(&search);
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = super::label("Pinned", "heading");
    title.set_hexpand(true);
    heading.append(&title);
    let all = gtk::ToggleButton::with_label(&gettextrs::gettext("All apps"));
    heading.append(&all);
    content.append(&heading);

    let results = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(5)
        .min_children_per_line(5)
        .homogeneous(true)
        .row_spacing(8)
        .column_spacing(4)
        .valign(gtk::Align::Start)
        .build();
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(220)
        .max_content_height(340)
        .propagate_natural_height(true)
        .child(&results)
        .build();
    content.append(&scroll);
    let empty = super::label(
        "No pinned applications. Choose All apps to pin one.",
        "muted",
    );
    empty.set_wrap(true);
    content.append(&empty);
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    footer.add_css_class("popup-footer");
    let user = glib::user_name();
    let user_label = gtk::Label::new(Some(&user.to_string_lossy()));
    user_label.set_hexpand(true);
    user_label.set_xalign(0.0);
    footer.append(&gtk::Image::from_icon_name("avatar-default-symbolic"));
    footer.append(&user_label);
    let settings = gtk::Button::from_icon_name("emblem-system-symbolic");
    settings.set_tooltip_text(Some("Monolito Settings"));
    let window = parent.clone();
    settings.connect_clicked(move |_| system::open_settings(&window));
    footer.append(&settings);
    let power = gtk::Button::from_icon_name("system-shutdown-symbolic");
    power.set_sensitive(false);
    power.set_tooltip_text(Some(&gettextrs::gettext(
        "Session actions are planned; use the XFCE session menu.",
    )));
    footer.append(&power);
    content.append(&footer);
    let popup = super::popover(&content);
    popup.set_halign(gtk::Align::Start);
    let search_apps = apps.to_vec();
    let search_parent = parent.clone();
    let search_popup = popup.downgrade();
    search.connect_activate(move |entry| {
        let query = entry.text().to_lowercase();
        if !query.is_empty() {
            if let Some(app) = search_apps
                .iter()
                .find(|a| a.name.to_lowercase().contains(&query))
            {
                if let Some(popup) = search_popup.upgrade() {
                    popup.popdown();
                }
                applications::launch(&app.id, &search_parent);
            }
        }
    });
    let apps = apps.to_vec();
    let refresh: Rc<dyn Fn()> = {
        let (results, search, all, title, empty, parent, popup) = (
            results.clone(),
            search.clone(),
            all.clone(),
            title.clone(),
            empty.clone(),
            parent.clone(),
            popup.clone(),
        );
        Rc::new(move || {
            while let Some(child) = results.first_child() {
                results.remove(&child);
            }
            let query = search.text().to_lowercase();
            let show_all = all.is_active() || !query.is_empty();
            title.set_text(&gettextrs::gettext(if show_all {
                "All apps"
            } else {
                "Pinned"
            }));
            let config_read = config.borrow();
            let mut shown: Vec<_> = apps
                .iter()
                .filter(|a| {
                    (show_all || config_read.pinned.contains(&a.id))
                        && a.name.to_lowercase().contains(&query)
                })
                .collect();
            if !show_all {
                shown.sort_by_key(|a| config_read.pinned.iter().position(|id| id == &a.id));
            }
            empty.set_visible(shown.is_empty());
            if show_all {
                empty.set_text(&gettextrs::gettext("No applications found."));
            } else {
                empty.set_text(&gettextrs::gettext(
                    "No pinned applications. Choose All apps to pin one.",
                ));
            }
            for app in shown {
                let cell = gtk::Box::new(gtk::Orientation::Vertical, 2);
                let launch = gtk::Button::new();
                launch.add_css_class("app-tile");
                let inner = gtk::Box::new(gtk::Orientation::Vertical, 8);
                inner.append(&app.image(36));
                let name = gtk::Label::new(Some(&app.name));
                name.set_ellipsize(gtk::pango::EllipsizeMode::End);
                name.set_max_width_chars(12);
                inner.append(&name);
                launch.set_child(Some(&inner));
                launch.set_tooltip_text(Some(&app.name));
                let (id, launch_parent, launch_popup) =
                    (app.id.clone(), parent.clone(), popup.clone());
                launch.connect_clicked(move |_| {
                    launch_popup.popdown();
                    applications::launch(&id, &launch_parent);
                });
                cell.append(&launch);
                let pin = gtk::Button::with_label(&gettextrs::gettext(
                    if config_read.pinned.contains(&app.id) {
                        "Unpin"
                    } else {
                        "Pin"
                    },
                ));
                pin.add_css_class("pin-action");
                let (id, config, parent, on_pins, popup) = (
                    app.id.clone(),
                    config.clone(),
                    parent.clone(),
                    on_pins.clone(),
                    popup.clone(),
                );
                pin.connect_clicked(move |_| {
                    let old = config.borrow().clone();
                    config.borrow_mut().toggle_pin(&id);
                    let result = config.borrow().save();
                    if let Err(e) = result {
                        *config.borrow_mut() = old;
                        system::show_error(&parent, &e.to_string());
                    } else {
                        on_pins();
                        popup.popdown();
                    }
                });
                cell.append(&pin);
                results.insert(&cell, -1);
            }
        })
    };
    let f = refresh.clone();
    search.connect_search_changed(move |_| f());
    let f = refresh.clone();
    all.connect_toggled(move |_| f());
    let f = refresh.clone();
    let search_focus = search.clone();
    popup.connect_map(move |_| {
        f();
        search_focus.grab_focus();
    });
    refresh();
    popup
}
