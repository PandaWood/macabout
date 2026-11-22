//! The "About This Computer" dialog, drawn with egui.
//!
//! `Layout` places every piece by hand, each label being its text plus 2 pixels of padding on
//! each side, so the window is laid out the same on every desktop. Sizes are pixels at 96 dpi,
//! which egui scales with the display.

mod fonts;
mod icon;
mod theme;

use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{
    self, Color32, Context, CursorIcon, FontId, Galley, IconData, Id, Painter, Pos2, RawInput,
    Rect, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2, ViewportBuilder, pos2,
    vec2,
};
use eframe::epaint::text::{FontsView, LayoutJob};
use resvg::usvg::fontdb::Database;

use crate::format::Summary;
use theme::Palette;

const TITLE: &str = "About This Computer";
const LEFT_W: f32 = 240.0;
const COPY_TIP: &str = "Copy to clipboard";
const COPIED_TIP: &str = "Copied!";
const COPIED_SECONDS: f64 = 1.5;
const BUTTON: f32 = 26.0;
/// Padding around a label's text, per side.
const LABEL_INSET: f32 = 2.0;

/// What the dialog shows.
struct Content {
    summary: Summary,
    /// "Version 17.1", or `None` when the distro has no version (rolling releases).
    version: Option<String>,
    copy_text: String,
}

impl Content {
    fn new(summary: Summary) -> Self {
        let version = Some(&summary.os_version)
            .filter(|v| !v.is_empty())
            .map(|v| format!("Version {v}"));
        Content {
            version,
            copy_text: summary.copy_text(),
            summary,
        }
    }
}

/// Laid-out text.
struct Text(Arc<Galley>);

impl Text {
    fn new(
        fonts: &mut FontsView<'_>,
        text: &str,
        font: FontId,
        color: Color32,
        wrap: Option<f32>,
    ) -> Self {
        let wrap = wrap.unwrap_or(f32::INFINITY);
        Text(fonts.layout_job(LayoutJob::simple(text.to_string(), font, color, wrap)))
    }

    /// In whole pixels.
    fn size(&self) -> Vec2 {
        let size = self.0.size();
        vec2(size.x.ceil(), size.y.ceil())
    }

    fn paint(&self, painter: &Painter, top_left: Pos2) {
        painter.galley(top_left, self.0.clone(), Color32::PLACEHOLDER);
    }
}

/// A label: its text and the top-left corner of its box.
struct Label {
    text: Text,
    at: Pos2,
}

fn label_size(text: &Text) -> Vec2 {
    text.size() + Vec2::splat(2.0 * LABEL_INSET)
}

/// Where everything goes.
struct Layout {
    size: Vec2,
    icon: Rect,
    labels: Vec<Label>,
    button: Rect,
}

