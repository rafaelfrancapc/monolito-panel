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
    let popover = gtk::Popover::builder()
        .position(gtk::PositionType::Top)
        .has_arrow(false)
        .autohide(true)
        .child(child)
        .build();
    popover.add_css_class("monolito-popup");
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
