mod app;
mod config;
mod platform;
mod popups;
mod services;

use gtk::prelude::*;
use gtk::{gdk, glib};

fn main() -> glib::ExitCode {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    let locale = std::env::var_os("MONOLITO_LOCALE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/locale")
        });
    let _ = gettextrs::bindtextdomain("monolito-panel", locale);
    let _ = gettextrs::textdomain("monolito-panel");
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        println!("Monolito Panel\n  --dev           Do not reserve desktop space\n  --theme=light   Override appearance for this run\n  --theme=dark\nX11 only. Config: $XDG_CONFIG_HOME/monolito/{{panel,appearance}}.toml");
        return glib::ExitCode::SUCCESS;
    }
    // A dock needs EWMH; opening an ordinary Wayland window is not support.
    gdk::set_allowed_backends("x11");
    let application = gtk::Application::builder()
        .application_id("org.monolito.Panel")
        .build();
    application.connect_activate(app::activate);
    // Custom options are parsed above/in app; do not pass them to GApplication.
    application.run_with_args(&["monolito-panel"])
}
