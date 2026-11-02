use gtk::{gio, glib, prelude::*};
use std::{io::Write, os::unix::fs::PermissionsExt};

#[derive(Clone, Debug)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub wm_class: String,
}
impl Application {
    pub fn image(&self, size: i32) -> gtk::Image {
        super::icons::image(
            &self.icon,
            &[
                self.id.trim_end_matches(".desktop"),
                &self.wm_class.to_lowercase(),
                "application-x-executable-symbolic",
            ],
            size,
        )
    }
    pub fn matches_class(&self, class: &str) -> bool {
        !class.is_empty()
            && ((!self.wm_class.is_empty() && self.wm_class.eq_ignore_ascii_case(class))
                || self
                    .id
                    .trim_end_matches(".desktop")
                    .eq_ignore_ascii_case(class)
                || self
                    .id
                    .trim_end_matches(".desktop")
                    .rsplit('.')
                    .next()
                    .is_some_and(|id| id.eq_ignore_ascii_case(class)))
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

pub fn create_desktop_shortcut(id: &str) -> Result<(), String> {
    let app = gio::DesktopAppInfo::new(id).ok_or("Application is no longer installed")?;
    let source = app.filename().ok_or("Application has no desktop file")?;
    let desktop = glib::user_special_dir(glib::UserDirectory::Desktop)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Desktop")))
        .ok_or("Desktop directory is unavailable")?;
    if !desktop.is_dir() {
        return Err(format!(
            "Desktop directory does not exist: {}",
            desktop.display()
        ));
    }
    let name = source.file_name().ok_or("Invalid desktop file name")?;
    let destination = desktop.join(name);
    let contents = std::fs::read(source).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|e| e.to_string())?;
    file.write_all(&contents).map_err(|e| e.to_string())?;
    file.set_permissions(std::fs::Permissions::from_mode(0o755))
        .map_err(|e| e.to_string())?;
    let info = gio::FileInfo::new();
    info.set_attribute_boolean("metadata::trusted", true);
    let _ = gio::File::for_path(destination).set_attributes_from_info(
        &info,
        gio::FileQueryInfoFlags::NONE,
        gio::Cancellable::NONE,
    );
    Ok(())
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
