//! The faces the dialog draws with: the first installed family of [`FONT_CANDIDATES`], else
//! the desktop's `sans-serif` as fontconfig resolves it.

use std::sync::Arc;

use eframe::egui::{FontData, FontDefinitions, FontFamily, FontId};
use resvg::usvg::fontdb::{Database, Family, Query, Stretch, Style, Weight};

use crate::system::run;

/// In order of preference.
const FONT_CANDIDATES: [&str; 7] = [
    "SF Pro Text",
    "SF Pro Display",
    "Helvetica Neue",
    "Inter",
    "Segoe UI",
    "Ubuntu",
    "DejaVu Sans",
];

/// Sizes are given in points at 96 dpi; egui sizes are pixels before display scaling.
const PX_PER_PT: f32 = 96.0 / 72.0;

pub fn regular(pt: f32) -> FontId {
    FontId::new(pt * PX_PER_PT, FontFamily::Name("regular".into()))
}

pub fn bold(pt: f32) -> FontId {
    FontId::new(pt * PX_PER_PT, FontFamily::Name("bold".into()))
}

/// Every installed font, also used for text in SVG logos.
pub fn system_database() -> Database {
    let mut db = Database::new();
    db.load_system_fonts();
    db
}

/// The desktop's sans-serif family, as fontconfig resolves it.
fn sans_serif() -> Option<String> {
    run("fc-match", &["-f", "%{family[0]}", "sans-serif"]).filter(|f| !f.is_empty())
}

/// The first candidate installed, else the desktop's sans-serif.
fn pick_family(db: &Database) -> Option<String> {
    FONT_CANDIDATES
        .iter()
        .find(|name| {
            db.faces()
                .any(|face| face.families.iter().any(|(family, _)| family == *name))
        })
        .map(|name| name.to_string())
        .or_else(sans_serif)
}

fn face(db: &Database, family: &str, weight: Weight) -> Option<FontData> {
    let id = db.query(&Query {
        families: &[Family::Name(family)],
        weight,
        stretch: Stretch::Normal,
        style: Style::Normal,
    })?;
    db.with_face_data(id, |data, index| {
        FontData::from_blob(Arc::new(data.to_vec()), index)
    })
}

/// egui's fonts, with families `regular` and `bold` put first and egui's built-in fonts behind
/// them for anything they lack.
pub fn definitions(db: &Database) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let fallback = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let family = pick_family(db);
    for (role, weight) in [("regular", Weight::NORMAL), ("bold", Weight::BOLD)] {
        let mut chain = Vec::new();
        if let Some(data) = family.as_deref().and_then(|f| face(db, f, weight)) {
            defs.font_data.insert(role.to_string(), Arc::new(data));
            chain.push(role.to_string());
        }
        chain.extend(fallback.iter().cloned());
        defs.families.insert(FontFamily::Name(role.into()), chain);
    }
    defs
}
