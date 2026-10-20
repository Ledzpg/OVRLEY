//! Dial ticks and numerals for arc-shaped gauges.
//!
//! Ticks and numerals are frame-invariant, so they are drawn once into the cached
//! static layer. Their angles use the same cap-aware mapping as the continuous
//! fill (or the plain sweep for segmented bars), so a tick marks exactly the
//! point where a fill of that value ends. Marks at or past the redline use the
//! redline color.

use super::geometry::{arc_point, ArcGaugeGeometry};
use super::inner_widget::LINE_HEIGHT;
use super::path::ArcTrackSpec;
use crate::error::CoreResult;
use crate::normalize::{TickPosition, ValidatedArcGaugeWidget, ValidatedGaugeTicks};
use crate::render::text::{
    draw_text, origin_x_for_centered_text, parse_color, resolve_font, ResolvedTextStyle,
};
use crate::render::widgets::gauges::metric::fill_percentage;
use crate::render::widgets::types::ResolvedRedline;
use crate::types::TrackFillStyle;
use skia_safe::{paint::Style, Canvas, Paint, PaintCap};
use std::path::PathBuf;

/// Extra room the static layer needs when ticks and numerals sit outside the track.
pub(super) fn ticks_padding(gauge: &ValidatedArcGaugeWidget, scale: f32) -> u32 {
    let Some(ticks) = gauge.ticks.as_ref().filter(|t| t.position == TickPosition::Outside) else {
        return 0;
    };
    let labels = if ticks.show_labels {
        ticks.label_offset + ticks.label_font_size * 2.0
    } else {
        0.0
    };
    ((ticks.offset + ticks.major_length.max(ticks.minor_length) + labels) * scale).ceil() as u32
}

/// Draws every tick and numeral of the dial into the static layer.
pub(super) fn draw_ticks(
    canvas: &Canvas,
    gauge: &ValidatedArcGaugeWidget,
    geometry: ArcGaugeGeometry,
    spec: ArcTrackSpec,
    scale: f32,
    font_dirs: &[PathBuf],
    range: (f64, f64),
    redline: Option<&ResolvedRedline>,
    text_style: &ResolvedTextStyle,
) -> CoreResult<()> {
    let Some(ticks) = gauge.ticks.as_ref() else {
        return Ok(());
    };
    let (min, max) = range;
    let thickness = (gauge.track_thickness + gauge.track_border_thickness * 2.0) * scale;
    let half = thickness * 0.5;
    let (edge, direction) = match ticks.position {
        TickPosition::Outside => (geometry.radius + half + ticks.offset * scale, 1.0),
        TickPosition::Inside => (geometry.radius - half - ticks.offset * scale, -1.0),
    };
    let font = if ticks.show_labels {
        Some(resolve_font(
            font_dirs,
            Some(&gauge.min_max_label_font),
            ticks.label_font_size * scale,
        )?)
    } else {
        None
    };

    for mark in ticks.marks(min, max) {
        let fraction = fill_percentage(mark.value, min, max);
        let angle = match gauge.track_fill_style {
            TrackFillStyle::Fill => spec.angle_at(fraction),
            TrackFillStyle::Bars => geometry.start_angle + geometry.sweep_angle * fraction,
        };
        let in_redline = redline
            .map(|redline| fraction >= redline.fraction)
            .unwrap_or(false);
        let color = match redline {
            Some(redline) if in_redline => redline.color.as_str(),
            _ => ticks.color.as_str(),
        };
        let length = if mark.major {
            ticks.major_length
        } else {
            ticks.minor_length
        } * scale;
        let from = arc_point(geometry.center_x, geometry.center_y, edge, angle);
        let to = arc_point(
            geometry.center_x,
            geometry.center_y,
            edge + direction * length,
            angle,
        );
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(Style::Stroke);
        paint.set_stroke_width((ticks.thickness * scale).max(0.5));
        paint.set_stroke_cap(PaintCap::Butt);
        paint.set_color(parse_color(color, text_style.opacity));
        canvas.draw_line(from, to, &paint);

        if let (true, Some(font)) = (mark.major && ticks.show_labels, font.as_ref()) {
            let label_color = match redline {
                Some(redline) if in_redline => redline.color.as_str(),
                _ => ticks.label_color.as_str(),
            };
            draw_numeral(
                canvas,
                ticks,
                geometry,
                edge + direction * (length + ticks.label_offset * scale),
                angle,
                mark.value,
                label_color,
                font,
                scale,
                font_dirs,
                gauge,
                text_style,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_numeral(
    canvas: &Canvas,
    ticks: &ValidatedGaugeTicks,
    geometry: ArcGaugeGeometry,
    radius: f32,
    angle: f32,
    value: f64,
    color: &str,
    font: &skia_safe::Font,
    scale: f32,
    font_dirs: &[PathBuf],
    gauge: &ValidatedArcGaugeWidget,
    text_style: &ResolvedTextStyle,
) -> CoreResult<()> {
    let text = format_numeral(value / ticks.label_divisor, ticks.label_decimals);
    let size = ticks.label_font_size * scale;
    // Place the numeral so its near edge, not its center, clears the tick: push the anchor out by
    // half the text extent along the radial direction.
    let (width, _) = font.measure_str(&text, None);
    let radial = (angle.to_radians().cos(), angle.to_radians().sin());
    let half_extent = (width * 0.5 * radial.0.abs()).max(size * 0.35 * radial.1.abs());
    let signed = match ticks.position {
        TickPosition::Outside => half_extent,
        TickPosition::Inside => -half_extent,
    };
    let anchor = arc_point(geometry.center_x, geometry.center_y, radius + signed, angle);
    let mut style = text_style.clone();
    style.font_name = Some(gauge.min_max_label_font.clone());
    style.font_size = size;
    style.line_height = size * LINE_HEIGHT;
    style.color = parse_color(color, text_style.opacity);
    style.x = origin_x_for_centered_text(&text, anchor.x, font);
    style.y = anchor.y - style.line_height * 0.5;
    draw_text(canvas, &text, &style, font_dirs)
}

/// Formats a scaled tick value with a fixed number of decimals (no trailing "-0").
pub(super) fn format_numeral(value: f64, decimals: u32) -> String {
    let text = format!("{value:.*}", decimals as usize);
    if text.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') {
        text.trim_start_matches('-').to_string()
    } else {
        text
    }
}
