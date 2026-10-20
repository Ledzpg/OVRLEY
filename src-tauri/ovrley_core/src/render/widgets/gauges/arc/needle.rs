//! Dial needle for arc-shaped gauges.
//!
//! A short rounded bar with a translucent body and a solid border, centered on the track
//! (plus the configured offset) and rotated to the angle of the current value. It is
//! drawn every frame on top of the fill.

use super::geometry::ArcGaugeGeometry;
use crate::normalize::ValidatedGaugeNeedle;
use crate::render::text::parse_color;
use skia_safe::{paint::Style, Canvas, Paint, RRect, Rect};

/// Draws the needle at `angle` (screen degrees, clockwise) around the gauge center.
pub(super) fn draw_needle(
    canvas: &Canvas,
    needle: &ValidatedGaugeNeedle,
    geometry: ArcGaugeGeometry,
    angle: f32,
    opacity: f32,
) {
    let center_radius = geometry.radius + needle.offset;
    let body = Rect::from_xywh(
        center_radius - needle.length * 0.5,
        -needle.width * 0.5,
        needle.length,
        needle.width,
    );
    let corner = needle.corner_radius.min(needle.width * 0.5).min(needle.length * 0.5);
    let shape = RRect::new_rect_xy(body, corner, corner);

    canvas.save();
    canvas.translate((geometry.center_x, geometry.center_y));
    canvas.rotate(angle, None);
    if needle.fill_opacity > 0.0 {
        let mut fill = Paint::default();
        fill.set_anti_alias(true);
        fill.set_color(parse_color(&needle.color, needle.fill_opacity * opacity));
        canvas.draw_rrect(shape, &fill);
    }
    if needle.border_thickness > 0.0 {
        let mut border = Paint::default();
        border.set_anti_alias(true);
        border.set_style(Style::Stroke);
        border.set_stroke_width(needle.border_thickness);
        border.set_color(parse_color(&needle.color, opacity));
        canvas.draw_rrect(shape, &border);
    }
    canvas.restore();
}
