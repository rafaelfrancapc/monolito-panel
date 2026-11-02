//! Resolve serialized GIcons, installed icon themes and real window icons.
use gtk::{gdk, gio, glib, prelude::*};

pub fn image(serialized: &str, alternatives: &[&str], size: i32) -> gtk::Image {
    let image = gtk::Image::new();
    image.set_pixel_size(size);
    let theme = gtk::IconTheme::for_display(&image.display());
    if let Ok(icon) = gio::Icon::for_string(serialized) {
        let file_exists = icon
            .downcast_ref::<gio::FileIcon>()
            .is_none_or(|icon| icon.file().path().is_some_and(|path| path.is_file()));
        let paintable = theme.lookup_by_gicon(
            &icon,
            size,
            image.scale_factor(),
            gtk::TextDirection::None,
            gtk::IconLookupFlags::empty(),
        );
        if file_exists
            && paintable.icon_name().as_deref() != Some(std::path::Path::new("image-missing"))
            && paintable.file().is_some()
        {
            image.set_from_gicon(&icon);
            return image;
        }
    }
    let name = alternatives
        .iter()
        .copied()
        .find(|name| theme.has_icon(name))
        .unwrap_or("application-x-executable-symbolic");
    image.set_icon_name(Some(name));
    image
}

pub fn window_image(icon: &crate::platform::WindowIcon, size: i32) -> gtk::Image {
    let bytes = glib::Bytes::from_owned(icon.rgba.clone());
    let texture = gdk::MemoryTexture::new(
        icon.width as i32,
        icon.height as i32,
        gdk::MemoryFormat::R8g8b8a8,
        &bytes,
        icon.width as usize * 4,
    );
    let image = gtk::Image::from_paintable(Some(&texture));
    image.set_pixel_size(size);
    image
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a private X11 display; run under xvfb-run"]
    fn resolves_battery_resources_and_missing_app_icons() {
        gtk::init().unwrap();
        gio::resources_register_include!("panel.gresource").unwrap();
        let theme = gtk::IconTheme::for_display(&gdk::Display::default().unwrap());
        theme.add_resource_path("/org/monolito/Panel/icons");
        for state in [
            crate::services::power::BatteryState::Charging,
            crate::services::power::BatteryState::Discharging,
            crate::services::power::BatteryState::FullyCharged,
        ] {
            for percentage in 0..=100 {
                let name = crate::services::power::battery_icon(percentage as f64, &state);
                assert!(theme.has_icon(name), "Missing battery asset: {name}");
            }
        }
        for name in [
            "start-symbolic",
            "airplane-mode-symbolic",
            "battery-saver-symbolic",
        ] {
            assert!(theme.has_icon(name), "Missing embedded asset: {name}");
        }
        let missing = image("a-nonexistent-application-icon", &[], 24);
        assert_eq!(
            missing.icon_name().as_deref(),
            Some("application-x-executable-symbolic")
        );
        let themed = image("utilities-terminal", &[], 24);
        assert!(themed.gicon().is_some());
        let missing_file = image("/nonexistent/panel-test-icon.svg", &[], 24);
        assert_eq!(
            missing_file.icon_name().as_deref(),
            Some("application-x-executable-symbolic")
        );
    }
}
