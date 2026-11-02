pub mod calendar;
pub mod quick_settings;
pub mod start_menu;

use gtk::prelude::*;
pub fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(&gettextrs::gettext(text)));
    label.set_xalign(0.0);
    label.add_css_class(class);
    label
}
pub fn popover(child: &impl IsA<gtk::Widget>) -> gtk::Popover {
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .propagate_natural_height(true)
        .propagate_natural_width(true)
        .child(child)
        .build();
    let popover = gtk::Popover::builder()
        .position(gtk::PositionType::Top)
        .has_arrow(false)
        .autohide(true)
        .child(&scroll)
        .build();
    popover.add_css_class("monolito-popup");
    popover.set_offset(0, 8);
    popover.connect_show(move |popup| {
        if let Some(surface) = popup.native().and_then(|native| native.surface()) {
            if let Some(monitor) = popup.display().monitor_at_surface(&surface) {
                scroll.set_max_content_height((monitor.geometry().height() - 128).max(200));
            }
        }
    });
    // SearchEntry consumes Escape; capture it before the focused child.
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = popover.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            if let Some(popup) = weak.upgrade() {
                popup.popdown();
            }
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    popover.add_controller(keys);
    popover
}
