use crate::{
    config::Config,
    platform::{DesktopCommand, DesktopEvent, DesktopShellBackend, WindowInfo, X11Backend},
    popups,
    services::{
        applications::{self, Application},
        system,
    },
};
use gtk::{gdk, glib, prelude::*};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::mpsc, time::Duration};

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
        .title("Monolito Panel")
        .decorated(false)
        .resizable(false)
        .default_height(i32::from(config.height))
        .build();
    window.add_css_class("monolito-panel");
    if !display.is_composited() {
        window.add_css_class("uncomposited");
    }
    window.add_css_class(if appearance.theme == monolito_ui::Theme::Light {
        "light"
    } else {
        "dark"
    });
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    row.set_height_request(i32::from(config.height));
    let start = gtk::MenuButton::builder()
        .icon_name("view-app-grid-symbolic")
        .tooltip_text(gettextrs::gettext("Start"))
        .build();
    start.add_css_class("start-button");
    row.append(&start);
    let taskbar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    taskbar.set_halign(gtk::Align::Start);
    let tasks_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::External)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .child(&taskbar)
        .build();
    row.append(&tasks_scroll);
    let quick = gtk::MenuButton::builder()
        .tooltip_text(gettextrs::gettext("Quick settings"))
        .build();
    quick.add_css_class("status-button");
    let icons = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    icons.append(&gtk::Image::from_icon_name("preferences-system-symbolic"));
    let quick_text = gtk::Label::new(Some(&gettextrs::gettext("System")));
    icons.append(&quick_text);
    quick.set_child(Some(&icons));
    quick.set_popover(Some(&popups::quick_settings::build(window.upcast_ref())));
    row.append(&quick);
    let clock = gtk::MenuButton::builder()
        .tooltip_text(gettextrs::gettext("Calendar and notifications"))
        .build();
    clock.add_css_class("clock-button");
    let clock_label = gtk::Label::new(None);
    clock_label.set_justify(gtk::Justification::Right);
    clock.set_child(Some(&clock_label));
    clock.set_popover(Some(&popups::calendar::build()));
    row.append(&clock);
    window.set_child(Some(&row));
    let update_clock =
        move || clock_label.set_text(&chrono::Local::now().format("%H:%M\n%d/%m/%Y").to_string());
    update_clock();
    let weak_window = window.downgrade();
    glib::timeout_add_seconds_local(1, move || {
        if weak_window.upgrade().is_none() {
            return glib::ControlFlow::Break;
        }
        update_clock();
        glib::ControlFlow::Continue
    });
    let reserve = !std::env::args().any(|a| a == "--dev");
    let (desktop, events) = X11Backend::start(config.height, config.monitor, reserve);
    let config = Rc::new(RefCell::new(config));
    let apps: Rc<RefCell<Vec<Application>>> = Rc::new(RefCell::new(vec![]));
    let windows: Rc<RefCell<Vec<WindowInfo>>> = Rc::new(RefCell::new(vec![]));
    let redraw: Rc<dyn Fn()> = {
        let (taskbar, config, apps, windows, desktop, parent) = (
            taskbar.clone(),
            config.clone(),
            apps.clone(),
            windows.clone(),
            desktop.clone(),
            window.clone(),
        );
        let view = RefCell::new(TaskbarView::default());
        Rc::new(move || {
            render_taskbar(
                &taskbar,
                &apps.borrow(),
                &windows,
                &config.borrow(),
                &desktop,
                parent.upcast_ref(),
                &mut view.borrow_mut(),
            )
        })
    };
    let (apps_tx, apps_rx) = mpsc::channel();
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
                    window
                        .set_default_size(geometry.width as i32, i32::from(config.borrow().height));
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
                    window
                        .set_default_size(geometry.width as i32, i32::from(config.borrow().height));
                    window.present();
                    eprintln!(
                        "Dock attached: {}x{} at {},{}; reservation {}",
                        geometry.width,
                        config.borrow().height,
                        geometry.x,
                        geometry.y + geometry.height as i32 - i32::from(config.borrow().height),
                        if geometry.bottom_strut(config.borrow().height.into(), reserve)[3] == 0 {
                            "off"
                        } else {
                            "on"
                        }
                    );
                }
                DesktopEvent::Geometry(geometry) => {
                    window
                        .set_default_size(geometry.width as i32, i32::from(config.borrow().height));
                    let _ = desktop.send(DesktopCommand::Reposition);
                }
                DesktopEvent::Windows(value) => {
                    if *windows.borrow() != value {
                        *windows.borrow_mut() = value;
                        redraw();
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

#[derive(Default)]
struct TaskbarView {
    structure: Vec<(String, Vec<u32>)>,
    buttons: BTreeMap<String, gtk::Widget>,
    window_items: BTreeMap<u32, gtk::Button>,
}

fn render_taskbar(
    container: &gtk::Box,
    apps: &[Application],
    live_windows: &Rc<RefCell<Vec<WindowInfo>>>,
    config: &Config,
    desktop: &mpsc::Sender<DesktopCommand>,
    parent: &gtk::Window,
    view: &mut TaskbarView,
) {
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
    // Keep widgets alive across focus/minimize/title changes. Replacing a button
    // under the pointer loses GTK's input sequence and can swallow the next click.
    if structure != view.structure {
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }
        view.buttons.clear();
        view.window_items.clear();
        for id in &order {
            let app = apps.iter().find(|a| &a.id == id);
            let group = groups.get(id).cloned().unwrap_or_default();
            let icon = app.map(|a| a.image(24)).unwrap_or_else(|| {
                let icon = gtk::Image::from_icon_name("application-x-executable-symbolic");
                icon.set_pixel_size(24);
                icon
            });
            let inner = gtk::Box::new(gtk::Orientation::Vertical, 0);
            inner.append(&icon);
            let marker = gtk::Label::new(Some(match group.len() {
                0 => " ",
                1 => "━",
                _ => "••",
            }));
            marker.add_css_class("window-marker");
            inner.append(&marker);
            let widget: gtk::Widget = if group.len() > 1 {
                let button = gtk::MenuButton::new();
                button.set_child(Some(&inner));
                let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
                let popover = popups::popover(&list);
                for window in &group {
                    let item = gtk::Button::with_label(&window.title);
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
            container.append(&widget);
            view.buttons.insert(id.clone(), widget);
        }
        view.structure = structure;
    }
    for id in &order {
        let group = groups.get(id).cloned().unwrap_or_default();
        if let Some(button) = view.buttons.get(id) {
            if group.iter().any(|w| w.active) {
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
    }
    for window in windows.iter() {
        if let Some(item) = view.window_items.get(&window.id) {
            item.set_label(&window.title);
        }
    }
}