impl Layout {
    fn new(ctx: &Context, content: &Content, palette: &Palette) -> Self {
        ctx.fonts_mut(|f| {
            let mut text = |s: &str, font, color, wrap| Text::new(f, s, font, color, wrap);
            let summary = &content.summary;
            let title = text(&summary.os_name, fonts::bold(24.0), palette.fg, None);
            let version = content
                .version
                .as_deref()
                .map(|v| text(v, fonts::regular(12.0), palette.subtle, None));
            let model = Some(summary.model.as_str())
                .filter(|m| !m.is_empty())
                .map(|m| text(m, fonts::regular(12.0), palette.fg, None));
            let rows: Vec<_> = summary
                .rows()
                .iter()
                .map(|(label, value)| {
                    (
                        text(label, fonts::regular(11.0), palette.subtle, None),
                        text(value, fonts::regular(11.0), palette.fg, Some(520.0)),
                    )
                })
                .collect();
            let footer = format!("macabout v{}", env!("CARGO_PKG_VERSION"));
            let footer = text(&footer, fonts::regular(8.0), palette.ver, None);

            // Right column: title, version, model, then the label/value grid.
            let x = 16.0 + LEFT_W;
            let mut labels = Vec::new();
            let mut y = 16.0 + 20.0;
            let mut right_w: f32 = 0.0;
            let mut stack = |text: Text, y: &mut f32, after: f32| {
                let size = label_size(&text);
                right_w = right_w.max(size.x);
                labels.push(Label {
                    text,
                    at: pos2(x, *y),
                });
                *y += size.y + after;
            };
            stack(title, &mut y, 4.0);
            if let Some(version) = version {
                stack(version, &mut y, 2.0);
            }
            if let Some(model) = model {
                stack(model, &mut y, 14.0);
            }
            y += 8.0;

            let widest = |pick: fn(&(Text, Text)) -> &Text| {
                rows.iter()
                    .map(|r| label_size(pick(r)).x)
                    .fold(0.0, f32::max)
            };
            let label_w = widest(|r| &r.0);
            let value_w = widest(|r| &r.1);
            right_w = right_w.max(label_w + 12.0 + value_w);
            for (label, value) in rows {
                let (ls, vs) = (label_size(&label), label_size(&value));
                let cell_h = ls.y.max(vs.y);
                let centred = |h: f32| y + 4.0 + ((cell_h - h) / 2.0).floor();
                labels.push(Label {
                    at: pos2(x + label_w - ls.x, centred(ls.y)),
                    text: label,
                });
                labels.push(Label {
                    at: pos2(x + label_w + 12.0, centred(vs.y)),
                    text: value,
                });
                y += cell_h + 8.0;
            }

            let main_h = y - 16.0;
            let size = vec2(16.0 + LEFT_W + right_w + 72.0 + 16.0, 16.0 + main_h + 82.0);
            let icon_at = pos2(
                16.0 + LEFT_W / 2.0 - icon::ICON_SIZE / 2.0,
                16.0 + (main_h / 2.0).round() - icon::ICON_SIZE / 2.0,
            );
            labels.push(Label {
                at: (size - label_size(&footer) - vec2(8.0, 6.0)).to_pos2(),
                text: footer,
            });
            Layout {
                size,
                icon: Rect::from_min_size(icon_at, Vec2::splat(icon::ICON_SIZE)),
                labels,
                button: Rect::from_min_size(
                    (size - vec2(12.0, 44.0) - Vec2::splat(BUTTON)).to_pos2(),
                    Vec2::splat(BUTTON),
                ),
            }
        })
    }
}

enum Logo {
    /// The image file, and the texture made from it at some pixel density.
    Image(std::path::PathBuf, Option<(f32, TextureHandle)>),
    /// The brand-coloured circle with the distro's initial.
    Circle,
}

struct Dialog {
    content: Content,
    palette: &'static Palette,
    fontdb: Arc<Database>,
    logo: Logo,
    button: CopyButton,
}

impl Dialog {
    fn text(&self, ctx: &Context, text: &str, font: FontId, color: Color32) -> Text {
        ctx.fonts_mut(|f| Text::new(f, text, font, color, None))
    }

