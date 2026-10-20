//! Dial needle for arc-shaped gauges.
//!
//! A short rounded bar with a translucent body and a solid border, centered on the track
//! (plus the configured offset) and rotated to the angle of the current value. The body is a
//! single color or a gradient from the end toward the center to the outer end, each end with its
//! own color and opacity. It is drawn every frame on top of the fill.

use super::geometry::ArcGaugeGeometry;
use crate::normalize::ValidatedGaugeNeedle;
use crate::render::text::parse_color;
use skia_safe::{paint::Style, Canvas, Color4f, Paint, RRect, Rect};

/// Draws the needle at `angle` (screen degrees, clockwise) around the gauge center.
pub(super) fn draw_needle(
    canvas: &Canvas,
    needle: &ValidatedGaugeNeedle,
    geometry: ArcGaugeGeometry,
    angle: f32,
    opacity: f32,
) {
    let center_radius = geometry.radius + needle.offset;
    let (inner, outer) = (
        center_radius - needle.length * 0.5,
        center_radius + needle.length * 0.5,
    );
    let body = Rect::from_xywh(inner, -needle.width * 0.5, needle.length, needle.width);
    let corner = needle.corner_radius.min(needle.width * 0.5).min(needle.length * 0.5);
    let shape = RRect::new_rect_xy(body, corner, corner);

    canvas.save();
    canvas.translate((geometry.center_x, geometry.center_y));
    canvas.rotate(angle, None);
    if let Some(fill) = fill_paint(needle, inner, outer, opacity) {
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

/// Paint of the body, or `None` when every end is fully transparent.
fn fill_paint(needle: &ValidatedGaugeNeedle, inner: f32, outer: f32, opacity: f32) -> Option<Paint> {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let start = parse_color(&needle.fill_color, needle.fill_opacity * opacity);
    let Some(end) = needle.fill_end.as_ref() else {
        if needle.fill_opacity <= 0.0 {
            return None;
        }
        paint.set_color(start);
        return Some(paint);
    };
    if needle.fill_opacity <= 0.0 && end.opacity <= 0.0 {
        return None;
    }
    let end_color = parse_color(&end.color, end.opacity * opacity);
    let colors = [Color4f::from(start), Color4f::from(end_color)];
    let gradient = skia_safe::gradient::Gradient::new(
        skia_safe::gradient::Colors::new(&colors, None, skia_safe::TileMode::Clamp, None),
        skia_safe::gradient::Interpolation::default(),
    );
    let shader = skia_safe::gradient::shaders::linear_gradient(((inner, 0.0), (outer, 0.0)), &gradient, None)?;
    paint.set_shader(shader);
    Some(paint)
}
