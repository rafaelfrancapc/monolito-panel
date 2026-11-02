//! Freedesktop StatusNotifierItem host and watcher for background applications.
use gtk::{gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const PATH: &str = "/StatusNotifierWatcher";
const XML: &str = r#"<node><interface name="org.kde.StatusNotifierWatcher">
  <method name="RegisterStatusNotifierItem"><arg type="s" direction="in"/></method>
  <method name="RegisterStatusNotifierHost"><arg type="s" direction="in"/></method>
  <property name="RegisteredStatusNotifierItems" type="as" access="read"/>
  <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
  <property name="ProtocolVersion" type="i" access="read"/>
  <signal name="StatusNotifierItemRegistered"><arg type="s"/></signal>
  <signal name="StatusNotifierItemUnregistered"><arg type="s"/></signal>
  <signal name="StatusNotifierHostRegistered"/>
</interface></node>"#;

#[derive(Clone, PartialEq)]
struct Item {
    service: String,
    path: String,
    icon: String,
    title: String,
    pixels: Option<crate::platform::WindowIcon>,
}

fn split_item(value: &str) -> Option<(String, String)> {
    let index = value.find('/');
    let (service, path) = match index {
        Some(index) => (&value[..index], &value[index..]),
        None => (value, "/StatusNotifierItem"),
    };
    if service.is_empty() || !path.starts_with('/') {
        None
    } else {
        Some((service.into(), path.into()))
    }
}

pub fn install(container: &gtk::Box) {
    let Ok(connection) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else {
        return;
    };
    let items = Rc::new(RefCell::new(Vec::<String>::new()));
    let request = connection.call_sync(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "RequestName",
        Some(&(WATCHER, 4u32).to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        1000,
        gio::Cancellable::NONE,
    );
    let owned = request.ok().and_then(|v| v.child_value(0).get::<u32>()) == Some(1);
    if owned {
        let node = gio::DBusNodeInfo::for_xml(XML).expect("StatusNotifierWatcher XML");
        let interface = node.lookup_interface(WATCHER).expect("Watcher interface");
        let method_items = items.clone();
        let property_items = items.clone();
        let registration = connection
            .register_object(PATH, &interface)
            .method_call(move |conn, sender, _, _, method, parameters, invocation| {
                match method {
                    "RegisterStatusNotifierItem" => {
                        let requested = parameters
                            .child_value(0)
                            .get::<String>()
                            .unwrap_or_default();
                        let item = if requested.starts_with('/') {
                            format!("{}{requested}", sender.unwrap_or_default())
                        } else {
                            requested
                        };
                        if split_item(&item).is_some() && !method_items.borrow().contains(&item) {
                            method_items.borrow_mut().push(item.clone());
                            let _ = conn.emit_signal(
                                None,
                                PATH,
                                WATCHER,
                                "StatusNotifierItemRegistered",
                                Some(&(item,).to_variant()),
                            );
                        }
                    }
                    "RegisterStatusNotifierHost" => {
                        let _ = conn.emit_signal(
                            None,
                            PATH,
                            WATCHER,
                            "StatusNotifierHostRegistered",
                            Some(&().to_variant()),
                        );
                    }
                    _ => {}
                }
                invocation.return_value(Some(&().to_variant()));
            })
            .property(move |_, _, _, _, name| match name {
                "RegisteredStatusNotifierItems" => property_items.borrow().clone().to_variant(),
                "IsStatusNotifierHostRegistered" => true.to_variant(),
                _ => 0i32.to_variant(),
            })
            .build();
        if registration.is_err() {
            eprintln!("Could not register StatusNotifierWatcher");
        }
    }

    let shown = Rc::new(RefCell::new(Vec::<Item>::new()));
    let busy = Rc::new(Cell::new(false));
    let refresh = {
        let (connection, items, shown, container, busy) = (
            connection.clone(),
            items.clone(),
            shown.clone(),
            container.clone(),
            busy.clone(),
        );
        move || {
            if busy.replace(true) {
                return;
            }
            let (conn, items, shown, box_, busy) = (
                connection.clone(),
                items.clone(),
                shown.clone(),
                container.clone(),
                busy.clone(),
            );
            glib::MainContext::default().spawn_local(async move {
                let names = if owned {
                    items.borrow().clone()
                } else {
                    conn.call_future(
                        Some(WATCHER),
                        PATH,
                        "org.freedesktop.DBus.Properties",
                        "Get",
                        Some(&(WATCHER, "RegisteredStatusNotifierItems").to_variant()),
                        None,
                        gio::DBusCallFlags::NO_AUTO_START,
                        1000,
                    )
                    .await
                    .ok()
                    .and_then(|v| v.child_value(0).get::<glib::Variant>())
                    .and_then(|v| v.get::<Vec<String>>())
                    .unwrap_or_default()
                };
                let mut current = Vec::new();
                let mut dead = Vec::new();
                for value in &names {
                    let Some((service, path)) = split_item(value) else {
                        dead.push(value.clone());
                        continue;
                    };
                    let reply = conn
                        .call_future(
                            Some(&service),
                            &path,
                            "org.freedesktop.DBus.Properties",
                            "GetAll",
                            Some(&("org.kde.StatusNotifierItem",).to_variant()),
                            None,
                            gio::DBusCallFlags::NO_AUTO_START,
                            700,
                        )
                        .await;
                    let Ok(reply) = reply else {
                        dead.push(value.clone());
                        continue;
                    };
                    let Some(props) = reply.child_value(0).get::<HashMap<String, glib::Variant>>()
                    else {
                        continue;
                    };
                    let property = |name: &str| {
                        props
                            .get(name)
                            .and_then(|v| {
                                v.get::<String>().or_else(|| {
                                    v.get::<glib::Variant>()
                                        .and_then(|inner| inner.get::<String>())
                                })
                            })
                            .unwrap_or_default()
                    };
                    if property("Status") == "Passive" {
                        continue;
                    }
                    let icon = property("IconName");
                    let pixels = props
                        .get("IconPixmap")
                        .and_then(|v| {
                            v.get::<Vec<(i32, i32, Vec<u8>)>>().or_else(|| {
                                v.get::<glib::Variant>()
                                    .and_then(|inner| inner.get::<Vec<(i32, i32, Vec<u8>)>>())
                            })
                        })
                        .and_then(|pixmaps| {
                            pixmaps
                                .into_iter()
                                .filter(|(w, h, bytes)| {
                                    *w > 0
                                        && *h > 0
                                        && *w <= 256
                                        && *h <= 256
                                        && bytes.len() == *w as usize * *h as usize * 4
                                })
                                .min_by_key(|(w, h, _)| w.abs_diff(24) + h.abs_diff(24))
                        })
                        .map(|(w, h, bytes)| crate::platform::WindowIcon {
                            width: w as u32,
                            height: h as u32,
                            rgba: bytes
                                .chunks_exact(4)
                                .flat_map(|p| [p[1], p[2], p[3], p[0]])
                                .collect(),
                        });
                    current.push(Item {
                        service,
                        path,
                        icon,
                        title: property("Title"),
                        pixels,
                    });
                }
                if owned && !dead.is_empty() {
                    items.borrow_mut().retain(|item| !dead.contains(item));
                    for item in dead {
                        let _ = conn.emit_signal(
                            None,
                            PATH,
                            WATCHER,
                            "StatusNotifierItemUnregistered",
                            Some(&(item,).to_variant()),
                        );
                    }
                }
                if *shown.borrow() != current {
                    while let Some(child) = box_.first_child() {
                        box_.remove(&child);
                    }
                    for item in &current {
                        let icon = if item.icon.is_empty() {
                            item.pixels
                                .as_ref()
                                .map(|pixels| super::icons::window_image(pixels, 16))
                                .unwrap_or_else(|| super::icons::image("", &[], 16))
                        } else {
                            super::icons::image(&item.icon, &[], 16)
                        };
                        let button = gtk::Button::new();
                        button.add_css_class("tray-item");
                        button.set_child(Some(&icon));
                        button.set_tooltip_text(Some(&item.title));
                        let (conn, service, path) =
                            (conn.clone(), item.service.clone(), item.path.clone());
                        let (menu_conn, menu_service, menu_path) =
                            (conn.clone(), service.clone(), path.clone());
                        button.connect_clicked(move |_| {
                            let (conn, service, path) =
                                (conn.clone(), service.clone(), path.clone());
                            glib::MainContext::default().spawn_local(async move {
                                let _ = conn
                                    .call_future(
                                        Some(&service),
                                        &path,
                                        "org.kde.StatusNotifierItem",
                                        "Activate",
                                        Some(&(0i32, 0i32).to_variant()),
                                        None,
                                        gio::DBusCallFlags::NO_AUTO_START,
                                        1000,
                                    )
                                    .await;
                            });
                        });
                        let gesture = gtk::GestureClick::new();
                        gesture.set_button(3);
                        let (conn, service, path) = (menu_conn, menu_service, menu_path);
                        gesture.connect_pressed(move |_, _, _, _| {
                            let (conn, service, path) =
                                (conn.clone(), service.clone(), path.clone());
                            glib::MainContext::default().spawn_local(async move {
                                let _ = conn
                                    .call_future(
                                        Some(&service),
                                        &path,
                                        "org.kde.StatusNotifierItem",
                                        "ContextMenu",
                                        Some(&(0i32, 0i32).to_variant()),
                                        None,
                                        gio::DBusCallFlags::NO_AUTO_START,
                                        1000,
                                    )
                                    .await;
                            });
                        });
                        button.add_controller(gesture);
                        box_.append(&button);
                    }
                    *shown.borrow_mut() = current;
                }
                busy.set(false);
            });
        }
    };
    refresh();
    glib::timeout_add_seconds_local(2, move || {
        refresh();
        glib::ControlFlow::Continue
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_status_notifier_address() {
        assert_eq!(
            split_item(":1.42/StatusNotifierItem"),
            Some((":1.42".into(), "/StatusNotifierItem".into()))
        );
        assert_eq!(
            split_item("org.example.Player"),
            Some(("org.example.Player".into(), "/StatusNotifierItem".into()))
        );
    }
}
