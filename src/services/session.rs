//! Session and power management via systemd-logind and XFCE fallbacks.
use super::system;
use gtk::{gio, prelude::*};
use std::path::PathBuf;

pub struct Profile {
    pub name: String,
    pub photo: Option<PathBuf>,
}

pub fn profile() -> Profile {
    let user = gtk::glib::user_name().to_string_lossy().into_owned();
    let real = gtk::glib::real_name().to_string_lossy().into_owned();
    let name = if real.is_empty() || real == "Unknown" {
        user.clone()
    } else {
        real
    };
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let photo = home
        .into_iter()
        .flat_map(|home| [home.join(".face"), home.join(".face.icon")])
        .chain(std::iter::once(
            PathBuf::from("/var/lib/AccountsService/icons").join(&user),
        ))
        .find(|path| path.is_file());
    Profile { name, photo }
}

pub async fn log_out() -> Result<(), String> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if desktop.to_ascii_lowercase().contains("cinnamon") {
        system::command(&["cinnamon-session-quit", "--logout"]).await?;
    } else {
        system::command(&["xfce4-session-logout", "--logout"]).await?;
    }
    Ok(())
}

async fn call_logind_method(method: &str) -> Result<(), String> {
    let proxy = match gio::DBusProxy::for_bus_future(
        gio::BusType::System,
        gio::DBusProxyFlags::DO_NOT_AUTO_START,
        None,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await
    {
        Ok(p) => p,
        Err(e) => return Err(e.to_string()),
    };

    let params = (true,).to_variant();
    proxy
        .call_future(method, Some(&params), gio::DBusCallFlags::NONE, 10_000)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn power_off() -> Result<(), String> {
    if call_logind_method("PowerOff").await.is_ok() {
        return Ok(());
    }
    if system::command(&["systemctl", "poweroff"]).await.is_ok() {
        return Ok(());
    }
    system::command(&["xfce4-session-logout", "--halt"]).await?;
    Ok(())
}

pub async fn reboot() -> Result<(), String> {
    if call_logind_method("Reboot").await.is_ok() {
        return Ok(());
    }
    if system::command(&["systemctl", "reboot"]).await.is_ok() {
        return Ok(());
    }
    system::command(&["xfce4-session-logout", "--reboot"]).await?;
    Ok(())
}

pub async fn suspend() -> Result<(), String> {
    if call_logind_method("Suspend").await.is_ok() {
        return Ok(());
    }
    if system::command(&["systemctl", "suspend"]).await.is_ok() {
        return Ok(());
    }
    system::command(&["xfce4-session-logout", "--suspend"]).await?;
    Ok(())
}

pub async fn lock_screen() -> Result<(), String> {
    if system::command(&["loginctl", "lock-session"]).await.is_ok() {
        return Ok(());
    }
    if system::command(&["xflock4"]).await.is_ok() {
        return Ok(());
    }
    if system::command(&["xdg-screensaver", "lock"]).await.is_ok() {
        return Ok(());
    }
    Err("No lock screen command succeeded".into())
}
