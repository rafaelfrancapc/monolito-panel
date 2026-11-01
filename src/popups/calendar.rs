use gtk::prelude::*;
pub fn build() -> gtk::Popover {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_size_request(340, -1);
    content.append(&super::label("Calendar", "heading"));
    content.append(&gtk::Calendar::new());
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&super::label("Notifications", "heading"));
    let note = super::label(
        "Notification history is not available. XFCE's notification service remains in control.",
        "muted",
    );
    note.set_wrap(true);
    note.set_max_width_chars(36);
    content.append(&note);
    super::popover(&content)
}
