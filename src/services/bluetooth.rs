//! BlueZ Bluetooth D-Bus integration.
use gtk::{gio, glib, prelude::*};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDevice {
    pub path: String,
    pub name: String,
    pub connected: bool,
    pub paired: bool,
    pub icon_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BluetoothStatus {
    pub available: bool,
    pub powered: bool,
    pub adapter_path: Option<String>,
    pub devices: Vec<BluetoothDevice>,
}

fn variant_str(v: &glib::Variant) -> Option<String> {
    v.str().map(|s| s.to_string()).or_else(|| v.get::<String>())
}

pub async fn bluez_manager() -> Result<gio::DBusProxy, String> {
    gio::DBusProxy::for_bus_future(
        gio::BusType::System,
        gio::DBusProxyFlags::DO_NOT_AUTO_START,
        None,
        "org.bluez",
        "/",
        "org.freedesktop.DBus.ObjectManager",
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_status() -> Result<BluetoothStatus, String> {
    let manager = bluez_manager().await?;
    if manager.name_owner().is_none() {
        return Ok(BluetoothStatus::default());
    }

    let result = manager
        .call_future("GetManagedObjects", None, gio::DBusCallFlags::NONE, 5_000)
        .await
        .map_err(|e| e.to_string())?;

    if result.n_children() == 0 {
        return Ok(BluetoothStatus {
            available: true,
            ..Default::default()
        });
    }
    let managed_objects = result.child_value(0);

    let mut powered = false;
    let mut adapter_path = None;
    let mut devices = Vec::new();

    // managed_objects is a{oa{sa{sv}}}
    for (obj_path_val, ifaces_val) in managed_objects.iter().filter_map(|entry| {
        (entry.n_children() >= 2).then(|| (entry.child_value(0), entry.child_value(1)))
    }) {
        let Some(path) = variant_str(&obj_path_val) else {
            continue;
        };

        for (iface_name_val, props_val) in ifaces_val.iter().filter_map(|entry| {
            (entry.n_children() >= 2).then(|| (entry.child_value(0), entry.child_value(1)))
        }) {
            let Some(iface_name) = variant_str(&iface_name_val) else {
                continue;
            };

            if iface_name == "org.bluez.Adapter1" {
                if adapter_path.is_none() {
                    adapter_path = Some(path.clone());
                }
                for (prop_name_val, prop_val_wrapper) in props_val.iter().filter_map(|entry| {
                    (entry.n_children() >= 2).then(|| (entry.child_value(0), entry.child_value(1)))
                }) {
                    if let Some(prop_name) = variant_str(&prop_name_val) {
                        if prop_name == "Powered" && prop_val_wrapper.n_children() > 0 {
                            let val = prop_val_wrapper.child_value(0);
                            powered = val.get::<bool>().unwrap_or(false);
                        }
                    }
                }
            } else if iface_name == "org.bluez.Device1" {
                let mut name = String::new();
                let mut connected = false;
                let mut paired = false;
                let mut icon_hint = String::new();

                for (prop_name_val, prop_val_wrapper) in props_val.iter().filter_map(|entry| {
                    (entry.n_children() >= 2).then(|| (entry.child_value(0), entry.child_value(1)))
                }) {
                    if let Some(prop_name) = variant_str(&prop_name_val) {
                        if prop_val_wrapper.n_children() > 0 {
                            let val = prop_val_wrapper.child_value(0);
                            match prop_name.as_str() {
                                "Alias" | "Name" if name.is_empty() => {
                                    if let Some(s) = variant_str(&val) {
                                        name = s;
                                    }
                                }
                                "Connected" => {
                                    connected = val.get::<bool>().unwrap_or(false);
                                }
                                "Paired" => {
                                    paired = val.get::<bool>().unwrap_or(false);
                                }
                                "Icon" => {
                                    if let Some(s) = variant_str(&val) {
                                        icon_hint = s;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if !name.is_empty() {
                    let icon_name = device_icon(&icon_hint).to_string();
                    devices.push(BluetoothDevice {
                        path: path.clone(),
                        name,
                        connected,
                        paired,
                        icon_name,
                    });
                }
            }
        }
    }

    // Sort: connected first, then paired, then alphabetically
    devices.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then_with(|| b.paired.cmp(&a.paired))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let available = adapter_path.is_some();

    Ok(BluetoothStatus {
        available,
        powered,
        adapter_path,
        devices,
    })
}

pub async fn set_powered(adapter_path: &str, powered: bool) -> Result<(), String> {
    let proxy = gio::DBusProxy::for_bus_future(
        gio::BusType::System,
        gio::DBusProxyFlags::DO_NOT_AUTO_START,
        None,
        "org.bluez",
        adapter_path,
        "org.bluez.Adapter1",
    )
    .await
    .map_err(|e| e.to_string())?;

    let param = ("org.bluez.Adapter1", "Powered", powered.to_variant()).to_variant();

    proxy
        .call_future(
            "org.freedesktop.DBus.Properties.Set",
            Some(&param),
            gio::DBusCallFlags::NONE,
            5_000,
        )
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn toggle_connection(device_path: &str, connect: bool) -> Result<(), String> {
    let proxy = gio::DBusProxy::for_bus_future(
        gio::BusType::System,
        gio::DBusProxyFlags::DO_NOT_AUTO_START,
        None,
        "org.bluez",
        device_path,
        "org.bluez.Device1",
    )
    .await
    .map_err(|e| e.to_string())?;

    let method = if connect { "Connect" } else { "Disconnect" };
    proxy
        .call_future(method, None, gio::DBusCallFlags::NONE, 15_000)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn device_icon(icon_hint: &str) -> &'static str {
    match icon_hint {
        "audio-headset" | "audio-headphones" => "audio-headset-symbolic",
        "audio-card" | "audio-speakers" => "audio-speakers-symbolic",
        "input-keyboard" => "input-keyboard-symbolic",
        "input-mouse" | "input-tablet" => "input-mouse-symbolic",
        "input-gaming" => "input-gaming-symbolic",
        "phone" => "phone-symbolic",
        _ => "bluetooth-active-symbolic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_bluetooth_device_icons() {
        assert_eq!(device_icon("audio-headset"), "audio-headset-symbolic");
        assert_eq!(device_icon("input-mouse"), "input-mouse-symbolic");
        assert_eq!(device_icon("unknown"), "bluetooth-active-symbolic");
    }
}
