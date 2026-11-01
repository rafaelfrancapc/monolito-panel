use crate::services::system;
use gtk::{gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub fn build(parent: &gtk::Window) -> gtk::Popover {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_size_request(360, -1);
    content.append(&super::label("Quick settings", "heading"));
    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(12)
        .column_homogeneous(true)
        .build();
    let wifi = gtk::Button::with_label("Wi-Fi");
    wifi.add_css_class("quick-tile");
    wifi.set_sensitive(false);
    let bluetooth = gtk::Button::with_label("Bluetooth");
    bluetooth.add_css_class("quick-tile");
    bluetooth.set_sensitive(false);
    grid.attach(&wifi, 0, 0, 1, 1);
    grid.attach(&bluetooth, 1, 0, 1, 1);
    content.append(&grid);
    let network = super::label("Checking system services…", "muted");
    network.set_wrap(true);
    content.append(&network);
    let bt_status = super::label("Bluetooth controls are planned.", "muted");
    content.append(&bt_status);
    let volume_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mute = gtk::Button::from_icon_name("audio-volume-high-symbolic");
    mute.set_tooltip_text(Some(&gettextrs::gettext("Toggle mute")));
    mute.set_sensitive(false);
    volume_row.append(&mute);
    let volume = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    volume.set_hexpand(true);
    volume.set_sensitive(false);
    volume.set_tooltip_text(Some(&gettextrs::gettext("Output volume")));
    volume_row.append(&volume);
    content.append(&volume_row);
    let audio_status = super::label("Checking audio…", "muted");
    content.append(&audio_status);
    let brightness = super::label("Brightness: no backend in this milestone.", "muted");
    brightness.set_wrap(true);
    content.append(&brightness);
    let power = super::label("", "muted");
    power.set_visible(false);
    content.append(&power);
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    footer.add_css_class("popup-footer");
    let note = super::label("Experimental system integration", "muted");
    note.set_hexpand(true);
    footer.append(&note);
    let settings = gtk::Button::from_icon_name("emblem-system-symbolic");
    settings.set_tooltip_text(Some("Monolito Settings"));
    let parent_settings = parent.clone();
    settings.connect_clicked(move |_| system::open_settings(&parent_settings));
    footer.append(&settings);
    content.append(&footer);
    let popup = super::popover(&content);
    popup.set_halign(gtk::Align::End);

    let nm: Rc<RefCell<Option<gio::DBusProxy>>> = Rc::new(RefCell::new(None));
    let enabled = Rc::new(Cell::new(false));
    let updating = Rc::new(Cell::new(false));
    let refresh_audio: Rc<dyn Fn()> = {
        let (volume, mute, audio_status, updating) = (
            volume.clone(),
            mute.clone(),
            audio_status.clone(),
            updating.clone(),
        );
        Rc::new(move || {
            let (volume, mute, status, updating) = (
                volume.clone(),
                mute.clone(),
                audio_status.clone(),
                updating.clone(),
            );
            glib::MainContext::default().spawn_local(async move {
                match system::command(&["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"])
                    .await
                    .ok()
                    .and_then(|v| system::parse_volume(&v))
                {
                    Some((value, muted)) => {
                        updating.set(true);
                        volume.set_value((value * 100.0).min(100.0));
                        updating.set(false);
                        volume.set_sensitive(true);
                        mute.set_sensitive(true);
                        mute.set_icon_name(if muted {
                            "audio-volume-muted-symbolic"
                        } else {
                            "audio-volume-high-symbolic"
                        });
                        status.set_text(&format!(
                            "{}: {:.0}%{}",
                            gettextrs::gettext("Output volume"),
                            value * 100.0,
                            if muted { " · mute" } else { "" }
                        ));
                    }
                    None => {
                        volume.set_sensitive(false);
                        mute.set_sensitive(false);
                        status.set_text(&gettextrs::gettext(
                            "Audio unavailable (WirePlumber/wpctl).",
                        ));
                    }
                }
            });
        })
    };
    let pending_volume: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let (guard, pending, refresh, parent_volume) = (
        updating.clone(),
        pending_volume.clone(),
        refresh_audio.clone(),
        parent.clone(),
    );
    volume.connect_value_changed(move |scale| {
        if guard.get() || !scale.is_sensitive() {
            return;
        }
        if let Some(source) = pending.borrow_mut().take() {
            source.remove();
        }
        let (pending, refresh, parent, value) = (
            pending.clone(),
            refresh.clone(),
            parent_volume.clone(),
            scale.value() / 100.0,
        );
        let source =
            glib::timeout_add_local_once(std::time::Duration::from_millis(180), move || {
                pending.borrow_mut().take();
                glib::MainContext::default().spawn_local(async move {
                    if let Err(e) = system::command(&[
                        "wpctl",
                        "set-volume",
                        "-l",
                        "1.0",
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{value:.2}"),
                    ])
                    .await
                    {
                        system::show_error(&parent, &e);
                    }
                    refresh();
                });
            });
        *pending_volume.borrow_mut() = Some(source);
    });
    let (refresh, parent_mute) = (refresh_audio.clone(), parent.clone());
    mute.connect_clicked(move |button| {
        button.set_sensitive(false);
        let (refresh, parent) = (refresh.clone(), parent_mute.clone());
        glib::MainContext::default().spawn_local(async move {
            if let Err(e) =
                system::command(&["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]).await
            {
                system::show_error(&parent, &e);
            }
            refresh();
        });
    });
    let (nm_toggle, enabled_toggle, parent_wifi) = (nm.clone(), enabled.clone(), parent.clone());
    wifi.connect_clicked(move |button| {
        let Some(proxy) = nm_toggle.borrow().clone() else {
            return;
        };
        let desired = !enabled_toggle.get();
        let (button, enabled, parent) =
            (button.clone(), enabled_toggle.clone(), parent_wifi.clone());
        button.set_sensitive(false);
        glib::MainContext::default().spawn_local(async move {
            let parameters = (
                "org.freedesktop.NetworkManager",
                "WirelessEnabled",
                desired.to_variant(),
            )
                .to_variant();
            match proxy
                .call_future(
                    "org.freedesktop.DBus.Properties.Set",
                    Some(&parameters),
                    gio::DBusCallFlags::NONE,
                    10_000,
                )
                .await
            {
                Ok(_) => {
                    enabled.set(desired);
                    button.set_label(if desired {
                        "Wi-Fi · on"
                    } else {
                        "Wi-Fi · off"
                    });
                }
                Err(e) => system::show_error(&parent, &e.to_string()),
            }
            button.set_sensitive(true);
        });
    });
    popup.connect_show(move |_| {
        refresh_audio();
        let (wifi, network, nm, enabled, bt_status, power) = (
            wifi.clone(),
            network.clone(),
            nm.clone(),
            enabled.clone(),
            bt_status.clone(),
            power.clone(),
        );
        glib::MainContext::default().spawn_local(async move {
            wifi.set_sensitive(false);
            power.set_visible(false);
            match system::proxy(
                "org.freedesktop.NetworkManager",
                "/org/freedesktop/NetworkManager",
                "org.freedesktop.NetworkManager",
            )
            .await
            {
                Ok(proxy) if proxy.name_owner().is_some() => {
                    if let Some(wireless) = system::property::<bool>(&proxy, "WirelessEnabled") {
                        enabled.set(wireless);
                        wifi.set_label(if wireless {
                            "Wi-Fi · on"
                        } else {
                            "Wi-Fi · off"
                        });
                        wifi.set_sensitive(true);
                    }
                    let state: u32 = system::property(&proxy, "State").unwrap_or(0);
                    network.set_text(&gettextrs::gettext(match state {
                        70 => "Network: connected",
                        50 | 60 => "Network: limited connectivity",
                        20 => "Network: disconnected",
                        _ => "Network: unavailable or transitioning",
                    }));
                    *nm.borrow_mut() = Some(proxy);
                }
                _ => {
                    network.set_text(&gettextrs::gettext("NetworkManager is unavailable."));
                    *nm.borrow_mut() = None;
                }
            }
            let bluez = system::proxy("org.bluez", "/", "org.freedesktop.DBus.ObjectManager").await;
            bt_status.set_text(&gettextrs::gettext(
                if bluez.ok().and_then(|p| p.name_owner()).is_some() {
                    "BlueZ detected · controls are planned."
                } else {
                    "Bluetooth unavailable · controls are planned."
                },
            ));
            if let Ok(proxy) = system::proxy(
                "org.freedesktop.UPower",
                "/org/freedesktop/UPower/devices/DisplayDevice",
                "org.freedesktop.UPower.Device",
            )
            .await
            {
                let present = system::property::<bool>(&proxy, "IsPresent").unwrap_or(false);
                let kind = system::property::<u32>(&proxy, "Type").unwrap_or(0);
                if present && kind == 2 {
                    if let Some(percent) =
                        system::property::<f64>(&proxy, "Percentage").filter(|v| v.is_finite())
                    {
                        power
                            .set_text(&format!("{}: {percent:.0}%", gettextrs::gettext("Battery")));
                        power.set_visible(true);
                    }
                } else {
                    power.set_visible(false);
                }
            }
        });
    });
    popup
}
