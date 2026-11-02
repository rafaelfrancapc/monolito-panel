//! Session and power management via systemd-logind and XFCE fallbacks.
use super::system;
use gtk::{gio, prelude::*};

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
