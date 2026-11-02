use super::*;
use anyhow::{Context, Result};
use std::{collections::HashSet, sync::mpsc, thread, time::Duration};
use x11rb::{
    connection::Connection,
    protocol::{
        randr::{ConnectionExt as _, NotifyMask},
        xproto::{ConnectionExt as _, *},
        Event,
    },
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

x11rb::atom_manager! {
    Atoms: AtomsCookie {
        _NET_WM_WINDOW_TYPE, _NET_WM_WINDOW_TYPE_DOCK,
        _NET_WM_WINDOW_TYPE_DESKTOP, _NET_WM_STRUT, _NET_WM_STRUT_PARTIAL,
        _NET_CLIENT_LIST, _NET_ACTIVE_WINDOW, _NET_CLOSE_WINDOW, _NET_WM_NAME, UTF8_STRING,
        _NET_WM_STATE, _NET_WM_STATE_SKIP_TASKBAR, _NET_WM_STATE_STICKY,
        _NET_WM_DESKTOP, _NET_CURRENT_DESKTOP, _NET_WM_STATE_ABOVE,
        WM_CLASS, WM_NAME, WM_CHANGE_STATE, _NET_WM_ICON,
    }
}

pub struct X11Backend;
impl DesktopShellBackend for X11Backend {
    fn start(
        height: u16,
        monitor: Option<usize>,
        reserve: bool,
    ) -> (mpsc::Sender<DesktopCommand>, mpsc::Receiver<DesktopEvent>) {
        let (tx, commands) = mpsc::channel();
        let (events, rx) = mpsc::channel();
        thread::spawn(move || {
            if let Err(e) = Worker::run(height, monitor, reserve, commands, &events) {
                let _ = events.send(DesktopEvent::Error(format!("X11: {e:#}")));
            }
        });
        (tx, rx)
    }
}
struct Worker {
    conn: RustConnection,
    root: u32,
    atoms: Atoms,
    dock: Option<u32>,
    height: u16,
    reserve: bool,
    monitor: Option<usize>,
    watched: HashSet<u32>,
}
impl Worker {
    fn run(
        height: u16,
        monitor: Option<usize>,
        reserve: bool,
        commands: Receiver<DesktopCommand>,
        events: &Sender<DesktopEvent>,
    ) -> Result<()> {
        let (conn, screen) = x11rb::connect(None).context("No accessible X11 display")?;
        let root = conn.setup().roots[screen].root;
        let atoms = Atoms::new(&conn)?.reply()?;
        conn.change_window_attributes(
            root,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
        )?
        .check()?;
        // RandR is optional. Older servers fall back to root geometry.
        let _ = conn.randr_select_input(
            root,
            NotifyMask::SCREEN_CHANGE | NotifyMask::CRTC_CHANGE | NotifyMask::OUTPUT_CHANGE,
        );
        let mut worker = Self {
            conn,
            root,
            atoms,
            dock: None,
            height,
            reserve,
            monitor,
            watched: HashSet::new(),
        };
        events.send(DesktopEvent::Ready(worker.geometry()?))?;
        worker.publish_windows(events)?;
        loop {
            match commands.recv_timeout(Duration::from_millis(80)) {
                Ok(DesktopCommand::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(command) => {
                    let result = worker.command(command, events);
                    if let Err(e) = result {
                        let _ = events.send(DesktopEvent::Error(e.to_string()));
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            let mut windows_changed = false;
            let mut geometry_changed = false;
            while let Some(event) = worker.conn.poll_for_event()? {
                match event {
                    Event::PropertyNotify(_) | Event::DestroyNotify(_) => windows_changed = true,
                    Event::RandrScreenChangeNotify(_) | Event::RandrNotify(_) => {
                        geometry_changed = true
                    }
                    Event::ConfigureNotify(e) if e.window == root => geometry_changed = true,
                    // A closing window can race with a read. X errors are not fatal to the shell.
                    Event::Error(_) => {}
                    _ => {}
                }
            }
            if geometry_changed {
                let geometry = worker.geometry()?;
                if worker.dock.is_some() {
                    worker.configure_dock(geometry)?;
                }
                events.send(DesktopEvent::Geometry(geometry))?;
            }
            if windows_changed {
                worker.publish_windows(events)?;
            }
        }
        // Release our reservation if stopping while the GTK window still exists.
        if let Some(dock) = worker.dock {
            let _ = worker
                .conn
                .delete_property(dock, worker.atoms._NET_WM_STRUT_PARTIAL);
            let _ = worker
                .conn
                .delete_property(dock, worker.atoms._NET_WM_STRUT);
            let _ = worker.conn.flush();
        }
        Ok(())
    }
    fn geometry(&self) -> Result<Geometry> {
        let root = self.conn.get_geometry(self.root)?.reply()?;
        let fallback = Geometry {
            x: 0,
            y: 0,
            width: root.width.into(),
            height: root.height.into(),
            root_height: root.height.into(),
        };
        let monitors = match self.conn.randr_get_monitors(self.root, true)?.reply() {
            Ok(reply) => reply.monitors,
            Err(_) => return Ok(fallback),
        };
        let selected = self
            .monitor
            .and_then(|i| monitors.get(i))
            .or_else(|| monitors.iter().find(|m| m.primary))
            .or_else(|| monitors.first());
        Ok(selected
            .map(|m| Geometry {
                x: m.x.into(),
                y: m.y.into(),
                width: m.width.into(),
                height: m.height.into(),
                root_height: root.height.into(),
            })
            .unwrap_or(fallback))
    }
    fn configure_dock(&self, geometry: Geometry) -> Result<()> {
        let dock = self.dock.context("Dock is not attached")?;
        let a = &self.atoms;
        self.conn.change_property32(
            PropMode::REPLACE,
            dock,
            a._NET_WM_WINDOW_TYPE,
            AtomEnum::ATOM,
            &[a._NET_WM_WINDOW_TYPE_DOCK],
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            dock,
            a._NET_WM_STATE,
            AtomEnum::ATOM,
            &[
                a._NET_WM_STATE_ABOVE,
                a._NET_WM_STATE_STICKY,
                a._NET_WM_STATE_SKIP_TASKBAR,
            ],
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            dock,
            a._NET_WM_DESKTOP,
            AtomEnum::CARDINAL,
            &[u32::MAX],
        )?;
        let strut = geometry.bottom_strut(self.height.into(), self.reserve);
        self.conn.change_property32(
            PropMode::REPLACE,
            dock,
            a._NET_WM_STRUT_PARTIAL,
            AtomEnum::CARDINAL,
            &strut,
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            dock,
            a._NET_WM_STRUT,
            AtomEnum::CARDINAL,
            &strut[..4],
        )?;
        self.conn.configure_window(
            dock,
            &ConfigureWindowAux::new()
                .x(geometry.x)
                .y(geometry.y + geometry.height as i32 - i32::from(self.height))
                .width(geometry.width)
                .height(u32::from(self.height))
                .stack_mode(StackMode::ABOVE),
        )?;
        self.conn.flush()?;
        Ok(())
    }
    fn command(&mut self, command: DesktopCommand, events: &Sender<DesktopEvent>) -> Result<()> {
        if std::env::var_os("MONOLITO_TRACE").is_some() {
            eprintln!("X11 command: {command:?}");
        }
        match command {
            DesktopCommand::Attach(id) => {
                self.dock = Some(id);
                let geometry = self.geometry()?;
                self.configure_dock(geometry)?;
                // Round-trip before allowing GTK to map the dock.
                self.conn.get_input_focus()?.reply()?;
                events.send(DesktopEvent::Attached(geometry))?;
            }
            DesktopCommand::Reposition => self.configure_dock(self.geometry()?)?,
            DesktopCommand::Activate(window) => {
                self.send_message(
                    window,
                    self.atoms._NET_ACTIVE_WINDOW,
                    [2, x11rb::CURRENT_TIME, 0, 0, 0],
                )?;
            }
            DesktopCommand::Minimize(window) => {
                self.send_message(window, self.atoms.WM_CHANGE_STATE, [3, 0, 0, 0, 0])?
            }
            DesktopCommand::Close(window) => {
                self.send_message(
                    window,
                    self.atoms._NET_CLOSE_WINDOW,
                    [x11rb::CURRENT_TIME, 2, 0, 0, 0],
                )?;
            }
            DesktopCommand::Stop => {}
        }
        Ok(())
    }
    fn send_message(&self, window: u32, atom: u32, data: [u32; 5]) -> Result<()> {
        let event = ClientMessageEvent::new(32, window, atom, data);
        self.conn.send_event(
            false,
            self.root,
            EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT,
            event,
        )?;
        self.conn.flush()?;
        Ok(())
    }
    fn property(&self, window: u32, atom: u32) -> Vec<u32> {
        self.conn
            .get_property(false, window, atom, AtomEnum::ANY, 0, 4096)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().map(|v| v.collect()))
            .unwrap_or_default()
    }
    fn text_property(&self, window: u32, atom: u32) -> Vec<u8> {
        self.conn
            .get_property(false, window, atom, AtomEnum::ANY, 0, 4096)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.value)
            .unwrap_or_default()
    }
    fn publish_windows(&mut self, events: &Sender<DesktopEvent>) -> Result<()> {
        let clients = self.property(self.root, self.atoms._NET_CLIENT_LIST);
        let active = self
            .property(self.root, self.atoms._NET_ACTIVE_WINDOW)
            .first()
            .copied();
        let desktop = self
            .property(self.root, self.atoms._NET_CURRENT_DESKTOP)
            .first()
            .copied()
            .unwrap_or(0);
        let mut windows = vec![];
        for &id in &clients {
            if Some(id) == self.dock {
                continue;
            }
            if self.watched.insert(id) {
                let _ = self.conn.change_window_attributes(
                    id,
                    &ChangeWindowAttributesAux::new()
                        .event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
                );
            }
            let state = self.property(id, self.atoms._NET_WM_STATE);
            let types = self.property(id, self.atoms._NET_WM_WINDOW_TYPE);
            let wd = self
                .property(id, self.atoms._NET_WM_DESKTOP)
                .first()
                .copied()
                .unwrap_or(desktop);
            if state.contains(&self.atoms._NET_WM_STATE_SKIP_TASKBAR)
                || types.contains(&self.atoms._NET_WM_WINDOW_TYPE_DOCK)
                || types.contains(&self.atoms._NET_WM_WINDOW_TYPE_DESKTOP)
                || (wd != u32::MAX && wd != desktop)
            {
                continue;
            }
            let title_bytes = self.text_property(id, self.atoms._NET_WM_NAME);
            let title = if title_bytes.is_empty() {
                self.text_property(id, self.atoms.WM_NAME)
            } else {
                title_bytes
            };
            let class_bytes = self.text_property(id, self.atoms.WM_CLASS);
            let parts: Vec<_> = class_bytes
                .split(|&b| b == 0)
                .filter(|p| !p.is_empty())
                .collect();
            let class = parts
                .get(1)
                .or_else(|| parts.first())
                .map(|p| String::from_utf8_lossy(p).into_owned())
                .unwrap_or_default();
            windows.push(WindowInfo {
                id,
                title: String::from_utf8_lossy(&title).into_owned(),
                class,
                active: active == Some(id),
                icon: self
                    .conn
                    .get_property(
                        false,
                        id,
                        self.atoms._NET_WM_ICON,
                        AtomEnum::CARDINAL,
                        0,
                        262144,
                    )
                    .ok()
                    .and_then(|cookie| cookie.reply().ok())
                    .and_then(|reply| {
                        reply
                            .value32()
                            .and_then(|values| WindowIcon::parse(&values.collect::<Vec<_>>()))
                    }),
            });
        }
        self.watched.retain(|id| clients.contains(id));
        self.conn.flush()?;
        events.send(DesktopEvent::Windows(windows))?;
        Ok(())
    }
}

/// GTK4/X11 bridge. Kept here so widgets do not depend on X11 surface types.
pub fn native_window_id(window: &gtk::Window) -> Option<u32> {
    use gtk::prelude::*;
    window
        .surface()
        .and_then(|surface| surface.downcast::<gdk4_x11::X11Surface>().ok())
        .map(|surface| surface.xid() as u32)
}
