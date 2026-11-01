use gtk::{gio, glib, prelude::*};

#[derive(Clone, Debug)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub wm_class: String,
}
impl Application {
    pub fn image(&self, size: i32) -> gtk::Image {
        let image = gio::Icon::for_string(&self.icon)
            .ok()
            .map(|icon| gtk::Image::from_gicon(&icon))
            .unwrap_or_else(|| gtk::Image::from_icon_name("application-x-executable-symbolic"));
        image.set_pixel_size(size);
        image
    }
    pub fn matches_class(&self, class: &str) -> bool {
        !class.is_empty()
            && ((!self.wm_class.is_empty() && self.wm_class.eq_ignore_ascii_case(class))
                || self
                    .id
                    .trim_end_matches(".desktop")
                    .eq_ignore_ascii_case(class))
    }
}
/// Run off the GTK thread. Return plain metadata, not thread-bound GTK objects.
pub fn installed() -> Vec<Application> {
    let mut apps: Vec<_> = gio::AppInfo::all()
        .into_iter()
        .filter(|a| a.should_show())
        .filter_map(|a| {
            let id = a.id()?.to_string();
            let desktop = gio::DesktopAppInfo::new(&id);
            Some(Application {
                id,
                name: a.display_name().to_string(),
                icon: a
                    .icon()
                    .and_then(|i| i.to_string())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                wm_class: desktop
                    .and_then(|d| d.startup_wm_class())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            })
        })
        .collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

pub fn launch(id: &str, parent: &gtk::Window) {
    let Some(app) = gio::DesktopAppInfo::new(id) else {
        super::system::show_error(parent, "The application is no longer installed.");
        return;
    };
    let parent = parent.clone();
    let context = gtk::prelude::WidgetExt::display(&parent).app_launch_context();
    glib::MainContext::default().spawn_local(async move {
        if let Err(error) = app.launch_uris_future(&[], Some(&context)).await {
            super::system::show_error(&parent, &error.to_string());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_is_metadata_not_window_title() {
        let app = Application {
            id: "org.xfce.Thunar.desktop".into(),
            name: "Files".into(),
            icon: String::new(),
            wm_class: "Thunar".into(),
        };
        assert!(app.matches_class("thunar"));
        assert!(!app.matches_class("Files"));
        assert!(!app.matches_class(""));
    }
}
