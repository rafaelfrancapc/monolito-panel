//! Desktop integration boundary. Wayland requires a separate protocol backend.
mod x11;
use std::sync::mpsc::{Receiver, Sender};
pub use x11::{native_window_id, X11Backend};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub root_height: u32,
}
impl Geometry {
    /// EWMH struts are measured from the root edges, not a monitor's edges.
    /// Interior monitors cannot safely reserve a bottom strip with this protocol.
    pub fn bottom_strut(self, panel_height: u32, reserve: bool) -> [u32; 12] {
        let mut strut = [0; 12];
        if reserve
            && self.x >= 0
            && self.y >= 0
            && self.y as u32 + self.height == self.root_height
            && self.width > 0
        {
            strut[3] = panel_height.min(self.height);
            strut[10] = self.x as u32;
            strut[11] = self.x as u32 + self.width - 1;
        }
        strut
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub class: String,
    pub active: bool,
    pub icon: Option<WindowIcon>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowIcon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl WindowIcon {
    /// EWMH packs dimensions followed by ARGB pixels; never trust client sizes.
    pub fn parse(values: &[u32]) -> Option<Self> {
        let mut rest = values;
        let mut best: Option<(u32, u32, &[u32])> = None;
        while rest.len() >= 2 {
            let (width, height) = (rest[0], rest[1]);
            let Some(end) = (width as usize)
                .checked_mul(height as usize)
                .and_then(|n| n.checked_add(2))
            else {
                break;
            };
            let Some(pixels) = rest.get(2..end) else {
                break;
            };
            if width > 0 && height > 0 && width <= 256 && height <= 256 {
                let score = width.abs_diff(48) + height.abs_diff(48);
                if best.is_none_or(|(w, h, _)| score < w.abs_diff(48) + h.abs_diff(48)) {
                    best = Some((width, height, pixels));
                }
            }
            rest = &rest[end..];
        }
        let (width, height, pixels) = best?;
        let rgba = pixels
            .iter()
            .flat_map(|pixel| {
                let [a, r, g, b] = pixel.to_be_bytes();
                [r, g, b, a]
            })
            .collect();
        Some(Self {
            width,
            height,
            rgba,
        })
    }
}
#[derive(Debug)]
pub enum DesktopEvent {
    Ready(Geometry),
    Attached(Geometry),
    Geometry(Geometry),
    Windows(Vec<WindowInfo>),
    ToggleStart,
    Error(String),
}
#[derive(Debug)]
pub enum DesktopCommand {
    Attach(u32),
    Reposition,
    Activate(u32),
    Minimize(u32),
    Close(u32),
    Stop,
}
pub trait DesktopShellBackend {
    fn start(
        height: u16,
        monitor: Option<usize>,
        reserve: bool,
    ) -> (Sender<DesktopCommand>, Receiver<DesktopEvent>);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_icons_validate_dimensions_and_decode_argb() {
        assert_eq!(
            WindowIcon::parse(&[1, 1, 0x80402010]).unwrap().rgba,
            [64, 32, 16, 128]
        );
        assert!(WindowIcon::parse(&[100, 100, 0]).is_none());
        assert!(WindowIcon::parse(&[u32::MAX, u32::MAX]).is_none());
        assert!(WindowIcon::parse(&[0, 0]).is_none());
        assert!(WindowIcon::parse(&[1, 1, 0xff123456, 256, 256]).is_some());
    }
    #[test]
    fn strut_uses_root_coordinates_and_inclusive_span() {
        let g = Geometry {
            x: 1920,
            y: 0,
            width: 1280,
            height: 1080,
            root_height: 1080,
        };
        let s = g.bottom_strut(48, true);
        assert_eq!((s[3], s[10], s[11]), (48, 1920, 3199));
        assert_eq!(g.bottom_strut(48, false), [0; 12]);
    }
    #[test]
    fn interior_monitor_cannot_reserve_global_space() {
        let g = Geometry {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            root_height: 2160,
        };
        assert_eq!(g.bottom_strut(48, true), [0; 12]);
    }
}
