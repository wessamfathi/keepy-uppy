//! Menu bar icon styles. Each style has an armed and an off template image
//! (36 px, shown at 18 pt).

use std::io::Cursor;

use serde::{Deserialize, Serialize};
use tray_icon::Icon;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconStyle {
    #[default]
    KeepyUppy,
    WatchfulLid,
    PocketSun,
    SunMoon,
    /// Ring when off, filled circle when armed.
    Plain,
}

impl IconStyle {
    pub const ALL: [IconStyle; 5] = [
        IconStyle::KeepyUppy,
        IconStyle::WatchfulLid,
        IconStyle::PocketSun,
        IconStyle::SunMoon,
        IconStyle::Plain,
    ];

    pub fn label(self) -> &'static str {
        match self {
            IconStyle::KeepyUppy => "Keepy Uppy",
            IconStyle::WatchfulLid => "Watchful Lid",
            IconStyle::PocketSun => "Pocket Sun",
            IconStyle::SunMoon => "Sun and Moon",
            IconStyle::Plain => "Plain",
        }
    }

    /// Template image: macOS tints it to match the menu bar.
    pub fn icon(self, armed: bool) -> Icon {
        let png: &[u8] = match (self, armed) {
            (IconStyle::Plain, _) => return plain(armed),
            (IconStyle::KeepyUppy, true) => include_bytes!("../assets/menubar/keepy-uppy-on.png"),
            (IconStyle::KeepyUppy, false) => include_bytes!("../assets/menubar/keepy-uppy-off.png"),
            (IconStyle::WatchfulLid, true) => {
                include_bytes!("../assets/menubar/watchful-lid-on.png")
            }
            (IconStyle::WatchfulLid, false) => {
                include_bytes!("../assets/menubar/watchful-lid-off.png")
            }
            (IconStyle::PocketSun | IconStyle::SunMoon, true) => {
                include_bytes!("../assets/menubar/pocket-sun-on.png")
            }
            (IconStyle::PocketSun, false) => include_bytes!("../assets/menubar/pocket-sun-off.png"),
            (IconStyle::SunMoon, false) => include_bytes!("../assets/menubar/pocket-moon-off.png"),
        };
        decode(png)
    }
}

/// Embedded assets are 8-bit RGBA PNGs; a bad one is a build mistake.
fn decode(bytes: &[u8]) -> Icon {
    let mut reader = png::Decoder::new(Cursor::new(bytes))
        .read_info()
        .expect("valid embedded PNG");
    let mut buf = vec![
        0;
        reader
            .output_buffer_size()
            .expect("PNG size fits in memory")
    ];
    let info = reader.next_frame(&mut buf).expect("valid embedded PNG");
    assert_eq!(
        info.color_type,
        png::ColorType::Rgba,
        "menu bar PNGs must be RGBA"
    );
    buf.truncate(info.buffer_size());
    Icon::from_rgba(buf, info.width, info.height).expect("valid icon buffer")
}

/// Filled circle when armed, ring when off.
fn plain(armed: bool) -> Icon {
    const SIZE: u32 = 36; // 18pt at 2x
    let c = (SIZE as f32 - 1.0) / 2.0;
    let (outer, inner) = (14.0, if armed { 0.0 } else { 10.0 });
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let r = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            // 1px anti-aliased edges on both radii.
            let a = (outer + 0.5 - r).clamp(0.0, 1.0) * (r - inner + 0.5).clamp(0.0, 1.0);
            rgba.extend_from_slice(&[0, 0, 0, (a * 255.0) as u8]);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("valid icon buffer")
}

#[cfg(test)]
mod tests {
    use super::IconStyle;

    #[test]
    fn every_style_and_state_decodes() {
        for style in IconStyle::ALL {
            for armed in [true, false] {
                style.icon(armed);
            }
        }
    }
}
