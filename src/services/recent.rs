//! XDG Recent Files manager via GtkRecentManager.
use gtk::{gio, glib, prelude::*};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentItem {
    pub name: String,
    pub uri: String,
    pub icon_name: String,
}

pub fn get_recent_files(limit: usize) -> Vec<RecentItem> {
    let Some(manager) = gtk::RecentManager::default() else {
        return Vec::new();
    };

    let mut items: Vec<_> = manager
        .items()
        .into_iter()
        .filter(|info| info.exists())
        .collect();

    // Sort by modified time descending (newest first)
    items.sort_by(|a, b| {
        let ma = a.modified().map(|t| t.to_unix()).unwrap_or(0);
        let mb = b.modified().map(|t| t.to_unix()).unwrap_or(0);
        mb.cmp(&ma)
    });

    items
        .into_iter()
        .take(limit)
        .map(|info| {
            let name = info
                .display_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    info.uri()
                        .split('/')
                        .last()
                        .unwrap_or("document")
                        .to_string()
                });

            let uri = info.uri().to_string();

            let icon_name = info
                .gicon()
                .and_then(|g| g.to_string())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "text-x-generic-symbolic".into());

            RecentItem {
                name,
                uri,
                icon_name,
            }
        })
        .collect()
}

pub fn open_recent_item(uri: &str, parent: &gtk::Window) {
    let parent = parent.clone();
    let uri = uri.to_string();
    glib::MainContext::default().spawn_local(async move {
        let file = gio::File::for_uri(&uri);
        let context = gtk::prelude::WidgetExt::display(&parent).app_launch_context();
        if let Err(e) = gio::AppInfo::launch_default_for_uri_future(&uri, Some(&context)).await {
            super::system::show_error(&parent, &format!("{}: {}", file.uri(), e));
        }
    });
}