    fn paint_logo(&mut self, ui: &egui::Ui, rect: Rect) {
        let ppp = ui.ctx().pixels_per_point();
        if let Logo::Image(path, texture) = &mut self.logo
            && texture.as_ref().is_none_or(|(at, _)| *at != ppp)
        {
            // Drawn at the display's density, so the logo stays sharp when scaled.
            let px = (icon::ICON_SIZE * ppp).round() as u32;
            match icon::render(path, px, self.fontdb.clone()) {
                Some(image) => {
                    let handle = ui.ctx().load_texture("logo", image, TextureOptions::LINEAR);
                    *texture = Some((ppp, handle));
                }
                None => self.logo = Logo::Circle,
            }
        }
        let painter = ui.painter();
        match &self.logo {
            Logo::Image(_, Some((_, texture))) => {
                let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
                painter.image(texture.id(), rect, uv, Color32::WHITE);
            }
            Logo::Image(_, None) => {}
            Logo::Circle => {
                let pad = (icon::ICON_SIZE / 18.0).floor();
                let color = icon::brand_color(&self.content.summary.distro_id);
                painter.circle_filled(rect.center(), icon::ICON_SIZE / 2.0 - pad, color);
                if let Some(first) = self.content.summary.os_name.chars().next() {
                    let initial: String = first.to_uppercase().collect();
                    let font = fonts::bold((icon::ICON_SIZE / 3.0).floor());
                    let text = self.text(ui.ctx(), &initial, font, Color32::WHITE);
                    // Centred on the glyph's ink, not its line box.
                    let ink = text.0.mesh_bounds.center().to_vec2();
                    text.paint(painter, rect.center() - ink);
                }
            }
        }
    }

    /// The copy button: two overlapping squares, outlined in the text colour while hovered.
    fn paint_button(&self, painter: &Painter, rect: Rect) {
        let p = self.palette;
        let color = if self.button.hovered { p.fg } else { p.subtle };
        let at = |x0: f32, y0: f32, x1: f32, y1: f32| {
            Rect::from_min_max(rect.min + vec2(x0, y0), rect.min + vec2(x1, y1))
        };
        let stroke = Stroke::new(2.0, color);
        painter.rect_stroke(at(8.0, 4.0, 21.0, 17.0), 0.0, stroke, StrokeKind::Middle);
        let front = at(4.0, 8.0, 17.0, 21.0);
        painter.rect_filled(front, 0.0, p.bg);
        painter.rect_stroke(front, 0.0, stroke, StrokeKind::Middle);
    }

    /// The tooltip above the button, kept inside the window.
    fn paint_tip(&self, painter: &Painter, button: Rect, window: Vec2, tip: &str) {
        let p = self.palette;
        let text = self.text(painter.ctx(), tip, fonts::regular(9.0), p.bg);
        let size = text.size() + vec2(12.0, 4.0);
        let x = (button.min.x + ((BUTTON - size.x) / 2.0).floor()).clamp(0.0, window.x - size.x);
        let rect = Rect::from_min_size(pos2(x, button.min.y - size.y - 4.0), size);
        painter.rect_filled(rect, 0.0, p.fg);
        text.paint(painter, rect.min + vec2(6.0, 2.0));
    }

