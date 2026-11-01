use crate::services::{
    audio, bluetooth, network, power, system,
};
use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub fn build(parent: &gtk::Window) -> gtk::Popover {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.set_size_request(380, -1);
    content.add_css_class("quick-settings-content");

    // Quick Action Tiles Grid (2 columns)
    let grid = gtk::Grid::builder()
        .column_spacing(8)
        .row_spacing(8)
        .column_homogeneous(true)
        .build();

    // 1. Wi-Fi Tile
    let wifi_btn = gtk::Button::new();
    wifi_btn.add_css_class("quick-tile");
    let wifi_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let wifi_icon = gtk::Image::from_icon_name("network-wireless-symbolic");
    wifi_box.append(&wifi_icon);
    let wifi_labels = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let wifi_title = gtk::Label::new(Some("Wi-Fi"));
    wifi_title.set_xalign(0.0);
    wifi_title.add_css_class("quick-tile-title");
    let wifi_sub = gtk::Label::new(Some(&gettextrs::gettext("Checking…")));
    wifi_sub.set_xalign(0.0);
    wifi_sub.set_ellipsize(gtk::pango::EllipsizeMode::End);
    wifi_sub.set_max_width_chars(12);
    wifi_sub.add_css_class("quick-tile-sub");
    wifi_labels.append(&wifi_title);
    wifi_labels.append(&wifi_sub);
    wifi_labels.set_hexpand(true);
    wifi_box.append(&wifi_labels);
    wifi_btn.set_child(Some(&wifi_box));
    grid.attach(&wifi_btn, 0, 0, 1, 1);

    // 2. Bluetooth Tile
    let bt_btn = gtk::Button::new();
    bt_btn.add_css_class("quick-tile");
    let bt_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let bt_icon = gtk::Image::from_icon_name("bluetooth-active-symbolic");
    bt_box.append(&bt_icon);
    let bt_labels = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let bt_title = gtk::Label::new(Some("Bluetooth"));
    bt_title.set_xalign(0.0);
    bt_title.add_css_class("quick-tile-title");
    let bt_sub = gtk::Label::new(Some(&gettextrs::gettext("Checking…")));
    bt_sub.set_xalign(0.0);
    bt_sub.set_ellipsize(gtk::pango::EllipsizeMode::End);
    bt_sub.set_max_width_chars(12);
    bt_sub.add_css_class("quick-tile-sub");
    bt_labels.append(&bt_title);
    bt_labels.append(&bt_sub);
    bt_labels.set_hexpand(true);
    bt_box.append(&bt_labels);
    bt_btn.set_child(Some(&bt_box));
    grid.attach(&bt_btn, 1, 0, 1, 1);

    // 3. Airplane Mode Tile
    let airplane_btn = gtk::Button::new();
    airplane_btn.add_css_class("quick-tile");
    let airplane_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let airplane_icon = gtk::Image::from_icon_name("airplane-mode-symbolic");
    airplane_box.append(&airplane_icon);
    let airplane_labels = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let airplane_title = gtk::Label::new(Some(&gettextrs::gettext("Airplane mode")));
    airplane_title.set_xalign(0.0);
    airplane_title.add_css_class("quick-tile-title");
    let airplane_sub = gtk::Label::new(Some(&gettextrs::gettext("Off")));
    airplane_sub.set_xalign(0.0);
    airplane_sub.add_css_class("quick-tile-sub");
    airplane_labels.append(&airplane_title);
    airplane_labels.append(&airplane_sub);
    airplane_labels.set_hexpand(true);
    airplane_box.append(&airplane_labels);
    airplane_btn.set_child(Some(&airplane_box));
    grid.attach(&airplane_btn, 0, 1, 1, 1);

    // 4. Battery Saver / Power Mode Tile
    let saver_btn = gtk::Button::new();
    saver_btn.add_css_class("quick-tile");
    let saver_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let saver_icon = gtk::Image::from_icon_name("battery-saver-symbolic");
    saver_box.append(&saver_icon);
    let saver_labels = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let saver_title = gtk::Label::new(Some(&gettextrs::gettext("Battery saver")));
    saver_title.set_xalign(0.0);
    saver_title.add_css_class("quick-tile-title");
    let saver_sub = gtk::Label::new(Some(&gettextrs::gettext("Off")));
    saver_sub.set_xalign(0.0);
    saver_sub.add_css_class("quick-tile-sub");
    saver_labels.append(&saver_title);
    saver_labels.append(&saver_sub);
    saver_labels.set_hexpand(true);
    saver_box.append(&saver_labels);
    saver_btn.set_child(Some(&saver_box));
    grid.attach(&saver_btn, 1, 1, 1, 1);

    content.append(&grid);

    // Expandable details container (e.g. Wi-Fi network list or Bluetooth devices)
    let details_expander = gtk::Box::new(gtk::Orientation::Vertical, 6);
    details_expander.add_css_class("quick-details-box");
    details_expander.set_visible(false);

    let details_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let details_title = gtk::Label::new(None);
    details_title.set_xalign(0.0);
    details_title.set_hexpand(true);
    details_title.add_css_class("heading");
    let details_close = gtk::Button::from_icon_name("window-close-symbolic");
    details_close.add_css_class("icon-button-subtle");
    details_header.append(&details_title);
    details_header.append(&details_close);
    details_expander.append(&details_header);

    let details_list = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let details_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(100)
        .max_content_height(180)
        .propagate_natural_height(true)
        .child(&details_list)
        .build();
    details_expander.append(&details_scroll);
    content.append(&details_expander);

    let close_box = details_expander.clone();
    details_close.connect_clicked(move |_| {
        close_box.set_visible(false);
    });

    // Separator before sliders
    let sep1 = gtk::Separator::new(gtk::Orientation::Horizontal);
    content.append(&sep1);

    // Sliders Section
    // 1. Display Brightness Slider
    let bright_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let bright_icon = gtk::Image::from_icon_name("display-brightness-symbolic");
    bright_row.append(&bright_icon);
    let bright_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    bright_scale.set_hexpand(true);
    bright_scale.add_css_class("fluent-slider");
    bright_row.append(&bright_scale);
    content.append(&bright_row);

    // 2. Output Volume Slider + Devices Selector
    let volume_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mute_btn = gtk::Button::from_icon_name("audio-volume-high-symbolic");
    mute_btn.add_css_class("icon-button-subtle");
    mute_btn.set_tooltip_text(Some(&gettextrs::gettext("Toggle mute")));
    volume_row.append(&mute_btn);

    let volume_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    volume_scale.set_hexpand(true);
    volume_scale.add_css_class("fluent-slider");
    volume_scale.set_tooltip_text(Some(&gettextrs::gettext("Output volume")));
    volume_row.append(&volume_scale);

    let sinks_menu_btn = gtk::Button::from_icon_name("go-next-symbolic");
    sinks_menu_btn.add_css_class("icon-button-subtle");
    sinks_menu_btn.set_tooltip_text(Some(&gettextrs::gettext("Select audio output device")));
    volume_row.append(&sinks_menu_btn);
    content.append(&volume_row);

    // 3. Microphone Row
    let mic_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mic_btn = gtk::Button::from_icon_name("audio-input-microphone-symbolic");
    mic_btn.add_css_class("icon-button-subtle");
    mic_btn.set_tooltip_text(Some(&gettextrs::gettext("Toggle microphone mute")));
    mic_row.append(&mic_btn);

    let mic_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    mic_scale.set_hexpand(true);
    mic_scale.add_css_class("fluent-slider");
    mic_scale.set_tooltip_text(Some(&gettextrs::gettext("Input volume")));
    mic_row.append(&mic_scale);
    content.append(&mic_row);

    // Bottom Footer (Battery Status + Settings Gear)
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    footer.add_css_class("popup-footer");

    let battery_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    battery_box.set_hexpand(true);
    let battery_icon = gtk::Image::from_icon_name("battery-level-100-charged-symbolic");
    let battery_label = gtk::Label::new(None);
    battery_label.set_xalign(0.0);
    battery_label.add_css_class("muted");
    battery_box.append(&battery_icon);
    battery_box.append(&battery_label);
    battery_box.set_visible(false);
    footer.append(&battery_box);

    let settings = gtk::Button::from_icon_name("emblem-system-symbolic");
    settings.set_tooltip_text(Some("Monolito Settings"));
    let parent_settings = parent.clone();
    settings.connect_clicked(move |_| system::open_settings(&parent_settings));
    footer.append(&settings);
    content.append(&footer);

    let popup = super::popover(&content);
    popup.set_halign(gtk::Align::End);

    // State cells
    let updating_audio = Rc::new(Cell::new(false));
    let updating_mic = Rc::new(Cell::new(false));
    let updating_brightness = Rc::new(Cell::new(false));
    let current_wifi_enabled = Rc::new(Cell::new(false));
    let current_airplane_mode = Rc::new(Cell::new(false));
    let current_bt_powered = Rc::new(Cell::new(false));
    let current_bt_adapter: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let saver_enabled = Rc::new(Cell::new(false));

    // Audio volume slider change (debounced)
    let pending_vol_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    {
        let guard = updating_audio.clone();
        let timer = pending_vol_timer.clone();
        let parent_win = parent.clone();
        volume_scale.connect_value_changed(move |scale| {
            if guard.get() || !scale.is_sensitive() {
                return;
            }
            if let Some(src) = timer.borrow_mut().take() {
                src.remove();
            }
            let val = scale.value() / 100.0;
            let timer_ref = timer.clone();
            let p_win = parent_win.clone();
            let src = glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                timer_ref.borrow_mut().take();
                glib::MainContext::default().spawn_local(async move {
                    if let Err(e) = audio::set_master_volume(val).await {
                        system::show_error(&p_win, &e);
                    }
                });
            });
            *timer.borrow_mut() = Some(src);
        });
    }

    // Audio mute toggle
    {
        let parent_win = parent.clone();
        let mute_b = mute_btn.clone();
        let vol_s = volume_scale.clone();
        mute_btn.connect_clicked(move |_| {
            let p_win = parent_win.clone();
            let mb = mute_b.clone();
            let vs = vol_s.clone();
            glib::MainContext::default().spawn_local(async move {
                if let Err(e) = audio::toggle_master_mute().await {
                    system::show_error(&p_win, &e);
                } else if let Ok(vol) = audio::get_master_volume().await {
                    mb.set_icon_name(audio::volume_icon(vol.value, vol.muted));
                    vs.set_sensitive(!vol.muted);
                }
            });
        });
    }

    // Mic mute toggle
    {
        let parent_win = parent.clone();
        let mb = mic_btn.clone();
        let ms = mic_scale.clone();
        mic_btn.connect_clicked(move |_| {
            let p_win = parent_win.clone();
            let m_btn = mb.clone();
            let m_sc = ms.clone();
            glib::MainContext::default().spawn_local(async move {
                if let Err(e) = audio::toggle_input_mute().await {
                    system::show_error(&p_win, &e);
                } else if let Ok(vol) = audio::get_input_volume().await {
                    m_btn.set_icon_name(audio::input_icon(vol.muted));
                    m_sc.set_sensitive(!vol.muted);
                }
            });
        });
    }

    // Brightness scale change (debounced)
    let pending_bright_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    {
        let guard = updating_brightness.clone();
        let timer = pending_bright_timer.clone();
        bright_scale.connect_value_changed(move |scale| {
            if guard.get() || !scale.is_sensitive() {
                return;
            }
            if let Some(src) = timer.borrow_mut().take() {
                src.remove();
            }
            let pct = scale.value().round() as u32;
            let timer_ref = timer.clone();
            let src = glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                timer_ref.borrow_mut().take();
                glib::MainContext::default().spawn_local(async move {
                    let _ = power::set_brightness(pct).await;
                });
            });
            *timer.borrow_mut() = Some(src);
        });
    }

    // Wi-Fi Tile Click: Toggle Wi-Fi or show details
    {
        let wifi_en = current_wifi_enabled.clone();
        let parent_win = parent.clone();
        let d_exp = details_expander.clone();
        let d_title = details_title.clone();
        let d_list = details_list.clone();
        let w_sub = wifi_sub.clone();
        let w_tile = wifi_btn.clone();

        wifi_btn.connect_clicked(move |_| {
            let is_enabled = wifi_en.get();
            if !is_enabled {
                // Turn on
                let w_en = wifi_en.clone();
                let p_win = parent_win.clone();
                let ws = w_sub.clone();
                let wt = w_tile.clone();
                glib::MainContext::default().spawn_local(async move {
                    if let Err(e) = network::set_wifi_enabled(true).await {
                        system::show_error(&p_win, &e);
                    } else {
                        w_en.set(true);
                        wt.add_css_class("active");
                        ws.set_text(&gettextrs::gettext("On"));
                    }
                });
            } else {
                // Show Wi-Fi list in details expander
                d_title.set_text("Wi-Fi");
                while let Some(c) = d_list.first_child() {
                    d_list.remove(&c);
                }
                d_exp.set_visible(true);

                let dl = d_list.clone();
                let de = d_exp.clone();
                glib::MainContext::default().spawn_local(async move {
                    if let Ok(networks) = network::list_wifi_networks().await {
                        for net in networks {
                            let item_btn = gtk::Button::new();
                            item_btn.add_css_class("network-list-item");
                            let ibox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                            let s_icon = gtk::Image::from_icon_name(network::signal_strength_to_icon(net.strength));
                            ibox.append(&s_icon);
                            let s_label = gtk::Label::new(Some(&net.ssid));
                            s_label.set_xalign(0.0);
                            s_label.set_hexpand(true);
                            ibox.append(&s_label);
                            if net.secured {
                                ibox.append(&gtk::Image::from_icon_name("network-wireless-encrypted-symbolic"));
                            }
                            if net.active {
                                ibox.append(&gtk::Image::from_icon_name("object-select-symbolic"));
                            }
                            item_btn.set_child(Some(&ibox));
                            dl.append(&item_btn);
                        }
                    }
                });
            }
        });
    }

    // Bluetooth Tile Click: Toggle Bluetooth or show devices
    {
        let bt_pow = current_bt_powered.clone();
        let adapter_ref = current_bt_adapter.clone();
        let parent_win = parent.clone();
        let d_exp = details_expander.clone();
        let d_title = details_title.clone();
        let d_list = details_list.clone();
        let b_sub = bt_sub.clone();
        let b_tile = bt_btn.clone();

        bt_btn.connect_clicked(move |_| {
            let is_powered = bt_pow.get();
            let adapter = adapter_ref.borrow().clone();
            if let Some(ad) = adapter {
                if !is_powered {
                    let bp = bt_pow.clone();
                    let p_win = parent_win.clone();
                    let bs = b_sub.clone();
                    let bt = b_tile.clone();
                    glib::MainContext::default().spawn_local(async move {
                        if let Err(e) = bluetooth::set_powered(&ad, true).await {
                            system::show_error(&p_win, &e);
                        } else {
                            bp.set(true);
                            bt.add_css_class("active");
                            bs.set_text(&gettextrs::gettext("On"));
                        }
                    });
                } else {
                    // Show Bluetooth devices list in details expander
                    d_title.set_text("Bluetooth");
                    while let Some(c) = d_list.first_child() {
                        d_list.remove(&c);
                    }
                    d_exp.set_visible(true);

                    let dl = d_list.clone();
                    let p_win = parent_win.clone();
                    glib::MainContext::default().spawn_local(async move {
                        if let Ok(status) = bluetooth::get_status().await {
                            for dev in status.devices {
                                let item_btn = gtk::Button::new();
                                item_btn.add_css_class("network-list-item");
                                let ibox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                                let d_icon = gtk::Image::from_icon_name(&dev.icon_name);
                                ibox.append(&d_icon);
                                let d_label = gtk::Label::new(Some(&dev.name));
                                d_label.set_xalign(0.0);
                                d_label.set_hexpand(true);
                                ibox.append(&d_label);

                                let status_lbl = gtk::Label::new(Some(if dev.connected {
                                    &gettextrs::gettext("Connected")
                                } else {
                                    &gettextrs::gettext("Paired")
                                }));
                                status_lbl.add_css_class("muted");
                                ibox.append(&status_lbl);

                                item_btn.set_child(Some(&ibox));

                                let dev_path = dev.path.clone();
                                let is_conn = dev.connected;
                                let pw = p_win.clone();
                                item_btn.connect_clicked(move |_| {
                                    let pw_inner = pw.clone();
                                    let dp = dev_path.clone();
                                    glib::MainContext::default().spawn_local(async move {
                                        let _ = bluetooth::toggle_connection(&dp, !is_conn).await;
                                    });
                                });
                                dl.append(&item_btn);
                            }
                        }
                    });
                }
            }
        });
    }

    // Airplane Mode Click: Toggle
    {
        let air_cell = current_airplane_mode.clone();
        let air_sub_lbl = airplane_sub.clone();
        let air_btn_widget = airplane_btn.clone();
        let parent_win = parent.clone();
        airplane_btn.connect_clicked(move |_| {
            let desired = !air_cell.get();
            let ac = air_cell.clone();
            let asl = air_sub_lbl.clone();
            let abw = air_btn_widget.clone();
            let p_win = parent_win.clone();
            glib::MainContext::default().spawn_local(async move {
                if let Err(e) = network::set_airplane_mode(desired).await {
                    system::show_error(&p_win, &e);
                } else {
                    ac.set(desired);
                    if desired {
                        abw.add_css_class("active");
                        asl.set_text(&gettextrs::gettext("On"));
                    } else {
                        abw.remove_css_class("active");
                        asl.set_text(&gettextrs::gettext("Off"));
                    }
                }
            });
        });
    }

    // Battery Saver Click: Toggle
    {
        let sav_cell = saver_enabled.clone();
        let sav_sub_lbl = saver_sub.clone();
        let sav_btn_widget = saver_btn.clone();
        saver_btn.connect_clicked(move |_| {
            let desired = !sav_cell.get();
            sav_cell.set(desired);
            if desired {
                sav_btn_widget.add_css_class("active");
                sav_sub_lbl.set_text(&gettextrs::gettext("On"));
            } else {
                sav_btn_widget.remove_css_class("active");
                sav_sub_lbl.set_text(&gettextrs::gettext("Off"));
            }
        });
    }

    // Sinks device list expander
    {
        let d_exp = details_expander.clone();
        let d_title = details_title.clone();
        let d_list = details_list.clone();
        sinks_menu_btn.connect_clicked(move |_| {
            d_title.set_text(&gettextrs::gettext("Audio output"));
            while let Some(c) = d_list.first_child() {
                d_list.remove(&c);
            }
            d_exp.set_visible(true);

            let dl = d_list.clone();
            let de = d_exp.clone();
            glib::MainContext::default().spawn_local(async move {
                if let Ok(devices) = audio::list_output_devices().await {
                    for dev in devices {
                        let item_btn = gtk::Button::new();
                        item_btn.add_css_class("network-list-item");
                        let ibox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                        ibox.append(&gtk::Image::from_icon_name("audio-speakers-symbolic"));
                        let d_label = gtk::Label::new(Some(&dev.description));
                        d_label.set_xalign(0.0);
                        d_label.set_hexpand(true);
                        ibox.append(&d_label);
                        if dev.is_default {
                            ibox.append(&gtk::Image::from_icon_name("object-select-symbolic"));
                        }
                        item_btn.set_child(Some(&ibox));

                        let dev_id = dev.id;
                        let de_inner = de.clone();
                        item_btn.connect_clicked(move |_| {
                            let de_close = de_inner.clone();
                            glib::MainContext::default().spawn_local(async move {
                                let _ = audio::set_default_output_device(dev_id).await;
                                de_close.set_visible(false);
                            });
                        });
                        dl.append(&item_btn);
                    }
                }
            });
        });
    }

    // Refresh function triggered on popup show
    let refresh_all: Rc<dyn Fn()> = {
        let (
            wifi_btn, wifi_icon, wifi_sub, wifi_enabled_cell,
            bt_btn, bt_sub, bt_adapter_ref, bt_pow_cell,
            airplane_btn, airplane_sub, air_cell,
            volume_scale, mute_btn, guard_audio,
            mic_scale, mic_btn, guard_mic,
            bright_scale, guard_bright,
            battery_box, battery_icon, battery_label,
        ) = (
            wifi_btn.clone(), wifi_icon.clone(), wifi_sub.clone(), current_wifi_enabled.clone(),
            bt_btn.clone(), bt_sub.clone(), current_bt_adapter.clone(), current_bt_powered.clone(),
            airplane_btn.clone(), airplane_sub.clone(), current_airplane_mode.clone(),
            volume_scale.clone(), mute_btn.clone(), updating_audio.clone(),
            mic_scale.clone(), mic_btn.clone(), updating_mic.clone(),
            bright_scale.clone(), updating_brightness.clone(),
            battery_box.clone(), battery_icon.clone(), battery_label.clone(),
        );

        Rc::new(move || {
            // 1. Audio Output
            let (vs, mb, ga) = (volume_scale.clone(), mute_btn.clone(), guard_audio.clone());
            glib::MainContext::default().spawn_local(async move {
                if let Ok(vol) = audio::get_master_volume().await {
                    ga.set(true);
                    vs.set_value(vol.value * 100.0);
                    vs.set_sensitive(!vol.muted);
                    mb.set_icon_name(audio::volume_icon(vol.value, vol.muted));
                    ga.set(false);
                } else {
                    vs.set_sensitive(false);
                    mb.set_sensitive(false);
                }
            });

            // 2. Audio Input
            let (ms, m_btn, gm) = (mic_scale.clone(), mic_btn.clone(), guard_mic.clone());
            glib::MainContext::default().spawn_local(async move {
                if let Ok(vol) = audio::get_input_volume().await {
                    gm.set(true);
                    ms.set_value(vol.value * 100.0);
                    ms.set_sensitive(!vol.muted);
                    m_btn.set_icon_name(audio::input_icon(vol.muted));
                    gm.set(false);
                } else {
                    ms.set_sensitive(false);
                    m_btn.set_sensitive(false);
                }
            });

            // 3. Brightness
            let (bs, gb) = (bright_scale.clone(), guard_bright.clone());
            glib::MainContext::default().spawn_local(async move {
                if let Ok(info) = power::get_brightness().await {
                    if info.supported {
                        gb.set(true);
                        bs.set_value(f64::from(info.percentage));
                        bs.set_sensitive(true);
                        gb.set(false);
                    } else {
                        bs.set_sensitive(false);
                    }
                }
            });

            // 4. NetworkManager
            let (wb, wi, ws, we_cell, ab, as_lbl, a_cell) = (
                wifi_btn.clone(), wifi_icon.clone(), wifi_sub.clone(), wifi_enabled_cell.clone(),
                airplane_btn.clone(), airplane_sub.clone(), air_cell.clone(),
            );
            glib::MainContext::default().spawn_local(async move {
                if let Ok(status) = network::get_status().await {
                    we_cell.set(status.wifi_enabled);
                    a_cell.set(status.airplane_mode);

                    if status.airplane_mode {
                        ab.add_css_class("active");
                        as_lbl.set_text(&gettextrs::gettext("On"));
                    } else {
                        ab.remove_css_class("active");
                        as_lbl.set_text(&gettextrs::gettext("Off"));
                    }

                    if status.wifi_enabled {
                        wb.add_css_class("active");
                        if let Some(ssid) = status.active_ssid {
                            ws.set_text(&ssid);
                            wi.set_icon_name(network::signal_strength_to_icon(status.signal_strength));
                        } else {
                            ws.set_text(&gettextrs::gettext("Disconnected"));
                            wi.set_icon_name("network-wireless-symbolic");
                        }
                    } else {
                        wb.remove_css_class("active");
                        ws.set_text(&gettextrs::gettext("Off"));
                        wi.set_icon_name("network-wireless-offline-symbolic");
                    }
                } else {
                    ws.set_text(&gettextrs::gettext("Unavailable"));
                }
            });

            // 5. Bluetooth
            let (bb, bs, ba_ref, bp_cell) = (
                bt_btn.clone(), bt_sub.clone(), bt_adapter_ref.clone(), bt_pow_cell.clone(),
            );
            glib::MainContext::default().spawn_local(async move {
                if let Ok(status) = bluetooth::get_status().await {
                    bp_cell.set(status.powered);
                    *ba_ref.borrow_mut() = status.adapter_path;

                    if !status.available {
                        bs.set_text(&gettextrs::gettext("Unavailable"));
                        bb.set_sensitive(false);
                    } else if status.powered {
                        bb.add_css_class("active");
                        if let Some(connected) = status.devices.iter().find(|d| d.connected) {
                            bs.set_text(&connected.name);
                        } else {
                            bs.set_text(&gettextrs::gettext("On"));
                        }
                    } else {
                        bb.remove_css_class("active");
                        bs.set_text(&gettextrs::gettext("Off"));
                    }
                }
            });

            // 6. Battery
            let (bbox, bicon, blabel) = (battery_box.clone(), battery_icon.clone(), battery_label.clone());
            glib::MainContext::default().spawn_local(async move {
                if let Ok(Some(bat)) = power::get_battery().await {
                    bbox.set_visible(true);
                    bicon.set_icon_name(power::battery_icon(bat.percentage, &bat.state));
                    let mut text = format!("{:.0}%", bat.percentage);
                    if bat.state.is_charging() {
                        text.push_str(&format!(" · {}", gettextrs::gettext("Charging")));
                    } else if let Some(secs) = bat.seconds_remaining {
                        text.push_str(&format!(" · {} {}", power::format_duration(secs), gettextrs::gettext("remaining")));
                    }
                    blabel.set_text(&text);
                } else {
                    bbox.set_visible(false);
                }
            });
        })
    };

    let f = refresh_all.clone();
    popup.connect_show(move |_| {
        f();
    });

    popup
}
