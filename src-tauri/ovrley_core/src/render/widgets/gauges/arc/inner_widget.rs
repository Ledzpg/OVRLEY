//! Value/unit layout shared by arc-shaped gauges.
//!
//! A unit above or below the value is measured with it as one centered vertical stack. Such a unit
//! is frame-invariant and belongs to the cached static layer; only the formatted metric value is
//! painted during per-frame composition. A unit beside the value follows the width of the current
//! value, so it is painted per frame, sharing the value's baseline.

use super::geometry::ArcGaugeGeometry;
use crate::error::CoreResult;
use crate::normalize::{ValidatedArcGaugeWidget, ValidatedArcUnitPosition};
use crate::render::text::{
    draw_text, measure_text_with_font, origin_x_for_centered_text, resolve_font, ResolvedTextStyle,
};
use crate::render::widgets::value::NUMERIC_VERTICAL_METRICS_TEXT;
use skia_safe::Canvas;
use std::path::PathBuf;

pub(crate) const LINE_HEIGHT: f32 = 0.92;
const UNIT_RATIO: f32 = 0.28;
const MIN_UNIT_FONT_SIZE: f32 = 12.0;
pub(crate) const DEFAULT_GAP_PX: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
/// Baselines and total bounds for the centered value/unit stack.
pub(crate) struct ArcInnerWidgetLayout {
    pub(crate) center_x: f32,
    pub(crate) value_top: f32,
    pub(crate) unit_top: Option<f32>,
}

/// Resolves the unit font size relative to the validated value style.
pub(crate) fn unit_font_size(text_style: &ResolvedTextStyle, scale: f32) -> f32 {
    (text_style.font_size * UNIT_RATIO).max(MIN_UNIT_FONT_SIZE * scale)
}

/// Measures and centers the value and an optional stacked unit around the inner anchor.
/// `unit_above` puts the stacked unit over the value instead of under it; a unit beside the value
/// is not part of the stack (pass no `unit_font_size`).
pub(crate) fn inner_widget_layout(
    geometry: ArcGaugeGeometry,
    offset_x: f32,
    offset_y: f32,
    text_style: &ResolvedTextStyle,
    unit_font_size: Option<f32>,
    minimum_gap: f32,
    unit_above: bool,
) -> ArcInnerWidgetLayout {
    let value_line_height = text_style.font_size * LINE_HEIGHT;
    let unit_line_height = unit_font_size.unwrap_or(0.0) * LINE_HEIGHT;
    let gap = (text_style.font_size * 0.08).max(minimum_gap);
    let group_height = if unit_font_size.is_some() {
        value_line_height + gap + unit_line_height
    } else {
        value_line_height
    };
    let top = geometry.inner_widget_center_y + offset_y - group_height * 0.5;

    let (value_top, unit_top) = if unit_above {
        (top + unit_line_height + gap, top)
    } else {
        (top, top + value_line_height + gap)
    };
    ArcInnerWidgetLayout {
        center_x: geometry.inner_widget_center_x + offset_x,
        value_top,
        unit_top: unit_font_size.map(|_| unit_top),
    }
}

/// Draws a unit beside the value, on the value's baseline.
///
/// The value stays centered on the anchor; the unit sits against its left or right edge.
pub(crate) fn draw_beside_unit(
    canvas: &Canvas,
    unit_style: &ResolvedTextStyle,
    unit_text: &str,
    position: ValidatedArcUnitPosition,
    value_style: &ResolvedTextStyle,
    center_x: f32,
    value_top: f32,
    value_advance: f32,
    gap: f32,
    font_dirs: &[PathBuf],
) -> CoreResult<()> {
    let value_font = resolve_font(font_dirs, value_style.font_name.as_deref(), value_style.font_size)?;
    let unit_font = resolve_font(font_dirs, unit_style.font_name.as_deref(), unit_style.font_size)?;
    let baseline_offset = |font: &skia_safe::Font, text: &str, line_height: f32| {
        let bounds = measure_text_with_font(text, font);
        let glyph_height = bounds.bounds_bottom - bounds.bounds_top;
        (line_height - glyph_height) * 0.5 - bounds.bounds_top
    };
    let value_baseline = value_top
        + baseline_offset(
            &value_font,
            NUMERIC_VERTICAL_METRICS_TEXT,
            value_style.font_size * LINE_HEIGHT,
        );
    let unit_line_height = unit_style.font_size * LINE_HEIGHT;
    let (unit_advance, _) = unit_font.measure_str(unit_text, None);
    let mut style = unit_style.clone();
    style.line_height = unit_line_height;
    style.y = value_baseline - baseline_offset(&unit_font, unit_text, unit_line_height);
    style.x = match position {
        ValidatedArcUnitPosition::Left => center_x - value_advance * 0.5 - gap - unit_advance,
        _ => center_x + value_advance * 0.5 + gap,
    };
    draw_text(canvas, unit_text, &style, font_dirs)
}

/// Draws the optional unit into the frame-invariant static layer.
pub(crate) fn draw_static_unit(
    canvas: &Canvas,
    gauge: &ValidatedArcGaugeWidget,
    geometry: ArcGaugeGeometry,
    scale: f32,
    font_dirs: &[PathBuf],
    unit_text: &str,
    text_style: &ResolvedTextStyle,
) -> CoreResult<()> {
    let mut unit_style = text_style.clone();
    unit_style.font_size = unit_font_size(text_style, scale);
    unit_style.line_height = unit_style.font_size * LINE_HEIGHT;
    unit_style.color = skia_safe::Color::from_argb(
        gauge.inner_value.unit_color[3],
        gauge.inner_value.unit_color[0],
        gauge.inner_value.unit_color[1],
        gauge.inner_value.unit_color[2],
    );
    let layout = inner_widget_layout(
        geometry,
        gauge.inner_widget_offset_x * scale,
        gauge.inner_widget_offset_y * scale,
        text_style,
        Some(unit_style.font_size),
        DEFAULT_GAP_PX * scale,
        gauge.unit_position == ValidatedArcUnitPosition::Above,
    );
    let font = resolve_font(
        font_dirs,
        unit_style.font_name.as_deref(),
        unit_style.font_size,
    )?;
    unit_style.x = origin_x_for_centered_text(unit_text, layout.center_x, &font);
    unit_style.y = layout
        .unit_top
        .expect("unit layout must include a top when a unit exists");
    draw_text(canvas, unit_text, &unit_style, font_dirs)
}