    /// Hover and click on the copy button.
    fn interact(&mut self, ui: &egui::Ui, button: Rect) {
        let ctx = ui.ctx();
        let hovered = ui
            .interact(button, Id::unique("copy"), Sense::click())
            .hovered();
        if hovered {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
        let pressed = hovered && ui.input(|i| i.pointer.primary_pressed());
        let now = ui.input(|i| i.time);
        if self.button.update(hovered, pressed, now) {
            ctx.copy_text(self.content.copy_text.clone());
        }
        if let Some(until) = self.button.copied_until {
            ctx.request_repaint_after(Duration::from_secs_f64((until - now).max(0.0)));
        }
    }
}

/// The copy button's state: hovering shows "Copy to clipboard"; a click copies and shows
/// "Copied!" for 1.5 seconds, whatever the pointer does meanwhile; then the tip shows again
/// only if the pointer is still over the button.
#[derive(Default)]
struct CopyButton {
    hovered: bool,
    tip: Option<&'static str>,
    copied_until: Option<f64>,
}

impl CopyButton {
    /// One frame. Returns whether to copy now.
    fn update(&mut self, hovered: bool, pressed: bool, now: f64) -> bool {
        if hovered != self.hovered && self.copied_until.is_none() {
            self.tip = hovered.then_some(COPY_TIP);
        }
        self.hovered = hovered;
        if pressed {
            self.tip = Some(COPIED_TIP);
            self.copied_until = Some(now + COPIED_SECONDS);
        } else if self.copied_until.is_some_and(|until| now >= until) {
            self.copied_until = None;
            self.tip = hovered.then_some(COPY_TIP);
        }
        pressed
    }
}

impl eframe::App for Dialog {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let layout = Layout::new(ui.ctx(), &self.content, self.palette);
        let window = ui.max_rect();
        ui.painter().rect_filled(window, 0.0, self.palette.bg);
        self.interact(ui, layout.button);
        self.paint_logo(ui, layout.icon);
        let painter = ui.painter();
        for label in &layout.labels {
            label
                .text
                .paint(painter, label.at + Vec2::splat(LABEL_INSET));
        }
        self.paint_button(painter, layout.button);
        if let Some(tip) = self.button.tip {
            self.paint_tip(painter, layout.button, window.size(), tip);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.palette.bg.to_normalized_gamma_f32()
    }
}

/// Open the window and wait for it to close.
pub fn show_dialog(summary: Summary) -> Result<(), String> {
    let content = Content::new(summary);
    let palette = if theme::detect_dark_mode() {
        &theme::DARK
    } else {
        &theme::LIGHT
    };
    let fontdb = Arc::new(fonts::system_database());
    let definitions = fonts::definitions(&fontdb);
    let new_context = || {
        let ctx = Context::default();
        ctx.set_fonts(definitions.clone());
        ctx.add_font_provider(Arc::new(
            egui_system_font_provider::SystemFontProvider::new(),
        ));
        ctx
    };
    // Lay the dialog out once, on a context of its own, to open the window at its final size.
    let mut size = Vec2::ZERO;
    let mut output = new_context().run_ui(RawInput::default(), |ui| {
        size = Layout::new(ui.ctx(), &content, palette).size;
    });
    output.textures_delta.clear();
    let logo = match icon::find_system_icon(&content.summary.distro_id, &content.summary.logo_id) {
        Some(path) => Logo::Image(path, None),
        None => Logo::Circle,
    };
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(TITLE)
            .with_app_id("macabout")
            .with_inner_size(size)
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(IconData::default()),
        centered: true,
        system_font_fallback: false,
        ..Default::default()
    };
    let dialog = Dialog {
        content,
        palette,
        fontdb,
        logo,
        button: CopyButton::default(),
    };
    eframe::run_native_ext(
        "macabout",
        options,
        Some(new_context()),
        Box::new(|_| Ok(Box::new(dialog))),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_shows_and_hides_the_tip() {
        let mut b = CopyButton::default();
        assert!(!b.update(true, false, 0.0));
        assert_eq!(b.tip, Some(COPY_TIP));
        b.update(false, false, 0.1);
        assert_eq!(b.tip, None);
    }

    #[test]
    fn copied_outlasts_leaving_then_follows_the_pointer() {
        let mut b = CopyButton::default();
        b.update(true, false, 0.0);
        assert!(b.update(true, true, 1.0));
        assert_eq!(b.tip, Some(COPIED_TIP));
        b.update(false, false, 1.2);
        assert_eq!(b.tip, Some(COPIED_TIP), "leaving does not hide 'Copied!'");
        b.update(false, false, 2.5);
        assert_eq!(b.tip, None, "gone once the time is up, pointer away");
        b.update(true, false, 3.0);
        b.update(true, true, 3.1);
        b.update(true, false, 4.7);
        assert_eq!(
            b.tip,
            Some(COPY_TIP),
            "back to the hover tip, pointer still over"
        );
    }

    #[test]
    fn a_second_click_restarts_the_timer() {
        let mut b = CopyButton::default();
        b.update(true, true, 0.0);
        b.update(true, true, 1.0);
        b.update(true, false, 2.0);
        assert_eq!(b.tip, Some(COPIED_TIP));
        b.update(true, false, 2.6);
        assert_eq!(b.tip, Some(COPY_TIP));
    }
}
