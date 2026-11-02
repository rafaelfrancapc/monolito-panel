use crate::{
    config::{Config, TaskbarAlignment},
    platform::{DesktopCommand, DesktopEvent, DesktopShellBackend, WindowInfo, X11Backend},
    popups,
    services::{
        applications::{self, Application},
        audio, network, power, system,
    },
};
use gtk::{gdk, glib, prelude::*};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::mpsc, time::Duration};

type RedrawLink = Rc<RefCell<Option<std::rc::Weak<dyn Fn()>>>>;
struct TaskbarActions {
    config: Rc<RefCell<Config>>,
    redraw: RedrawLink,
}

pub fn activate(application: &gtk::Application) {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration: {e:#}");
            application.quit();
            return;
        }
    };
    let mut appearance = match monolito_ui::Appearance::load() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Appearance: {e}");
            application.quit();
            return;
        }
    };
    for arg in std::env::args() {
        match arg.as_str() {
            "--theme=light" => appearance.theme = monolito_ui::Theme::Light,
            "--theme=dark" => appearance.theme = monolito_ui::Theme::Dark,
            _ => {}
        }
    }
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_theme_name(Some("Adwaita"));
        settings
            .set_gtk_application_prefer_dark_theme(appearance.theme == monolito_ui::Theme::Dark);
        settings.set_gtk_enable_animations(appearance.animations);
    }
    let Some(display) = gdk::Display::default() else {
        eprintln!("An accessible X11 display is required.");
        application.quit();
        return;
    };

    // Assets travel with the binary, including when installed outside the checkout.
    gtk::gio::resources_register_include!("panel.gresource").expect("Embedded panel icons");
    let icon_theme = gtk::IconTheme::for_display(&display);
    icon_theme.add_resource_path("/org/monolito/Panel/icons");
    if let Some(name) = appearance
        .icon_theme
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        if icon_theme
            .search_path()
            .iter()
            .any(|root| root.join(name).join("index.theme").is_file())
        {
            if let Some(settings) = gtk::Settings::default() {
                settings.set_gtk_icon_theme_name(Some(name));
            }
        } else {
            eprintln!("Icon theme {name} is not installed; using the desktop fallback.");
        }
    }

    let provider = gtk::CssProvider::new();
    provider.load_from_string(&format!(
        "{}\n{}",
        monolito_ui::CSS,
        appearance.accent_css()
    ));
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("Desktop Panel")
        .decorated(false)
        .resizable(false)
        .default_height(i32::from(config.height))
        .build();
    window.add_css_class("monolito-panel");
    if !appearance.animations {
        window.add_css_class("no-motion");
    }
    if !display.is_composited() {
        window.add_css_class("uncomposited");
    }
    window.add_css_class(if appearance.theme == monolito_ui::Theme::Light {
        "light"
    } else {
        "dark"
    });

    let row = gtk::CenterBox::new();
    row.add_css_class("panel-row");
    row.set_height_request(i32::from(config.height));

    // Start button
    let start = gtk::MenuButton::builder()
        .tooltip_text(gettextrs::gettext("Start"))
        .build();
    start.add_css_class("start-button");
    let start_icon = gtk::Image::from_icon_name("start-symbolic");
    start_icon.set_pixel_size(24);
    start.set_child(Some(&start_icon));
    let search_button = gtk::Button::from_icon_name("system-search-symbolic");
    search_button.add_css_class("search-button");
    search_button.set_tooltip_text(Some(&gettextrs::gettext("Search applications")));
    let (search_parent, search_start) = (window.clone(), start.clone());
    search_button.connect_clicked(move |_| {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_ascii_lowercase();
        if desktop.contains("xfce") {
            let parent = search_parent.clone();
            glib::MainContext::default().spawn_local(async move {
                if let Err(error) = system::command(&["xfce4-appfinder", "--collapsed"]).await {
                    system::show_error(parent.upcast_ref(), &error);
                }
            });
        } else if let Some(popover) = search_start.popover() {
            popover.popup();
        }
    });

    // Taskbar container
    let taskbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);

    let taskbar_alignment = config.alignment;
    let center_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    center_box.set_hexpand(true);
    center_box.set_valign(gtk::Align::Center);
    center_box.set_halign(match taskbar_alignment {
        TaskbarAlignment::Center => gtk::Align::Center,
        TaskbarAlignment::Left => gtk::Align::Start,
    });

    center_box.append(&start);
    center_box.append(&search_button);
    center_box.add_css_class("taskbar-group");
    taskbar.add_css_class("taskbar-items");

    let tasks_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::External)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_width(true)
        .max_content_width(800)
        .child(&taskbar)
        .build();
    center_box.append(&tasks_scroll);
    let left_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    left_box.set_valign(gtk::Align::Center);
    if config.weather_enabled {
        left_box.append(&weather_button(&config, window.upcast_ref()));
    }
    row.set_start_widget(Some(&left_box));
    if taskbar_alignment == TaskbarAlignment::Center {
        row.set_center_widget(Some(&center_box));
    } else {
        center_box.set_hexpand(false);
        left_box.append(&center_box);
    }

    // Right tray area (Windows 11 Unified Quick Settings + Clock)
    let tray_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    tray_box.add_css_class("panel-tray");
    tray_box.set_halign(gtk::Align::End);
    tray_box.set_valign(gtk::Align::Center);
    let notification_tray = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    notification_tray.add_css_class("notification-tray");
    crate::services::tray::install(&notification_tray);
    tray_box.append(&notification_tray);

    let quick = gtk::MenuButton::builder()
        .tooltip_text(gettextrs::gettext("Quick settings"))
        .build();
    quick.add_css_class("status-pill-button");

    let tray_icons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let tray_net_icon = gtk::Image::from_icon_name("network-wireless-symbolic");
    let tray_vol_icon = gtk::Image::from_icon_name("audio-volume-high-symbolic");
    let tray_bat_icon = gtk::Image::from_icon_name("battery-level-100-charged-symbolic");
    for icon in [&tray_net_icon, &tray_vol_icon, &tray_bat_icon] {
        icon.set_pixel_size(16);
        icon.set_valign(gtk::Align::Center);
    }
    tray_bat_icon.set_visible(false);

    tray_icons.append(&tray_net_icon);
    tray_icons.append(&tray_vol_icon);
    tray_icons.append(&tray_bat_icon);
    quick.set_child(Some(&tray_icons));
    quick.set_popover(Some(&popups::quick_settings::build(window.upcast_ref())));
    tray_box.append(&quick);

    let clock = gtk::MenuButton::builder()
        .tooltip_text(gettextrs::gettext("Calendar and notifications"))
        .build();
    clock.add_css_class("clock-button");
    let clock_label = gtk::Label::new(None);
    clock_label.set_justify(gtk::Justification::Right);
    clock.set_child(Some(&clock_label));
    clock.set_popover(Some(&popups::calendar::build()));
    tray_box.append(&clock);

    row.set_end_widget(Some(&tray_box));
    window.set_child(Some(&row));

    let update_clock =
        move || clock_label.set_text(&chrono::Local::now().format("%H:%M\n%d/%m/%Y").to_string());
    update_clock();

    // 1-second clock timer
    let weak_window = window.downgrade();
    glib::timeout_add_seconds_local(1, move || {
        if weak_window.upgrade().is_none() {
            return glib::ControlFlow::Break;
        }
        update_clock();
        glib::ControlFlow::Continue
    });

    // 2-second tray status polling
    {
        let weak_window = window.downgrade();
        let (t_net, t_vol, t_bat) = (
            tray_net_icon.clone(),
            tray_vol_icon.clone(),
            tray_bat_icon.clone(),
        );
        glib::timeout_add_seconds_local(2, move || {
            if weak_window.upgrade().is_none() {
                return glib::ControlFlow::Break;
            }
            let (net_i, vol_i, bat_i) = (t_net.clone(), t_vol.clone(), t_bat.clone());
            glib::MainContext::default().spawn_local(async move {
                if let Ok(status) = network::get_status().await {
                    net_i.set_icon_name(Some(network::network_status_icon(&status)));
                    net_i.set_tooltip_text(Some(&gettextrs::gettext(
                        network::network_status_message(&status),
                    )));
                }
                if let Ok(vol) = audio::get_master_volume().await {
                    vol_i.set_icon_name(Some(audio::volume_icon(vol.value, vol.muted)));
                }
                if let Ok(bat_opt) = power::get_battery().await {
                    if let Some(bat) = bat_opt {
                        bat_i.set_visible(true);
                        bat_i.set_icon_name(Some(power::battery_icon(bat.percentage, &bat.state)));
                    } else {
                        bat_i.set_visible(false);
                    }
                }
            });
            glib::ControlFlow::Continue
        });
    }

    let reserve = !std::env::args().any(|a| a == "--dev");
    let scale = window.scale_factor().max(1);
    let physical_height = config.height.saturating_mul(scale as u16);
    let (desktop, events) = X11Backend::start(physical_height, config.monitor, reserve);
    let config = Rc::new(RefCell::new(config));
    let apps: Rc<RefCell<Vec<Application>>> = Rc::new(RefCell::new(vec![]));
    let windows: Rc<RefCell<Vec<WindowInfo>>> = Rc::new(RefCell::new(vec![]));
    let redraw_link: RedrawLink = Rc::new(RefCell::new(None));
    let redraw: Rc<dyn Fn()> = {
        let (taskbar, config, apps, windows, desktop, parent, redraw_link) = (
            taskbar.clone(),
            config.clone(),
            apps.clone(),
            windows.clone(),
            desktop.clone(),
            window.clone(),
            redraw_link.clone(),
        );
        let view = RefCell::new(TaskbarView::default());
        let actions = TaskbarActions {
            config,
            redraw: redraw_link,
        };
        Rc::new(move || {
            render_taskbar(
                &taskbar,
                &apps.borrow(),
                &windows,
                &desktop,
                parent.upcast_ref(),
                &mut view.borrow_mut(),
                &actions,
            )
        })
    };
    *redraw_link.borrow_mut() = Some(Rc::downgrade(&redraw));

    let (apps_tx, apps_rx) = mpsc::channel();
    let monitor_tx = apps_tx.clone();
    let app_monitor = gtk::gio::AppInfoMonitor::get();
    app_monitor.connect_changed(move |_| {
        let tx = monitor_tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(applications::installed());
        });
    });
    std::thread::spawn(move || {
        let _ = apps_tx.send(applications::installed());
    });
    let tx = desktop.clone();
    window.connect_map(move |_| {
        let _ = tx.send(DesktopCommand::Reposition);
    });
    let tx = desktop.clone();
    window.connect_close_request(move |_| {
        let _ = tx.send(DesktopCommand::Stop);
        glib::Propagation::Proceed
    });

    let weak_window = window.downgrade();
    glib::timeout_add_local(Duration::from_millis(40), move || {
        let _keep_app_monitor_alive = &app_monitor;
        let Some(window) = weak_window.upgrade() else {
            let _ = desktop.send(DesktopCommand::Stop);
            return glib::ControlFlow::Break;
        };
        if let Ok(installed) = apps_rx.try_recv() {
            *apps.borrow_mut() = installed;
            start.set_popover(Some(&popups::start_menu::build(
                &apps.borrow(),
                config.clone(),
                window.upcast_ref(),
                redraw.clone(),
            )));
            redraw();
        }
        for event in events.try_iter() {
            match event {
                DesktopEvent::Ready(geometry) => {
                    window.set_default_size(
                        geometry.width as i32 / scale,
                        i32::from(config.borrow().height),
                    );
                    gtk::prelude::WidgetExt::realize(&window);
                    let xid = crate::platform::native_window_id(window.upcast_ref());
                    if let Some(id) = xid {
                        let _ = desktop.send(DesktopCommand::Attach(id));
                    } else {
                        eprintln!("No native GTK X11 surface; Wayland is not supported.");
                        window.close();
                        return glib::ControlFlow::Break;
                    }
                }
                DesktopEvent::Attached(geometry) => {
                    window.set_default_size(
                        geometry.width as i32 / scale,
                        i32::from(config.borrow().height),
                    );
                    window.present();
                    eprintln!(
                        "Dock attached: {}x{} at {},{}; reservation {}",
                        geometry.width,
                        physical_height,
                        geometry.x,
                        geometry.y + geometry.height as i32 - i32::from(physical_height),
                        if geometry.bottom_strut(physical_height.into(), reserve)[3] == 0 {
                            "off"
                        } else {
                            "on"
                        }
                    );
                }
                DesktopEvent::Geometry(geometry) => {
                    window.set_default_size(
                        geometry.width as i32 / scale,
                        i32::from(config.borrow().height),
                    );
                    let _ = desktop.send(DesktopCommand::Reposition);
                }
                DesktopEvent::Windows(value) => {
                    if *windows.borrow() != value {
                        *windows.borrow_mut() = value;
                        redraw();
                    }
                }
                DesktopEvent::ToggleStart => {
                    if let Some(popover) = start.popover() {
                        if popover.is_visible() {
                            popover.popdown();
                        } else {
                            popover.popup();
                        }
                    }
                }
                DesktopEvent::Error(message) => {
                    eprintln!("{message}");
                    if window.is_visible() {
                        system::show_error(window.upcast_ref(), &message);
                    } else {
                        window.close();
                        return glib::ControlFlow::Break;
                    }
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

fn weather_button(config: &Config, parent: &gtk::Window) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("weather-button");
    button.set_tooltip_text(Some(&gettextrs::gettext("Updating weather…")));
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let icon = gtk::Image::from_icon_name("weather-overcast-symbolic");
    icon.set_pixel_size(20);
    let temperature = gtk::Label::new(Some("—"));
    temperature.add_css_class("weather-temperature");
    content.append(&icon);
    content.append(&temperature);
    button.set_child(Some(&content));

    let location = config.weather_location.clone();
    let refresh: Rc<dyn Fn()> = {
        let (icon, temperature, button, location) = (
            icon.clone(),
            temperature.clone(),
            button.clone(),
            location.clone(),
        );
        Rc::new(move || {
            let (icon, temperature, button, location) = (
                icon.clone(),
                temperature.clone(),
                button.clone(),
                location.clone(),
            );
            glib::MainContext::default().spawn_local(async move {
                match crate::services::weather::current(location.as_deref()).await {
                    Ok(weather) => {
                        icon.set_icon_name(Some(weather.icon));
                        temperature.set_text(&weather.temperature);
                        button.set_tooltip_text(Some(&weather.condition));
                    }
                    Err(error) => {
                        button.set_tooltip_text(Some(&format!(
                            "{}: {error}",
                            gettextrs::gettext("Weather unavailable")
                        )));
                    }
                }
            });
        })
    };
    refresh();
    let timer_refresh = refresh.clone();
    glib::timeout_add_seconds_local(30 * 60, move || {
        timer_refresh();
        glib::ControlFlow::Continue
    });

    let hide = gtk::Button::with_label(&gettextrs::gettext("Hide weather"));
    let menu = popups::popover(&hide);
    menu.set_parent(&button);
    let (hide_button, hide_parent, hide_menu) = (button.clone(), parent.clone(), menu.clone());
    hide.connect_clicked(move |_| {
        match Config::load().and_then(|mut config| {
            config.weather_enabled = false;
            config.save()
        }) {
            Ok(()) => {
                hide_menu.popdown();
                hide_button.set_visible(false);
            }
            Err(error) => system::show_error(&hide_parent, &error.to_string()),
        }
    });
    let gesture = gtk::GestureClick::new();
    gesture.set_button(3);
    let open_menu = menu.clone();
    gesture.connect_pressed(move |_, _, _, _| open_menu.popup());
    button.add_controller(gesture);
    button.connect_unrealize(move |_| menu.unparent());
    button
}

#[derive(Default)]
struct TaskbarView {
    structure: Vec<(String, Vec<u32>)>,
    buttons: BTreeMap<String, gtk::Widget>,
    indicators: BTreeMap<String, gtk::Box>,
    window_items: BTreeMap<u32, gtk::Button>,
    fallback_icons: BTreeMap<String, (gtk::Image, Option<crate::platform::WindowIcon>)>,
}

fn render_taskbar(
    container: &gtk::Box,
    apps: &[Application],
    live_windows: &Rc<RefCell<Vec<WindowInfo>>>,
    desktop: &mpsc::Sender<DesktopCommand>,
    parent: &gtk::Window,
    view: &mut TaskbarView,
    actions: &TaskbarActions,
) {
    let config_guard = actions.config.borrow();
    let config = &*config_guard;
    let windows = live_windows.borrow();
    let mut groups: BTreeMap<String, Vec<&WindowInfo>> = BTreeMap::new();
    for window in windows.iter() {
        let id = apps
            .iter()
            .find(|a| a.matches_class(&window.class))
            .map(|a| a.id.clone())
            .unwrap_or_else(|| {
                if window.class.is_empty() {
                    format!("window:{}", window.id)
                } else {
                    format!("class:{}", window.class.to_lowercase())
                }
            });
        groups.entry(id).or_default().push(window);
    }
    let mut order = config.pinned.clone();
    for id in groups.keys() {
        if !order.contains(id) {
            order.push(id.clone());
        }
    }
    order.retain(|id| apps.iter().any(|a| &a.id == id) || groups.contains_key(id));
    let structure: Vec<_> = order
        .iter()
        .map(|id| {
            (
                id.clone(),
                groups
                    .get(id)
                    .map(|g| g.iter().map(|w| w.id).collect())
                    .unwrap_or_default(),
            )
        })
        .collect();

    if structure != view.structure {
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }
        view.buttons.clear();
        view.indicators.clear();
        view.window_items.clear();
        view.fallback_icons.clear();

        for id in &order {
            let app = apps.iter().find(|a| &a.id == id);
            let group = groups.get(id).cloned().unwrap_or_default();
            let icon = app
                .map(|a| a.image(24))
                .unwrap_or_else(|| crate::services::icons::image("", &[], 24));
            if icon.icon_name().as_deref() == Some("application-x-executable-symbolic") {
                view.fallback_icons.insert(id.clone(), (icon.clone(), None));
            }

            // Keep the glyph centered independently of the running indicator.
            let inner = gtk::Overlay::new();
            inner.set_size_request(40, 40);
            icon.set_halign(gtk::Align::Center);
            icon.set_valign(gtk::Align::Center);
            inner.set_child(Some(&icon));

            // Windows 11 Running Indicator Bar
            let indicator = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            indicator.set_halign(gtk::Align::Center);
            indicator.set_valign(gtk::Align::End);
            indicator.set_margin_bottom(2);
            indicator.add_css_class("task-indicator");
            inner.add_overlay(&indicator);

            let widget: gtk::Widget = if group.len() > 1 {
                let button = gtk::MenuButton::new();
                button.set_child(Some(&inner));

                let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
                list.add_css_class("window-group-menu");
                let popover = popups::popover(&list);

                for window in &group {
                    let item_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                    let item = gtk::Button::new();
                    let title_label = gtk::Label::new(Some(&window.title));
                    title_label.set_xalign(0.0);
                    title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    title_label.set_max_width_chars(24);
                    title_label.set_hexpand(true);
                    item_box.append(&title_label);

                    // Close button for individual window
                    let close_btn = gtk::Button::from_icon_name("window-close-symbolic");
                    close_btn.add_css_class("icon-button-subtle");
                    let (tx_close, wid_close) = (desktop.clone(), window.id);
                    close_btn.connect_clicked(move |_| {
                        let _ = tx_close.send(DesktopCommand::Close(wid_close));
                    });
                    item_box.append(&close_btn);

                    item.set_child(Some(&item_box));
                    item.add_css_class("window-item-button");

                    let (tx, wid, pop) = (desktop.clone(), window.id, popover.clone());
                    item.connect_clicked(move |_| {
                        let _ = tx.send(DesktopCommand::Activate(wid));
                        pop.popdown();
                    });
                    list.append(&item);
                    view.window_items.insert(window.id, item);
                }
                button.set_popover(Some(&popover));
                button.upcast()
            } else {
                let button = gtk::Button::new();
                button.set_child(Some(&inner));
                if let Some(window) = group.first() {
                    let (tx, wid, live) = (desktop.clone(), window.id, live_windows.clone());
                    button.connect_clicked(move |_| {
                        let active = live.borrow().iter().any(|w| w.id == wid && w.active);
                        let _ = tx.send(if active {
                            DesktopCommand::Minimize(wid)
                        } else {
                            DesktopCommand::Activate(wid)
                        });
                    });
                } else {
                    let (parent, id) = (parent.clone(), id.clone());
                    button.connect_clicked(move |_| applications::launch(&id, &parent));
                }
                button.upcast()
            };

            widget.add_css_class("task-button");
            if let Some(app) = app {
                let action_list = gtk::Box::new(gtk::Orientation::Vertical, 4);
                for (in_start, label) in [
                    (
                        true,
                        if config.start_pinned.contains(&app.id) {
                            "Unpin from Start"
                        } else {
                            "Pin to Start"
                        },
                    ),
                    (
                        false,
                        if config.pinned.contains(&app.id) {
                            "Unpin from taskbar"
                        } else {
                            "Pin to taskbar"
                        },
                    ),
                ] {
                    let action = gtk::Button::with_label(&gettextrs::gettext(label));
                    let (id, config, parent, redraw_link) = (
                        app.id.clone(),
                        actions.config.clone(),
                        parent.clone(),
                        actions.redraw.clone(),
                    );
                    action.connect_clicked(move |_| {
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
                        } else if let Some(redraw) =
                            redraw_link.borrow().as_ref().and_then(|w| w.upgrade())
                        {
                            glib::idle_add_local_once(move || redraw());
                        }
                    });
                    action_list.append(&action);
                }
                let shortcut =
                    gtk::Button::with_label(&gettextrs::gettext("Create desktop shortcut"));
                let (id, parent) = (app.id.clone(), parent.clone());
                shortcut.connect_clicked(move |_| {
                    if let Err(e) = applications::create_desktop_shortcut(&id) {
                        system::show_error(&parent, &e);
                    }
                });
                action_list.append(&shortcut);
                let context = popups::popover(&action_list);
                context.set_parent(&widget);
                let gesture = gtk::GestureClick::new();
                gesture.set_button(3);
                let menu = context.clone();
                gesture.connect_pressed(move |_, _, _, _| menu.popup());
                widget.add_controller(gesture);
                widget.connect_unrealize(move |_| context.unparent());
            }
            container.append(&widget);
            view.buttons.insert(id.clone(), widget);
            view.indicators.insert(id.clone(), indicator);
        }
        view.structure = structure;
    }

    // Update active / running indicator states
    for id in &order {
        let group = groups.get(id).cloned().unwrap_or_default();
        let is_running = !group.is_empty();
        let is_active = group.iter().any(|w| w.active);

        // Some clients publish their icon after mapping. Update it without
        // rebuilding the button, which would close an open window-group menu.
        if let Some((image, previous)) = view.fallback_icons.get_mut(id) {
            let current = group.iter().find_map(|w| w.icon.as_ref());
            if current != previous.as_ref() {
                if let Some(icon) = current {
                    image.set_paintable(
                        crate::services::icons::window_image(icon, 24)
                            .paintable()
                            .as_ref(),
                    );
                } else {
                    image.set_icon_name(Some("application-x-executable-symbolic"));
                }
                *previous = current.cloned();
            }
        }

        if let Some(button) = view.buttons.get(id) {
            if is_active {
                button.add_css_class("active-window");
            } else {
                button.remove_css_class("active-window");
            }

            let name = apps
                .iter()
                .find(|a| &a.id == id)
                .map(|a| a.name.as_str())
                .or_else(|| group.first().map(|w| w.title.as_str()))
                .unwrap_or("Application");
            button.set_tooltip_text(Some(name));
        }

        if let Some(ind) = view.indicators.get(id) {
            ind.remove_css_class("running");
            ind.remove_css_class("active");
            ind.remove_css_class("multiple");

            if is_running {
                ind.add_css_class("running");
                if is_active {
                    ind.add_css_class("active");
                }
                if group.len() > 1 {
                    ind.add_css_class("multiple");
                }
            }
        }
    }

    for window in windows.iter() {
        if let Some(item) = view.window_items.get(&window.id) {
            item.set_tooltip_text(Some(&window.title));
        }
    }
}
