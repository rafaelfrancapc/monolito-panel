use crate::{
    config::Config,
    services::{
        applications::{self, Application},
        recent, session, system,
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
    content.set_size_request(monolito_ui::START_WIDTH, 540);
    content.add_css_class("start-menu-content");

    // Search bar at the top (Windows 11 style)
    let search = gtk::SearchEntry::builder()
        .placeholder_text(gettextrs::gettext("Type here to search"))
        .tooltip_text(gettextrs::gettext(
            "Search installed applications and files",
        ))
        .build();
    search.add_css_class("start-search-entry");
    content.append(&search);

    // Section header: Pinned & All apps toggle
    let pinned_header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = super::label("Pinned", "heading");
    title.set_hexpand(true);
    pinned_header.append(&title);
    let all = gtk::ToggleButton::with_label(&gettextrs::gettext("All apps"));
    all.add_css_class("all-apps-toggle");
    pinned_header.append(&all);
    content.append(&pinned_header);

    // Grid of pinned / search results
    let results = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(6)
        .min_children_per_line(4)
        .homogeneous(true)
        .row_spacing(6)
        .column_spacing(4)
        .valign(gtk::Align::Start)
        .build();

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(190)
        .max_content_height(280)
        .propagate_natural_height(true)
        .vexpand(true)
        .child(&results)
        .build();
    content.append(&scroll);

    let empty = super::label(
        "No pinned applications. Choose All apps to pin one.",
        "muted",
    );
    empty.set_wrap(true);
    content.append(&empty);

    // Recommended / Recent section
    let rec_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    rec_section.add_css_class("recommended-section");
    let rec_header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let rec_title = super::label("Recommended", "heading");
    rec_title.set_hexpand(true);
    rec_header.append(&rec_title);
    rec_section.append(&rec_header);

    let rec_grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(6)
        .column_homogeneous(true)
        .build();

    let empty_rec = super::label("No recent files.", "muted");
    rec_section.append(&rec_grid);
    rec_section.append(&empty_rec);
    content.append(&rec_section);

    // Footer with user profile & functional power options
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    footer.add_css_class("popup-footer");

    let user_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    user_box.set_hexpand(true);
    let user_button = gtk::MenuButton::new();
    let user_inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let profile = crate::services::session::profile();
    let avatar = if let Some(path) = profile.photo {
        gtk::Image::from_file(path)
    } else {
        gtk::Image::from_icon_name("avatar-default-symbolic")
    };
    avatar.set_pixel_size(24);
    user_inner.append(&avatar);
    let user_label = gtk::Label::new(Some(&profile.name));
    user_label.set_xalign(0.0);
    user_inner.append(&user_label);
    user_button.set_child(Some(&user_inner));
    let logout = gtk::Button::with_label(&gettextrs::gettext("Log out"));
    let logout_menu = super::popover(&logout);
    let logout_parent = parent.clone();
    logout.connect_clicked(move |_| {
        let parent = logout_parent.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Err(error) = session::log_out().await {
                system::show_error(&parent, &error);
            }
        });
    });
    user_button.set_popover(Some(&logout_menu));
    user_box.append(&user_button);
    footer.append(&user_box);

    let settings = gtk::Button::from_icon_name("emblem-system-symbolic");
    settings.set_tooltip_text(Some(&gettextrs::gettext("System settings")));
    let window_for_settings = parent.clone();
    settings.connect_clicked(move |_| system::open_settings(&window_for_settings));
    footer.append(&settings);

    // Power menu button
    let power_btn = gtk::MenuButton::builder()
        .icon_name("system-shutdown-symbolic")
        .tooltip_text(gettextrs::gettext("Power"))
        .build();
    power_btn.add_css_class("power-button");

    let power_menu_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    power_menu_box.set_margin_top(4);
    power_menu_box.set_margin_bottom(4);
    power_menu_box.set_margin_start(4);
    power_menu_box.set_margin_end(4);

    let power_popover = gtk::Popover::builder()
        .child(&power_menu_box)
        .has_arrow(false)
        .build();
    power_popover.add_css_class("monolito-power-menu");

    // Sleep
    let sleep_btn = gtk::Button::with_label(&gettextrs::gettext("Sleep"));
    let sleep_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    sleep_box.append(&gtk::Image::from_icon_name("system-suspend-symbolic"));
    sleep_box.append(&gtk::Label::new(Some(&gettextrs::gettext("Sleep"))));
    sleep_btn.set_child(Some(&sleep_box));
    sleep_btn.add_css_class("menu-item-button");
    let (pop_ref, win_ref) = (power_popover.clone(), parent.clone());
    sleep_btn.connect_clicked(move |_| {
        pop_ref.popdown();
        let win = win_ref.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = session::suspend().await {
                system::show_error(&win, &e);
            }
        });
    });
    power_menu_box.append(&sleep_btn);

    // Restart
    let restart_btn = gtk::Button::with_label(&gettextrs::gettext("Restart"));
    let restart_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    restart_box.append(&gtk::Image::from_icon_name("system-reboot-symbolic"));
    restart_box.append(&gtk::Label::new(Some(&gettextrs::gettext("Restart"))));
    restart_btn.set_child(Some(&restart_box));
    restart_btn.add_css_class("menu-item-button");
    let (pop_ref, win_ref) = (power_popover.clone(), parent.clone());
    restart_btn.connect_clicked(move |_| {
        pop_ref.popdown();
        let win = win_ref.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = session::reboot().await {
                system::show_error(&win, &e);
            }
        });
    });
    power_menu_box.append(&restart_btn);

    // Shut down
    let shutdown_btn = gtk::Button::with_label(&gettextrs::gettext("Shut down"));
    let shutdown_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    shutdown_box.append(&gtk::Image::from_icon_name("system-shutdown-symbolic"));
    shutdown_box.append(&gtk::Label::new(Some(&gettextrs::gettext("Shut down"))));
    shutdown_btn.set_child(Some(&shutdown_box));
    shutdown_btn.add_css_class("menu-item-button");
    let (pop_ref, win_ref) = (power_popover.clone(), parent.clone());
    shutdown_btn.connect_clicked(move |_| {
        pop_ref.popdown();
        let win = win_ref.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = session::power_off().await {
                system::show_error(&win, &e);
            }
        });
    });
    power_menu_box.append(&shutdown_btn);

    // Lock screen
    let lock_btn = gtk::Button::with_label(&gettextrs::gettext("Lock screen"));
    let lock_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    lock_box.append(&gtk::Image::from_icon_name("system-lock-screen-symbolic"));
    lock_box.append(&gtk::Label::new(Some(&gettextrs::gettext("Lock screen"))));
    lock_btn.set_child(Some(&lock_box));
    lock_btn.add_css_class("menu-item-button");
    let (pop_ref, win_ref) = (power_popover.clone(), parent.clone());
    lock_btn.connect_clicked(move |_| {
        pop_ref.popdown();
        let win = win_ref.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = session::lock_screen().await {
                system::show_error(&win, &e);
            }
        });
    });
    power_menu_box.append(&lock_btn);

    power_btn.set_popover(Some(&power_popover));
    footer.append(&power_btn);
    content.append(&footer);

    let popup = super::popover(&content);
    popup.set_halign(gtk::Align::Center);
    let super_keys = gtk::EventControllerKey::new();
    super_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak_popup = popup.downgrade();
    super_keys.connect_key_released(move |_, key, _, _| {
        if matches!(key, gtk::gdk::Key::Super_L | gtk::gdk::Key::Super_R) {
            if let Some(popup) = weak_popup.upgrade() {
                popup.popdown();
            }
        }
    });
    popup.add_controller(super_keys);

    // Search activation
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
        let (results, search, all, title, empty, rec_section, rec_grid, empty_rec, parent, popup) = (
            results.clone(),
            search.clone(),
            all.clone(),
            title.clone(),
            empty.clone(),
            rec_section.clone(),
            rec_grid.clone(),
            empty_rec.clone(),
            parent.clone(),
            popup.clone(),
        );
        Rc::new(move || {
            while let Some(child) = results.first_child() {
                results.remove(&child);
            }
            while let Some(child) = rec_grid.first_child() {
                rec_grid.remove(&child);
            }

            let query = search.text().to_lowercase();
            let show_all = all.is_active() || !query.is_empty();

            // When searching or showing all apps, hide recommended section to dedicate space to apps
            rec_section.set_visible(!show_all);

            title.set_text(&gettextrs::gettext(if show_all {
                "All apps"
            } else {
                "Pinned"
            }));

            let config_read = config.borrow();
            let mut shown: Vec<_> = apps
                .iter()
                .filter(|a| {
                    (show_all || config_read.start_pinned.contains(&a.id))
                        && a.name.to_lowercase().contains(&query)
                })
                .collect();

            if !show_all {
                shown.sort_by_key(|a| config_read.start_pinned.iter().position(|id| id == &a.id));
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

                let inner = gtk::Box::new(gtk::Orientation::Vertical, 6);
                inner.set_halign(gtk::Align::Center);
                inner.append(&app.image(32));

                let name = gtk::Label::new(Some(&app.name));
                name.set_ellipsize(gtk::pango::EllipsizeMode::End);
                name.set_max_width_chars(11);
                name.set_justify(gtk::Justification::Center);
                name.add_css_class("app-tile-name");
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

                let action_list = gtk::Box::new(gtk::Orientation::Vertical, 4);
                for (in_start, label) in [
                    (
                        true,
                        if config_read.start_pinned.contains(&app.id) {
                            "Unpin from Start"
                        } else {
                            "Pin to Start"
                        },
                    ),
                    (
                        false,
                        if config_read.pinned.contains(&app.id) {
                            "Unpin from taskbar"
                        } else {
                            "Pin to taskbar"
                        },
                    ),
                ] {
                    let pin = gtk::Button::with_label(&gettextrs::gettext(label));
                    pin.add_css_class("menu-item-button");
                    let (id, config, parent, on_pins, popup) = (
                        app.id.clone(),
                        config.clone(),
                        parent.clone(),
                        on_pins.clone(),
                        popup.clone(),
                    );
                    pin.connect_clicked(move |_| {
                        let old = config.borrow().clone();
                        if in_start {
                            config.borrow_mut().toggle_start_pin(&id);
                        } else {
                            config.borrow_mut().toggle_pin(&id);
                        }
                        let result = config.borrow().save();
                        if let Err(e) = result {
                            *config.borrow_mut() = old;
                            system::show_error(&parent, &e.to_string());
                        } else {
                            on_pins();
                            popup.popdown();
                        }
                    });
                    action_list.append(&pin);
                }
                let shortcut =
                    gtk::Button::with_label(&gettextrs::gettext("Create desktop shortcut"));
                shortcut.add_css_class("menu-item-button");
                let (id, parent) = (app.id.clone(), parent.clone());
                shortcut.connect_clicked(move |_| {
                    if let Err(e) = applications::create_desktop_shortcut(&id) {
                        system::show_error(&parent, &e);
                    }
                });
                action_list.append(&shortcut);
                let actions = super::popover(&action_list);
                actions.set_parent(&launch);
                let gesture = gtk::GestureClick::new();
                gesture.set_button(3);
                let menu = actions.clone();
                gesture.connect_pressed(move |_, _, _, _| menu.popup());
                launch.add_controller(gesture);
                let keys = gtk::EventControllerKey::new();
                let menu = actions.clone();
                keys.connect_key_pressed(move |_, key, _, modifiers| {
                    if key == gtk::gdk::Key::Menu
                        || (key == gtk::gdk::Key::F10
                            && modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK))
                    {
                        menu.popup();
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                });
                launch.add_controller(keys);
                launch.connect_unrealize(move |_| actions.unparent());
                results.insert(&cell, -1);
            }

            // Populate Recommended section (recent files)
            if !show_all {
                let recent_items = recent::get_recent_files(6);
                empty_rec.set_visible(recent_items.is_empty());
                for (idx, item) in recent_items.into_iter().enumerate() {
                    let row = (idx / 2) as i32;
                    let col = (idx % 2) as i32;

                    let btn = gtk::Button::new();
                    btn.add_css_class("recent-item-tile");

                    let item_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    let icon = crate::services::icons::image(
                        &item.icon_name,
                        &["text-x-generic-symbolic"],
                        24,
                    );
                    item_box.append(&icon);

                    let text_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
                    let label = gtk::Label::new(Some(&item.name));
                    label.set_xalign(0.0);
                    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    label.set_max_width_chars(20);
                    text_box.append(&label);

                    let sub = gtk::Label::new(Some(&gettextrs::gettext("Recent")));
                    sub.set_xalign(0.0);
                    sub.add_css_class("muted");
                    sub.add_css_class("recent-subtext");
                    text_box.append(&sub);

                    item_box.append(&text_box);
                    btn.set_child(Some(&item_box));
                    btn.set_tooltip_text(Some(&item.uri));

                    let (uri, item_parent, item_popup) =
                        (item.uri.clone(), parent.clone(), popup.clone());
                    btn.connect_clicked(move |_| {
                        item_popup.popdown();
                        recent::open_recent_item(&uri, &item_parent);
                    });

                    rec_grid.attach(&btn, col, row, 1, 1);
                }
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
