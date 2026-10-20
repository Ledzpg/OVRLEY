//! Shared telemetry, fill, and boundary-label helpers for gauges.
//!
//! This module owns the common mapping from a validated metric to its dense
//! series, reads full-activity bounds during cache preparation, and keeps
//! fill quantization and boundary-label formatting identical across arc and
//! linear renderers.

use crate::activity::schema::{DenseActivityReport, DenseSeriesReport};
use crate::normalize::ValidatedGaugeScale;
use crate::render::text::parse_color;
use crate::render::widgets::types::{FillStop, ResolvedRedline};
use crate::render::format::{convert_fuel_level, convert_standard_metric_value};
use crate::types::MetricKind;

/// Maps a metric value into the inclusive normalized gauge range.
///
/// Values outside the range are clamped. A degenerate range has no measurable
/// progress and therefore resolves to zero fill.
pub(crate) fn fill_percentage(value: f64, min: f64, max: f64) -> f32 {
    if max <= min {
        return 0.0;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0) as f32
}

/// Returns the number of completely filled bars at the supplied progress.
pub(crate) fn bar_fill_count(fill01: f32, count: u32) -> usize {
    (fill01.clamp(0.0, 1.0) * count as f32).floor() as usize
}

/// Reads the metric's source-activity bounds independently of export coverage.
///
/// An absent or constant series uses the documented neutral gauge range so
/// cache preparation still has a usable scale.
pub(crate) fn metric_range(activity: &DenseActivityReport, metric: MetricKind) -> (f64, f64) {
    activity
        .full_activity_metric_ranges
        .get(&metric)
        .copied()
        .filter(|(min, max)| max > min)
        .unwrap_or((0.0, 100.0))
}

/// Resolves the gauge range: the configured fixed scale, else the observed range.
pub(crate) fn gauge_range(
    scale: &ValidatedGaugeScale,
    activity: &DenseActivityReport,
    metric: MetricKind,
) -> (f64, f64) {
    scale
        .range
        .unwrap_or_else(|| metric_range(activity, metric))
}

/// Places the configured redline on the resolved range.
pub(crate) fn resolve_redline(
    scale: &ValidatedGaugeScale,
    min: f64,
    max: f64,
) -> Option<ResolvedRedline> {
    scale.redline.as_ref().map(|redline| ResolvedRedline {
        fraction: fill_percentage(redline.value, min, max),
        color: redline.color.clone(),
        value: redline.value,
        fill_all: redline.fill_all,
        blink_hz: redline.blink_hz,
    })
}

/// Resolves the fill color bands into gradient stops along the track.
///
/// A solid band contributes its color at both ends; a gradient band blends to its
/// `end_color` at the start of the next band. The last band ends at the redline when one
/// lies inside it (the fill past the redline is the redline color anyway), so a gradient in
/// the last band reaches its end color exactly at the redline; without a redline it runs to
/// the scale end. Hard boundaries between bands are separated by a hair so positions stay
/// strictly increasing.
pub(crate) fn fill_stops(
    scale: &ValidatedGaugeScale,
    min: f64,
    max: f64,
    opacity: f32,
) -> Option<Vec<FillStop>> {
    if scale.fill_zones.is_empty() {
        return None;
    }
    let mut stops: Vec<FillStop> = Vec::with_capacity(scale.fill_zones.len() * 2);
    for (index, zone) in scale.fill_zones.iter().enumerate() {
        let start = fill_percentage(zone.start, min, max);
        let last_band_end = scale
            .redline
            .as_ref()
            .filter(|redline| redline.value > zone.start && redline.value < max)
            .map_or(1.0, |redline| fill_percentage(redline.value, min, max));
        let end = scale
            .fill_zones
            .get(index + 1)
            .map_or(last_band_end, |next| fill_percentage(next.start, min, max));
        let start_color = parse_color(&zone.color, opacity);
        let end_color = zone
            .end_color
            .as_deref()
            .map_or(start_color, |color| parse_color(color, opacity));
        stops.push(FillStop {
            fraction: start,
            color: start_color,
        });
        stops.push(FillStop {
            fraction: end,
            color: end_color,
        });
    }
    for index in 1..stops.len() {
        if stops[index].fraction <= stops[index - 1].fraction {
            stops[index].fraction = (stops[index - 1].fraction + 1e-5).min(1.0);
        }
    }
    Some(stops)
}

/// Interpolated fill color at `fraction` along the track.
pub(crate) fn stop_color_at(stops: &[FillStop], fraction: f32) -> skia_safe::Color {
    let first = stops[0];
    if fraction <= first.fraction {
        return first.color;
    }
    for pair in stops.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if fraction <= to.fraction {
            let span = (to.fraction - from.fraction).max(f32::EPSILON);
            let t = ((fraction - from.fraction) / span).clamp(0.0, 1.0);
            let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
            return skia_safe::Color::from_argb(
                mix(from.color.a(), to.color.a()),
                mix(from.color.r(), to.color.r()),
                mix(from.color.g(), to.color.g()),
                mix(from.color.b(), to.color.b()),
            );
        }
    }
    stops[stops.len() - 1].color
}

/// Evaluates the redline overrun state of one frame: `(overrun, blink_on)`.
///
/// `overrun` is true when a gauge that fills completely at the redline reaches it; `blink_on`
/// is the visible phase of the optional blinking, derived from the frame time so the exported
/// video blinks at the configured rate.
pub(crate) fn redline_overrun(
    redline: Option<&ResolvedRedline>,
    value: f64,
    frame_seconds: f64,
) -> (bool, bool) {
    let Some(redline) = redline.filter(|redline| redline.fill_all && value >= redline.value) else {
        return (false, true);
    };
    let blink_on = redline
        .blink_hz
        .map_or(true, |hz| ((frame_seconds * hz * 2.0).floor() as i64).rem_euclid(2) == 0);
    (true, blink_on)
}

/// Reports whether a discrete bar starts inside the redline zone.
pub(crate) fn bar_is_redline(index: u32, count: u32, fraction: f32) -> bool {
    count > 0 && index as f32 / count as f32 >= fraction
}

/// Selects the canonical dense telemetry series for a metric.
///
/// Derived metrics without their own dense series return an empty slice; their
/// presentation range consequently follows [`metric_range`]'s neutral range.
pub(crate) fn metric_values(series: &DenseSeriesReport, metric: MetricKind) -> &[Option<f64>] {
    match metric {
        MetricKind::GearPosition
        | MetricKind::LeftRightBalance
        | MetricKind::GpsCoordinates
        | MetricKind::Gradient
        | MetricKind::TotalAscent
        | MetricKind::Time
        | MetricKind::LapTimer => &[],
        metric => series.numeric_series_for(metric).unwrap_or(&[]),
    }
}

/// Converts a raw telemetry min/max value through the selected display unit
/// and rounds it to the nearest integer label.
pub(crate) fn format_gauge_label(
    kind: MetricKind,
    display_unit: Option<&str>,
    fuel_tank_capacity: Option<f64>,
    value: f64,
) -> String {
    let converted = match kind {
        MetricKind::FuelLevel => convert_fuel_level(display_unit, value, fuel_tank_capacity),
        _ => convert_standard_metric_value(kind, display_unit, value),
    };
    (converted.round() as i64).to_string()
}
