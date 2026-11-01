//! Menu bar icon: Watchful Lid. Template images (36 px, shown at 18 pt)
//! drawn by scripts/draw-menubar-icons.swift.

use std::io::Cursor;

use tray_icon::Icon;

/// Open eye on a solid lid when armed, closed eye on an outlined lid when off.
pub fn icon(armed: bool) -> Icon {
    decode(if armed {
        include_bytes!("../assets/menubar/watchful-lid-on.png")
    } else {
        include_bytes!("../assets/menubar/watchful-lid-off.png")
    })
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

#[cfg(test)]
mod tests {
    #[test]
    fn both_states_decode() {
        super::icon(true);
        super::icon(false);
    }
}
