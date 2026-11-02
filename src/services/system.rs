use gtk::{gio, glib, prelude::*};

pub fn show_error(parent: &gtk::Window, message: &str) {
    let dialog = gtk::AlertDialog::builder()
        .message(gettextrs::gettext("Operation unavailable"))
        .detail(message)
        .build();
    dialog.show(Some(parent));
}

/// Fixed executable/arguments only, never a shell or a command from a desktop file.
pub async fn command(args: &[&str]) -> Result<String, String> {
    let args: Vec<_> = args.iter().map(std::ffi::OsStr::new).collect();
    let process = gio::Subprocess::newv(
        &args,
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )
    .map_err(|e| e.to_string())?;
    let (out, err) = process
        .communicate_utf8_future(None)
        .await
        .map_err(|e| e.to_string())?;
    if !process.is_successful() {
        return Err(err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Command failed".into()));
    }
    Ok(out.map(|s| s.to_string()).unwrap_or_default())
}

pub fn open_settings(parent: &gtk::Window) {
    const SETTINGS_APPS: &[&str] = &[
        "org.monolito.Settings.desktop",
        "xfce-settings-manager.desktop",
        "xfce4-settings-manager.desktop",
        "cinnamon-settings.desktop",
        "org.gnome.Settings.desktop",
        "matecc.desktop",
        "systemsettings.desktop",
    ];

    for desktop_id in SETTINGS_APPS {
        if gio::DesktopAppInfo::new(desktop_id).is_some() {
            super::applications::launch(desktop_id, parent);
            return;
        }
    }

    show_error(
        parent,
        &gettextrs::gettext("No supported system settings application was found."),
    );
}

pub async fn proxy(name: &str, path: &str, interface: &str) -> Result<gio::DBusProxy, glib::Error> {
    gio::DBusProxy::for_bus_future(
        gio::BusType::System,
        gio::DBusProxyFlags::DO_NOT_AUTO_START,
        None,
        name,
        path,
        interface,
    )
    .await
}
pub fn property<T: glib::variant::FromVariant>(proxy: &gio::DBusProxy, name: &str) -> Option<T> {
    proxy.name_owner()?;
    proxy.cached_property(name)?.get()
}

pub fn parse_volume(output: &str) -> Option<(f64, bool)> {
    let mut words = output.split_whitespace();
    if words.next()? != "Volume:" {
        return None;
    }
    let value: f64 = words.next()?.parse().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    Some((value, output.contains("[MUTED]")))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wireplumber_output_is_validated() {
        assert_eq!(parse_volume("Volume: 0.42 [MUTED]\n"), Some((0.42, true)));
        assert_eq!(parse_volume("Volume: NaN"), None);
        assert_eq!(parse_volume("Volume: -1"), None);
        assert_eq!(parse_volume("service missing"), None);
    }
}
