use crate::services::{audio, bluetooth, network, power, system};
use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub fn build(parent: &gtk::Window) -> gtk::Popover {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_size_request(monolito_ui::QUICK_WIDTH, -1);
    content.add_css_class("quick-settings-content");

    let grid = gtk::Grid::builder()
        .column_spacing(16)
        .row_spacing(16)
        .column_homogeneous(true)
        .build();
    let (wifi_tile, wifi_btn, wifi_icon, wifi_sub, wifi_more) =
        radio_tile("Wi-Fi", "network-wireless-symbolic");
    wifi_more.set_tooltip_text(Some(&gettextrs::gettext("Show available networks")));
    let (bt_tile, bt_btn, _, bt_sub, bt_more) =
        radio_tile("Bluetooth", "bluetooth-active-symbolic");
    bt_more.set_tooltip_text(Some(&gettextrs::gettext("Show Bluetooth devices")));
    let (airplane_tile, airplane_btn, _, airplane_sub, airplane_more) =
        radio_tile("Airplane mode", "airplane-mode-symbolic");
    airplane_more.set_visible(false);
    let (saver_tile, saver_btn, _, saver_sub, saver_more) =
        radio_tile("Battery saver", "battery-saver-symbolic");
    saver_more.set_visible(false);
    grid.attach(&wifi_tile, 0, 0, 1, 1);
    grid.attach(&bt_tile, 1, 0, 1, 1);
    grid.attach(&airplane_tile, 0, 1, 1, 1);
    grid.attach(&saver_tile, 1, 1, 1, 1);

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
    settings.set_hexpand(true);
    settings.set_halign(gtk::Align::End);
    settings.set_tooltip_text(Some(&gettextrs::gettext("System settings")));
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
            let src =
                glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
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

    // Microphone volume slider change (debounced)
    let pending_mic_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    {
        let guard = updating_mic.clone();
        let timer = pending_mic_timer.clone();
        let parent_win = parent.clone();
        mic_scale.connect_value_changed(move |scale| {
            if guard.get() || !scale.is_sensitive() {
                return;
            }
            if let Some(source) = timer.borrow_mut().take() {
                source.remove();
            }
            let value = scale.value() / 100.0;
            let timer_ref = timer.clone();
            let parent_win = parent_win.clone();
            let source =
                glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                    timer_ref.borrow_mut().take();
                    glib::MainContext::default().spawn_local(async move {
                        if let Err(error) = audio::set_input_volume(value).await {
                            system::show_error(&parent_win, &error);
                        }
                    });
                });
            *timer.borrow_mut() = Some(source);
        });
    }

    // Brightness scale change (debounced)
    let pending_bright_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    {
        let guard = updating_brightness.clone();
        let timer = pending_bright_timer.clone();
        let parent_win = parent.clone();
        bright_scale.connect_value_changed(move |scale| {
            if guard.get() || !scale.is_sensitive() {
                return;
            }
            if let Some(src) = timer.borrow_mut().take() {
                src.remove();
            }
            let pct = scale.value().round() as u32;
            let timer_ref = timer.clone();
            let parent_win = parent_win.clone();
            let src =
                glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                    timer_ref.borrow_mut().take();
                    glib::MainContext::default().spawn_local(async move {
                        if let Err(error) = power::set_brightness(pct).await {
                            system::show_error(&parent_win, &error);
                        }
                    });
                });
            *timer.borrow_mut() = Some(src);
        });
    }

    // Wi-Fi tile: main area toggles the radio; chevron opens network details.
    {
        let wifi_en = current_wifi_enabled.clone();
        let parent_win = parent.clone();
        let w_sub = wifi_sub.clone();
        let w_tile = wifi_btn.clone();
        let w_more = wifi_more.clone();

        wifi_btn.connect_clicked(move |_| {
            let desired = !wifi_en.get();
            let w_en = wifi_en.clone();
            let p_win = parent_win.clone();
            let ws = w_sub.clone();
            let wt = w_tile.clone();
            let wm = w_more.clone();
            wt.set_sensitive(false);
            glib::MainContext::default().spawn_local(async move {
                wt.set_sensitive(true);
                match network::set_wifi_enabled(desired).await {
                    Ok(()) => {
                        w_en.set(desired);
                        wm.set_sensitive(desired);
                        if desired {
                            wt.add_css_class("active");
                            ws.set_text(&gettextrs::gettext("On"));
                        } else {
                            wt.remove_css_class("active");
                            ws.set_text(&gettextrs::gettext("Off"));
                        }
                    }
                    Err(error) => system::show_error(&p_win, &error),
                }
            });
        });
    }

    {
        let parent_win = parent.clone();
        let d_exp = details_expander.clone();
        let d_title = details_title.clone();
        let d_list = details_list.clone();
        wifi_more.connect_clicked(move |_| {
            d_title.set_text("Wi-Fi");
            while let Some(child) = d_list.first_child() {
                d_list.remove(&child);
            }
            d_exp.set_visible(true);

            let dl = d_list.clone();
            let de = d_exp.clone();
            let p_win = parent_win.clone();
            glib::MainContext::default().spawn_local(async move {
                match network::list_wifi_networks().await {
                    Ok(networks) => {
                        if networks.is_empty() {
                            let empty = gtk::Label::new(Some(&gettextrs::gettext(
                                "No Wi-Fi networks found.",
                            )));
                            empty.add_css_class("muted");
                            empty.set_margin_top(8);
                            empty.set_margin_bottom(8);
                            dl.append(&empty);
                        }
                        for net in networks {
                            let item_btn = gtk::Button::new();
                            item_btn.add_css_class("network-list-item");
                            let ibox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                            let s_icon = gtk::Image::from_icon_name(
                                network::signal_strength_to_icon(net.strength),
                            );
                            ibox.append(&s_icon);
                            let s_label = gtk::Label::new(Some(&net.ssid));
                            s_label.set_xalign(0.0);
                            s_label.set_hexpand(true);
                            ibox.append(&s_label);
                            if net.secured {
                                ibox.append(&gtk::Image::from_icon_name(
                                    "network-wireless-encrypted-symbolic",
                                ));
                            }
                            if net.active {
                                ibox.append(&gtk::Image::from_icon_name("object-select-symbolic"));
                            }
                            item_btn.set_child(Some(&ibox));
                            if net.active {
                                item_btn.set_sensitive(false);
                            } else {
                                let ssid = net.ssid.clone();
                                let dialog_parent = p_win.clone();
                                let details = de.clone();
                                item_btn.connect_clicked(move |button| {
                                    button.set_sensitive(false);
                                    let button = button.clone();
                                    let ssid = ssid.clone();
                                    let dialog_parent = dialog_parent.clone();
                                    let details = details.clone();
                                    glib::MainContext::default().spawn_local(async move {
                                        match network::connect_wifi(&ssid).await {
                                            Ok(()) => details.set_visible(false),
                                            Err(error) => {
                                                button.set_sensitive(true);
                                                system::show_error(&dialog_parent, &error)
                                            }
                                        }
                                    });
                                });
                            }
                            dl.append(&item_btn);
                        }
                    }
                    Err(error) => system::show_error(&p_win, &error),
                }
            });
        });
    }

    // Bluetooth tile: main area toggles the adapter; chevron opens devices.
    {
        let bt_pow = current_bt_powered.clone();
        let adapter_ref = current_bt_adapter.clone();
        let parent_win = parent.clone();
        let b_sub = bt_sub.clone();
        let b_tile = bt_btn.clone();
        let b_more = bt_more.clone();

        bt_btn.connect_clicked(move |_| {
            let adapter = adapter_ref.borrow().clone();
            if let Some(ad) = adapter {
                let desired = !bt_pow.get();
                let bp = bt_pow.clone();
                let p_win = parent_win.clone();
                let bs = b_sub.clone();
                let bt = b_tile.clone();
                let bm = b_more.clone();
                bt.set_sensitive(false);
                glib::MainContext::default().spawn_local(async move {
                    bt.set_sensitive(true);
                    match bluetooth::set_powered(&ad, desired).await {
                        Ok(()) => {
                            bp.set(desired);
                            bm.set_sensitive(desired);
                            if desired {
                                bt.add_css_class("active");
                                bs.set_text(&gettextrs::gettext("On"));
                            } else {
                                bt.remove_css_class("active");
                                bs.set_text(&gettextrs::gettext("Off"));
                            }
                        }
                        Err(error) => system::show_error(&p_win, &error),
                    }
                });
            }
        });
    }

    {
        let parent_win = parent.clone();
        let d_exp = details_expander.clone();
        let d_title = details_title.clone();
        let d_list = details_list.clone();
        bt_more.connect_clicked(move |_| {
            d_title.set_text("Bluetooth");
            while let Some(child) = d_list.first_child() {
                d_list.remove(&child);
            }
            d_exp.set_visible(true);

            let dl = d_list.clone();
            let p_win = parent_win.clone();
            glib::MainContext::default().spawn_local(async move {
                match bluetooth::get_status().await {
                    Ok(status) => {
                        if status.devices.is_empty() {
                            let empty = gtk::Label::new(Some(&gettextrs::gettext(
                                "No paired devices found.",
                            )));
                            empty.add_css_class("muted");
                            empty.set_margin_top(8);
                            empty.set_margin_bottom(8);
                            dl.append(&empty);
                        } else {
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

                                let status_text = if dev.connected {
                                    gettextrs::gettext("Connected")
                                } else {
                                    gettextrs::gettext("Paired")
                                };
                                let status_lbl = gtk::Label::new(Some(&status_text));
                                status_lbl.add_css_class("muted");
                                ibox.append(&status_lbl);

                                item_btn.set_child(Some(&ibox));

                                let dev_path = dev.path.clone();
                                let is_conn = dev.connected;
                                let pw = p_win.clone();
                                item_btn.connect_clicked(move |_| {
                                    let dp = dev_path.clone();
                                    let dialog_parent = pw.clone();
                                    glib::MainContext::default().spawn_local(async move {
                                        if let Err(error) =
                                            bluetooth::toggle_connection(&dp, !is_conn).await
                                        {
                                            system::show_error(&dialog_parent, &error);
                                        }
                                    });
                                });
                                dl.append(&item_btn);
                            }
                        }
                    }
                    Err(error) => system::show_error(&p_win, &error),
                }
            });
        });
    }

    // Airplane Mode Click: Toggle
    {
        let air_cell = current_airplane_mode.clone();
        let air_sub_lbl = airplane_sub.clone();
        let air_btn_widget = airplane_btn.clone();
        let wifi_enabled = current_wifi_enabled.clone();
        let wifi_button = wifi_btn.clone();
        let wifi_details = wifi_more.clone();
        let wifi_subtitle = wifi_sub.clone();
        let parent_win = parent.clone();
        airplane_btn.connect_clicked(move |_| {
            let desired = !air_cell.get();
            let ac = air_cell.clone();
            let asl = air_sub_lbl.clone();
            let abw = air_btn_widget.clone();
            let we = wifi_enabled.clone();
            let wb = wifi_button.clone();
            let wd = wifi_details.clone();
            let ws = wifi_subtitle.clone();
            let p_win = parent_win.clone();
            glib::MainContext::default().spawn_local(async move {
                if let Err(e) = network::set_airplane_mode(desired).await {
                    system::show_error(&p_win, &e);
                } else {
                    ac.set(desired);
                    if desired {
                        abw.add_css_class("active");
                        asl.set_text(&gettextrs::gettext("On"));
                        we.set(false);
                        wb.remove_css_class("active");
                        wd.set_sensitive(false);
                        ws.set_text(&gettextrs::gettext("Off"));
                    } else {
                        abw.remove_css_class("active");
                        asl.set_text(&gettextrs::gettext("Off"));
                        we.set(true);
                        wb.add_css_class("active");
                        wd.set_sensitive(true);
                        ws.set_text(&gettextrs::gettext("On"));
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
        let parent_win = parent.clone();
        saver_btn.connect_clicked(move |_| {
            let desired = !sav_cell.get();
            let enabled = sav_cell.clone();
            let subtitle = sav_sub_lbl.clone();
            let button = sav_btn_widget.clone();
            let dialog_parent = parent_win.clone();
            button.set_sensitive(false);
            glib::MainContext::default().spawn_local(async move {
                button.set_sensitive(true);
                match power::set_power_saver(desired).await {
                    Ok(()) => {
                        enabled.set(desired);
                        if desired {
                            button.add_css_class("active");
                            subtitle.set_text(&gettextrs::gettext("On"));
                        } else {
                            button.remove_css_class("active");
                            subtitle.set_text(&gettextrs::gettext("Off"));
                        }
                    }
                    Err(error) => system::show_error(&dialog_parent, &error),
                }
            });
        });
    }

    // Sinks device list expander
    {
        let d_exp = details_expander.clone();
        let d_title = details_title.clone();
        let d_list = details_list.clone();
        let parent_win = parent.clone();
        sinks_menu_btn.connect_clicked(move |_| {
            d_title.set_text(&gettextrs::gettext("Audio output"));
            while let Some(c) = d_list.first_child() {
                d_list.remove(&c);
            }
            d_exp.set_visible(true);

            let dl = d_list.clone();
            let de = d_exp.clone();
            let p_win = parent_win.clone();
            glib::MainContext::default().spawn_local(async move {
                match audio::list_output_devices().await {
                    Ok(devices) => {
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
                            let dialog_parent = p_win.clone();
                            item_btn.connect_clicked(move |_| {
                                let de_close = de_inner.clone();
                                let dialog_parent = dialog_parent.clone();
                                glib::MainContext::default().spawn_local(async move {
                                    match audio::set_default_output_device(dev_id).await {
                                        Ok(()) => de_close.set_visible(false),
                                        Err(error) => system::show_error(&dialog_parent, &error),
                                    }
                                });
                            });
                            dl.append(&item_btn);
                        }
                    }
                    Err(error) => system::show_error(&p_win, &error),
                }
            });
        });
    }

    // Refresh function triggered on popup show
    let refresh_all: Rc<dyn Fn()> = {
        let (
            wifi_btn,
            wifi_more,
            wifi_icon,
            wifi_sub,
            wifi_enabled_cell,
            bt_btn,
            bt_more,
            bt_sub,
            bt_adapter_ref,
            bt_pow_cell,
            airplane_btn,
            airplane_sub,
            air_cell,
            volume_scale,
            mute_btn,
            guard_audio,
            mic_scale,
            mic_btn,
            guard_mic,
            bright_scale,
            guard_bright,
            battery_box,
            battery_icon,
            battery_label,
            saver_btn,
            saver_sub,
            saver_enabled,
        ) = (
            wifi_btn.clone(),
            wifi_more.clone(),
            wifi_icon.clone(),
            wifi_sub.clone(),
            current_wifi_enabled.clone(),
            bt_btn.clone(),
            bt_more.clone(),
            bt_sub.clone(),
            current_bt_adapter.clone(),
            current_bt_powered.clone(),
            airplane_btn.clone(),
            airplane_sub.clone(),
            current_airplane_mode.clone(),
            volume_scale.clone(),
            mute_btn.clone(),
            updating_audio.clone(),
            mic_scale.clone(),
            mic_btn.clone(),
            updating_mic.clone(),
            bright_scale.clone(),
            updating_brightness.clone(),
            battery_box.clone(),
            battery_icon.clone(),
            battery_label.clone(),
            saver_btn.clone(),
            saver_sub.clone(),
            saver_enabled.clone(),
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
            let (wb, wm, wi, ws, we_cell, ab, as_lbl, a_cell) = (
                wifi_btn.clone(),
                wifi_more.clone(),
                wifi_icon.clone(),
                wifi_sub.clone(),
                wifi_enabled_cell.clone(),
                airplane_btn.clone(),
                airplane_sub.clone(),
                air_cell.clone(),
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
                        wm.set_sensitive(true);
                        if let Some(ssid) = status.active_ssid {
                            ws.set_text(&ssid);
                            wi.set_icon_name(Some(network::signal_strength_to_icon(
                                status.signal_strength,
                            )));
                        } else {
                            ws.set_text(&gettextrs::gettext("Disconnected"));
                            wi.set_icon_name(Some("network-wireless-symbolic"));
                        }
                    } else {
                        wb.remove_css_class("active");
                        wm.set_sensitive(false);
                        ws.set_text(&gettextrs::gettext("Off"));
                        wi.set_icon_name(Some("network-wireless-offline-symbolic"));
                    }
                } else {
                    ws.set_text(&gettextrs::gettext("Unavailable"));
                    as_lbl.set_text(&gettextrs::gettext("Unavailable"));
                    wb.set_sensitive(false);
                    wm.set_sensitive(false);
                    ab.set_sensitive(false);
                }
            });

            // 5. Bluetooth
            let (bb, bm, bs, ba_ref, bp_cell) = (
                bt_btn.clone(),
                bt_more.clone(),
                bt_sub.clone(),
                bt_adapter_ref.clone(),
                bt_pow_cell.clone(),
            );
            glib::MainContext::default().spawn_local(async move {
                if let Ok(status) = bluetooth::get_status().await {
                    bp_cell.set(status.powered);
                    *ba_ref.borrow_mut() = status.adapter_path;

                    if !status.available {
                        bs.set_text(&gettextrs::gettext("Unavailable"));
                        bb.set_sensitive(false);
                        bm.set_sensitive(false);
                    } else if status.powered {
                        bb.set_sensitive(true);
                        bm.set_sensitive(true);
                        bb.add_css_class("active");
                        if let Some(connected) = status.devices.iter().find(|d| d.connected) {
                            bs.set_text(&connected.name);
                        } else {
                            bs.set_text(&gettextrs::gettext("On"));
                        }
                    } else {
                        bb.set_sensitive(true);
                        bm.set_sensitive(false);
                        bb.remove_css_class("active");
                        bs.set_text(&gettextrs::gettext("Off"));
                    }
                } else {
                    bs.set_text(&gettextrs::gettext("Unavailable"));
                    bb.set_sensitive(false);
                    bm.set_sensitive(false);
                }
            });

            // 6. Battery
            let (bbox, bicon, blabel) = (
                battery_box.clone(),
                battery_icon.clone(),
                battery_label.clone(),
            );
            glib::MainContext::default().spawn_local(async move {
                if let Ok(Some(bat)) = power::get_battery().await {
                    bbox.set_visible(true);
                    bicon.set_icon_name(Some(power::battery_icon(bat.percentage, &bat.state)));
                    let mut text = format!("{:.0}%", bat.percentage);
                    if bat.state.is_charging() {
                        text.push_str(&format!(" · {}", gettextrs::gettext("Charging")));
                    } else if let Some(secs) = bat.seconds_remaining {
                        text.push_str(&format!(
                            " · {} {}",
                            power::format_duration(secs),
                            gettextrs::gettext("remaining")
                        ));
                    }
                    blabel.set_text(&text);
                } else {
                    bbox.set_visible(false);
                }
            });

            // 7. Power profile
            let (button, subtitle, enabled) =
                (saver_btn.clone(), saver_sub.clone(), saver_enabled.clone());
            glib::MainContext::default().spawn_local(async move {
                match power::power_saver_enabled().await {
                    Ok(active) => {
                        enabled.set(active);
                        button.set_sensitive(true);
                        if active {
                            button.add_css_class("active");
                            subtitle.set_text(&gettextrs::gettext("On"));
                        } else {
                            button.remove_css_class("active");
                            subtitle.set_text(&gettextrs::gettext("Off"));
                        }
                    }
                    Err(_) => {
                        button.remove_css_class("active");
                        button.set_sensitive(false);
                        subtitle.set_text(&gettextrs::gettext("Unavailable"));
                    }
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

fn radio_tile(
    title: &str,
    icon: &str,
) -> (gtk::Box, gtk::Button, gtk::Image, gtk::Label, gtk::Button) {
    let tile = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let button = gtk::Button::new();
    button.add_css_class("quick-tile");
    button.set_hexpand(true);
    button.set_tooltip_text(Some(&gettextrs::gettext(title)));
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(20);
    button.set_child(Some(&image));
    let more = gtk::Button::from_icon_name("go-next-symbolic");
    more.add_css_class("quick-tile");
    more.add_css_class("quick-tile-more");
    more.set_sensitive(false);
    controls.append(&button);
    controls.append(&more);
    tile.append(&controls);
    let label = gtk::Label::new(Some(&gettextrs::gettext(title)));
    label.add_css_class("quick-tile-title");
    tile.append(&label);
    let status = gtk::Label::new(Some(&gettextrs::gettext("Checking…")));
    status.add_css_class("quick-tile-sub");
    status.set_ellipsize(gtk::pango::EllipsizeMode::End);
    status.set_max_width_chars(18);
    tile.append(&status);
    (tile, button, image, status, more)
}
