//! NetworkManager D-Bus client integration.
use gtk::{gio, prelude::*};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectivityState {
    Unknown,
    None,
    Portal,
    Limited,
    Full,
}

impl ConnectivityState {
    pub fn from_u32(state: u32) -> Self {
        match state {
            4 => ConnectivityState::Full,    // NM_CONNECTIVITY_FULL
            3 => ConnectivityState::Limited, // NM_CONNECTIVITY_LIMITED
            2 => ConnectivityState::Portal,  // NM_CONNECTIVITY_PORTAL
            1 => ConnectivityState::None,    // NM_CONNECTIVITY_NONE
            _ => ConnectivityState::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub strength: u8,
    pub secured: bool,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkStatus {
    pub connected: bool,
    pub connectivity: ConnectivityState,
    pub primary_type: String, // "wifi", "ethernet", "none"
    pub wifi_enabled: bool,
    pub airplane_mode: bool,
    pub active_ssid: Option<String>,
    pub signal_strength: u8,
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self {
            connected: false,
            connectivity: ConnectivityState::None,
            primary_type: "none".into(),
            wifi_enabled: false,
            airplane_mode: false,
            active_ssid: None,
            signal_strength: 0,
        }
    }
}

pub async fn nm_proxy() -> Result<gio::DBusProxy, String> {
    gio::DBusProxy::for_bus_future(
        gio::BusType::System,
        gio::DBusProxyFlags::DO_NOT_AUTO_START,
        None,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.NetworkManager",
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_status() -> Result<NetworkStatus, String> {
    let proxy = nm_proxy().await?;
    if proxy.name_owner().is_none() {
        return Err("NetworkManager is not running".into());
    }

    let wifi_enabled: bool = proxy
        .cached_property("WirelessEnabled")
        .and_then(|v| v.get())
        .unwrap_or(false);

    let wwan_enabled: bool = proxy
        .cached_property("WwanEnabled")
        .and_then(|v| v.get())
        .unwrap_or(false);

    let state: u32 = proxy
        .cached_property("State")
        .and_then(|v| v.get())
        .unwrap_or(0);

    let connectivity: u32 = proxy
        .cached_property("Connectivity")
        .and_then(|v| v.get())
        .unwrap_or(0);

    let primary_type: String = proxy
        .cached_property("PrimaryConnectionType")
        .and_then(|v| v.get())
        .unwrap_or_default();

    let connected = state >= 50; // NM_STATE_CONNECTED_LOCAL or above
    let airplane_mode = !wifi_enabled && !wwan_enabled;

    let mut is_wifi = primary_type == "802-11-wireless";
    let is_ethernet = primary_type == "802-3-ethernet";
    let mut active_ssid = None;
    let mut signal_strength = 0;

    // Discover active Wi-Fi AP if connected to Wi-Fi
    if wifi_enabled {
        if let Ok(networks) = list_wifi_networks_internal(&proxy).await {
            if let Some(active) = networks.iter().find(|n| n.active) {
                is_wifi = true;
                active_ssid = Some(active.ssid.clone());
                signal_strength = active.strength;
            }
        }
    }

    let type_str = if is_wifi {
        "wifi"
    } else if is_ethernet {
        "ethernet"
    } else {
        "none"
    };

    Ok(NetworkStatus {
        connected,
        connectivity: ConnectivityState::from_u32(connectivity),
        primary_type: type_str.into(),
        wifi_enabled,
        airplane_mode,
        active_ssid,
        signal_strength,
    })
}

pub async fn set_wifi_enabled(enabled: bool) -> Result<(), String> {
    let proxy = nm_proxy().await?;
    let parameters = (
        "org.freedesktop.NetworkManager",
        "WirelessEnabled",
        enabled.to_variant(),
    )
        .to_variant();

    proxy
        .call_future(
            "org.freedesktop.DBus.Properties.Set",
            Some(&parameters),
            gio::DBusCallFlags::NONE,
            10_000,
        )
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn set_airplane_mode(enabled: bool) -> Result<(), String> {
    let proxy = nm_proxy().await?;
    let wireless_val = (!enabled).to_variant();
    let p_wifi = (
        "org.freedesktop.NetworkManager",
        "WirelessEnabled",
        wireless_val.clone(),
    )
        .to_variant();
    let p_wwan = (
        "org.freedesktop.NetworkManager",
        "WwanEnabled",
        wireless_val,
    )
        .to_variant();

    let wifi_result = proxy
        .call_future(
            "org.freedesktop.DBus.Properties.Set",
            Some(&p_wifi),
            gio::DBusCallFlags::NONE,
            5_000,
        )
        .await;

    let wwan_result = proxy
        .call_future(
            "org.freedesktop.DBus.Properties.Set",
            Some(&p_wwan),
            gio::DBusCallFlags::NONE,
            5_000,
        )
        .await;

    if wifi_result.is_err() && wwan_result.is_err() {
        return Err("NetworkManager did not accept the airplane mode change".into());
    }
    Ok(())
}

async fn list_wifi_networks_internal(proxy: &gio::DBusProxy) -> Result<Vec<WifiNetwork>, String> {
    let devices_res = proxy
        .call_future("GetDevices", None, gio::DBusCallFlags::NONE, 5_000)
        .await
        .map_err(|e| e.to_string())?;

    if devices_res.n_children() == 0 {
        return Ok(vec![]);
    }
    let child = devices_res.child_value(0);

    let device_paths: Vec<String> = child
        .iter()
        .filter_map(|v| v.str().map(|s| s.to_string()).or_else(|| v.get::<String>()))
        .collect();

    let mut networks = Vec::new();

    for dev_path in device_paths {
        let dev_proxy = match gio::DBusProxy::for_bus_future(
            gio::BusType::System,
            gio::DBusProxyFlags::DO_NOT_AUTO_START,
            None,
            "org.freedesktop.NetworkManager",
            &dev_path,
            "org.freedesktop.NetworkManager.Device",
        )
        .await
        {
            Ok(p) => p,
            Err(_) => continue,
        };

        let dev_type: u32 = dev_proxy
            .cached_property("DeviceType")
            .and_then(|v| v.get())
            .unwrap_or(0);

        // NM_DEVICE_TYPE_WIFI == 2
        if dev_type != 2 {
            continue;
        }

        let wifi_proxy = match gio::DBusProxy::for_bus_future(
            gio::BusType::System,
            gio::DBusProxyFlags::DO_NOT_AUTO_START,
            None,
            "org.freedesktop.NetworkManager",
            &dev_path,
            "org.freedesktop.NetworkManager.Device.Wireless",
        )
        .await
        {
            Ok(p) => p,
            Err(_) => continue,
        };

        let active_ap_path: String = wifi_proxy
            .cached_property("ActiveAccessPoint")
            .and_then(|v| v.str().map(|s| s.to_string()))
            .unwrap_or_default();

        let aps_res = wifi_proxy
            .call_future("GetAccessPoints", None, gio::DBusCallFlags::NONE, 5_000)
            .await;

        let aps_val = match aps_res {
            Ok(v) => v,
            Err(_) => continue,
        };

        if aps_val.n_children() == 0 {
            continue;
        }
        let ap_array = aps_val.child_value(0);

        for ap_val in ap_array.iter() {
            let ap_path = match ap_val
                .str()
                .map(|s| s.to_string())
                .or_else(|| ap_val.get::<String>())
            {
                Some(p) => p,
                None => continue,
            };

            let ap_proxy = match gio::DBusProxy::for_bus_future(
                gio::BusType::System,
                gio::DBusProxyFlags::DO_NOT_AUTO_START,
                None,
                "org.freedesktop.NetworkManager",
                &ap_path,
                "org.freedesktop.NetworkManager.AccessPoint",
            )
            .await
            {
                Ok(p) => p,
                Err(_) => continue,
            };

            let ssid_bytes: Vec<u8> = ap_proxy
                .cached_property("Ssid")
                .and_then(|v| v.get())
                .unwrap_or_default();

            let ssid = decode_ssid(&ssid_bytes);
            if ssid.is_empty() {
                continue;
            }

            let strength: u8 = ap_proxy
                .cached_property("Strength")
                .and_then(|v| v.get())
                .unwrap_or(0);

            let rsn_flags: u32 = ap_proxy
                .cached_property("RsnFlags")
                .and_then(|v| v.get())
                .unwrap_or(0);
            let wpa_flags: u32 = ap_proxy
                .cached_property("WpaFlags")
                .and_then(|v| v.get())
                .unwrap_or(0);
            let flags: u32 = ap_proxy
                .cached_property("Flags")
                .and_then(|v| v.get())
                .unwrap_or(0);

            let secured = rsn_flags > 0 || wpa_flags > 0 || (flags & 0x1) != 0;
            let active = !active_ap_path.is_empty() && active_ap_path == ap_path;

            // Avoid duplicate SSIDs, keep the one with higher strength
            if let Some(existing) = networks
                .iter_mut()
                .find(|n: &&mut WifiNetwork| n.ssid == ssid)
            {
                if strength > existing.strength {
                    existing.strength = strength;
                    existing.secured = secured;
                }
                if active {
                    existing.active = true;
                }
            } else {
                networks.push(WifiNetwork {
                    ssid,
                    strength,
                    secured,
                    active,
                });
            }
        }
    }

    networks.sort_by_key(|network| std::cmp::Reverse(network.strength));
    Ok(networks)
}

pub async fn list_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    let proxy = nm_proxy().await?;
    list_wifi_networks_internal(&proxy).await
}

pub async fn connect_wifi(ssid: &str) -> Result<(), String> {
    if ssid.trim().is_empty() {
        return Err("The Wi-Fi network name is empty".into());
    }
    system_command_wifi_connect(ssid).await
}

async fn system_command_wifi_connect(ssid: &str) -> Result<(), String> {
    super::system::command(&["nmcli", "--wait", "20", "device", "wifi", "connect", ssid]).await?;
    Ok(())
}

pub fn decode_ssid(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    String::from_utf8(bytes.to_vec())
        .unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
        .trim()
        .to_string()
}

pub fn signal_strength_to_icon(strength: u8) -> &'static str {
    match strength {
        75..=100 => "network-wireless-signal-excellent-symbolic",
        50..=74 => "network-wireless-signal-good-symbolic",
        25..=49 => "network-wireless-signal-ok-symbolic",
        _ => "network-wireless-signal-weak-symbolic",
    }
}

pub fn network_status_icon(status: &NetworkStatus) -> &'static str {
    if status.airplane_mode {
        return "airplane-mode-symbolic";
    }
    if !status.connected {
        return "network-offline-symbolic";
    }
    if status.primary_type == "ethernet" {
        return "network-wired-symbolic";
    }
    if status.primary_type == "wifi" {
        return signal_strength_to_icon(status.signal_strength);
    }
    "network-wired-symbolic"
}

pub fn network_status_message(status: &NetworkStatus) -> &'static str {
    if !status.connected || status.connectivity == ConnectivityState::None {
        return "Network: disconnected";
    }
    match status.connectivity {
        ConnectivityState::Full => "Network: connected",
        ConnectivityState::Portal => "Network: sign-in required",
        ConnectivityState::Limited => "Network: limited connectivity",
        ConnectivityState::Unknown => "Network: unavailable or transitioning",
        ConnectivityState::None => "Network: disconnected",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssid_decoding_handles_utf8_and_lossy() {
        assert_eq!(decode_ssid(b"MinhaRede"), "MinhaRede");
        assert_eq!(decode_ssid(b""), "");
        let with_invalid = vec![0xFF, 0xFE, b'a'];
        assert!(!decode_ssid(&with_invalid).is_empty());
    }

    #[test]
    fn signal_strength_maps_to_appropriate_symbolic_icon() {
        assert_eq!(
            signal_strength_to_icon(90),
            "network-wireless-signal-excellent-symbolic"
        );
        assert_eq!(
            signal_strength_to_icon(60),
            "network-wireless-signal-good-symbolic"
        );
        assert_eq!(
            signal_strength_to_icon(35),
            "network-wireless-signal-ok-symbolic"
        );
        assert_eq!(
            signal_strength_to_icon(10),
            "network-wireless-signal-weak-symbolic"
        );
    }

    #[test]
    fn network_status_icon_prioritizes_airplane_and_offline() {
        let mut status = NetworkStatus {
            airplane_mode: true,
            connected: true,
            primary_type: "wifi".into(),
            ..Default::default()
        };
        assert_eq!(network_status_icon(&status), "airplane-mode-symbolic");

        status.airplane_mode = false;
        status.connected = false;
        assert_eq!(network_status_icon(&status), "network-offline-symbolic");

        status.connected = true;
        status.primary_type = "ethernet".into();
        assert_eq!(network_status_icon(&status), "network-wired-symbolic");

        status.primary_type = "wifi".into();
        status.signal_strength = 80;
        assert_eq!(
            network_status_icon(&status),
            "network-wireless-signal-excellent-symbolic"
        );
    }

    #[test]
    fn network_status_message_reflects_connectivity() {
        let mut status = NetworkStatus {
            connected: true,
            connectivity: ConnectivityState::Full,
            ..Default::default()
        };
        assert_eq!(network_status_message(&status), "Network: connected");

        status.connectivity = ConnectivityState::Portal;
        assert_eq!(network_status_message(&status), "Network: sign-in required");

        status.connected = false;
        assert_eq!(network_status_message(&status), "Network: disconnected");
    }
}
