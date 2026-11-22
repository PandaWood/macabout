//! The distro logo: an icon the distro installs, or a circle in its brand colour.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{Color32, ColorImage};
use image::imageops::{FilterType, resize};
use image::{ImageBuffer, Rgba};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, fontdb::Database};

/// The logo's side, in points.
pub const ICON_SIZE: f32 = 180.0;

/// The fallback circle's colour per distro id.
const BRAND_COLORS: [(&str, Color32); 15] = [
    ("zorin", Color32::from_rgb(0x15, 0xA6, 0xF0)),
    ("ubuntu", Color32::from_rgb(0xE9, 0x54, 0x20)),
    ("debian", Color32::from_rgb(0xA8, 0x00, 0x30)),
    ("fedora", Color32::from_rgb(0x29, 0x41, 0x72)),
    ("linuxmint", Color32::from_rgb(0x87, 0xC0, 0x4D)),
    ("pop", Color32::from_rgb(0x48, 0xB9, 0xC7)),
    ("elementary", Color32::from_rgb(0x64, 0xBA, 0xFF)),
    ("arch", Color32::from_rgb(0x17, 0x93, 0xD1)),
    ("manjaro", Color32::from_rgb(0x35, 0xBF, 0x5C)),
    ("opensuse", Color32::from_rgb(0x73, 0xBA, 0x25)),
    ("kali", Color32::from_rgb(0x26, 0x8B, 0xEE)),
    ("raspbian", Color32::from_rgb(0xC5, 0x1A, 0x4A)),
    ("centos", Color32::from_rgb(0x93, 0x22, 0x79)),
    ("rhel", Color32::from_rgb(0xEE, 0x00, 0x00)),
    ("linux", Color32::from_rgb(0xF6, 0xC5, 0x19)),
];

pub fn brand_color(distro_id: &str) -> Color32 {
    let id = distro_id.to_lowercase();
    BRAND_COLORS
        .iter()
        .find(|(key, _)| *key == id)
        .map_or(Color32::from_rgb(0x60, 0x70, 0x80), |(_, color)| *color)
}

const ICON_DIRS: [&str; 8] = [
    "/usr/share/icons/hicolor/256x256/apps",
    "/usr/share/icons/hicolor/128x128/apps",
    "/usr/share/icons/hicolor/96x96/apps",
    "/usr/share/icons/hicolor/64x64/apps",
    "/usr/share/icons/hicolor/48x48/apps",
    "/usr/share/icons/hicolor/scalable/apps",
    "/usr/share/pixmaps",
    // Some distros drop their logo loose in the icon root (CachyOS ships
    // /usr/share/icons/cachyos.svg). Last, so that themed icons still win.
    "/usr/share/icons",
];

/// The distro's logo: `LOGO=` from os-release first, then the usual names, in each icon
/// directory. PNG and SVG only.
pub fn find_system_icon(distro_id: &str, logo_id: &str) -> Option<PathBuf> {
    let names = [
        logo_id.to_string(),
        "distributor-logo".to_string(),
        format!("distributor-logo-{distro_id}"),
        format!("{distro_id}-logo"),
        distro_id.to_string(),
    ];
    names
        .iter()
        .filter(|name| !name.is_empty())
        .flat_map(|name| {
            ICON_DIRS.iter().flat_map(move |dir| {
                ["png", "svg"].map(|ext| PathBuf::from(format!("{dir}/{name}.{ext}")))
            })
        })
        .find(|path| path.exists())
}

type Premultiplied = ImageBuffer<Rgba<u8>, Vec<u8>>;

/// The logo at `px` by `px` pixels, or `None` if it cannot be read (the circle is drawn
/// instead). An SVG is drawn at twice the size, stretched to fill the square, then scaled
/// down with Lanczos for smooth edges; any other image is scaled to the square with Lanczos.
pub fn render(path: &Path, px: u32, fontdb: Arc<Database>) -> Option<ColorImage> {
    let bytes = std::fs::read(path).ok()?;
    let image = if path.extension().is_some_and(|ext| ext == "svg") {
        render_svg(&bytes, px * 2, fontdb)?
    } else {
        let mut rgba = image::load_from_memory(&bytes).ok()?.into_rgba8();
        for pixel in rgba.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            let scale = |c: u8| ((u16::from(c) * u16::from(a) + 127) / 255) as u8;
            pixel.0 = [scale(r), scale(g), scale(b), a];
        }
        rgba
    };
    // Scaled premultiplied, so transparent edges do not darken.
    let scaled = resize(&image, px, px, FilterType::Lanczos3);
    let side = px as usize;
    Some(ColorImage::from_rgba_premultiplied(
        [side, side],
        scaled.as_raw(),
    ))
}

fn render_svg(bytes: &[u8], px: u32, fontdb: Arc<Database>) -> Option<Premultiplied> {
    let options = usvg::Options {
        fontdb,
        ..Default::default()
    };
    let tree = usvg::Tree::from_data(bytes, &options).ok()?;
    let size = tree.size();
    let mut pixmap = Pixmap::new(px, px)?;
    let transform = Transform::from_scale(px as f32 / size.width(), px as f32 / size.height());
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    ImageBuffer::from_raw(px, px, pixmap.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_colors_by_lowercase_id() {
        assert_eq!(brand_color("Ubuntu"), Color32::from_rgb(0xE9, 0x54, 0x20));
        assert_eq!(
            brand_color("nosuchdistro"),
            Color32::from_rgb(0x60, 0x70, 0x80)
        );
    }

    #[test]
    fn svg_is_stretched_to_the_square() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
            <rect width="200" height="100" fill="red"/></svg>"#;
        let image = render_svg(svg, 40, Arc::new(Database::new()));
        let image = image.map(|i| (i.width(), i.get_pixel(20, 39).0));
        assert_eq!(image, Some((40, [255, 0, 0, 255])));
    }
}
