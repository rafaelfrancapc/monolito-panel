use gtk::prelude::*;

pub fn build() -> gtk::Popover {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_size_request(360, -1);
    content.add_css_class("calendar-popup-content");

    // Header with current localized day/month
    let now = chrono::Local::now();
    let date_str = now.format("%A, %e de %B").to_string();
    let date_label = gtk::Label::new(Some(&date_str));
    date_label.set_xalign(0.0);
    date_label.add_css_class("calendar-date-header");
    content.append(&date_label);

    // Notifications section
    let notif_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let notif_title = super::label("Notifications", "heading");
    notif_title.set_hexpand(true);
    notif_header.append(&notif_title);

    let clear_btn = gtk::Button::with_label(&gettextrs::gettext("Clear all"));
    clear_btn.add_css_class("subtle-link-button");
    notif_header.append(&clear_btn);
    content.append(&notif_header);

    let empty_notif_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    empty_notif_box.add_css_class("notifications-container");
    let notif_icon = gtk::Image::from_icon_name("preferences-system-notifications-symbolic");
    notif_icon.set_pixel_size(32);
    notif_icon.add_css_class("muted");
    empty_notif_box.append(&notif_icon);

    let note = super::label("No new notifications", "muted");
    note.set_justify(gtk::Justification::Center);
    empty_notif_box.append(&note);
    content.append(&empty_notif_box);

    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    // Calendar widget
    let calendar = gtk::Calendar::new();
    calendar.add_css_class("fluent-calendar");
    content.append(&calendar);

    super::popover(&content)
}
