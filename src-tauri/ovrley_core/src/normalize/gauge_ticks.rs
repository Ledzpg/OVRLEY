//! Optional dial ticks and numerals for arc-shaped gauges.
//!
//! A tachometer-style dial marks its fixed scale with major ticks (optionally
//! numbered), minor ticks between them, and recolors everything past the
//! redline. Ticks only make sense on a known scale, so they require a fixed
//! `scale_min`/`scale_max`. Setting `tick_major_interval` turns ticks on; once on,
//! every other tick field is required (the frontend materializes the defaults).
//! Values are in the metric's canonical unit (rpm, °C).
//!
//! Numerals use the gauge's min/max label font (`min_max_label_font`).

use super::gauge_scale::ValidatedGaugeScale;
use super::helpers::{
    require_bool, require_f32, require_finite_f64, require_hex_color, require_non_negative_f32,
    require_positive_f32, require_str,
};
use super::raw::ValueConfig;
use crate::error::{CoreError, CoreResult};

/// Largest number of ticks (major plus minor) a dial may draw.
pub const MAX_GAUGE_TICKS: usize = 400;
/// Largest number of minor ticks between two major ticks.
pub const MAX_MINOR_TICKS_PER_MAJOR: u32 = 9;

/// Side of the track the ticks are drawn on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TickPosition {
    /// Toward the dial center.
    Inside,
    /// Away from the dial center.
    Outside,
}

/// Validated dial tick configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedGaugeTicks {
    /// Distance between major ticks, in the metric's canonical unit.
    pub major_interval: f64,
    /// Minor ticks drawn between two major ticks.
    pub minor_per_major: u32,
    pub position: TickPosition,
    pub color: String,
    pub major_length: f32,
    pub minor_length: f32,
    pub thickness: f32,
    /// Gap between the track edge and the start of the ticks.
    pub offset: f32,
    pub show_labels: bool,
    /// Numerals show `value / label_divisor` (for example rpm / 1000).
    pub label_divisor: f64,
    pub label_decimals: u32,
    pub label_font_size: f32,
    pub label_color: String,
    /// Gap between the end of a tick and its numeral.
    pub label_offset: f32,
}

/// One tick position on the scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickMark {
    pub value: f64,
    pub major: bool,
}

impl ValidatedGaugeTicks {
    /// Lists every tick between `min` and `max`, majors anchored at multiples of the interval.
    pub fn marks(&self, min: f64, max: f64) -> Vec<TickMark> {
        let step = self.major_interval / f64::from(self.minor_per_major + 1);
        let epsilon = step * 1e-6;
        let first = (min / step - 1e-6).ceil() as i64;
        let last = (max / step + 1e-6).floor() as i64;
        let subdivisions = i64::from(self.minor_per_major + 1);
        (first..=last)
            .map(|index| TickMark {
                value: index as f64 * step,
                major: index.rem_euclid(subdivisions) == 0,
            })
            .filter(|mark| mark.value >= min - epsilon && mark.value <= max + epsilon)
            .collect()
    }
}

fn parse_position(value: Option<&str>, path: &str) -> CoreResult<TickPosition> {
    match require_str(value, path)? {
        "inside" => Ok(TickPosition::Inside),
        "outside" => Ok(TickPosition::Outside),
        other => Err(CoreError::Config(format!(
            "{path}: expected 'inside' or 'outside', got '{other}'"
        ))),
    }
}

/// Validates the optional tick fields of an arc-shaped gauge.
///
/// Returns `None` when `tick_major_interval` is absent. Ticks require a fixed scale.
pub(crate) fn validate_gauge_ticks(
    value: &ValueConfig,
    path: &dyn Fn(&str) -> String,
    scale: &ValidatedGaugeScale,
) -> CoreResult<Option<ValidatedGaugeTicks>> {
    let Some(interval) = value.tick_major_interval else {
        return Ok(None);
    };
    let (min, max) = scale.range.ok_or_else(|| {
        CoreError::Config(format!(
            "{}: ticks require a fixed scale (scale_min and scale_max)",
            path("tick_major_interval")
        ))
    })?;
    let major_interval = require_finite_f64(interval, &path("tick_major_interval"))?;
    if major_interval <= 0.0 {
        return Err(CoreError::Config(format!(
            "{}: must be greater than 0",
            path("tick_major_interval")
        )));
    }

    let minor_per_major = value
        .tick_minor_count
        .ok_or_else(|| CoreError::Config(format!("{}: required", path("tick_minor_count"))))?;
    if minor_per_major > MAX_MINOR_TICKS_PER_MAJOR {
        return Err(CoreError::Config(format!(
            "{}: must be at most {MAX_MINOR_TICKS_PER_MAJOR}",
            path("tick_minor_count")
        )));
    }

    let label_divisor = match value.tick_label_divisor {
        Some(divisor) => require_finite_f64(divisor, &path("tick_label_divisor"))?,
        None => {
            return Err(CoreError::Config(format!(
                "{}: required",
                path("tick_label_divisor")
            )))
        }
    };
    if label_divisor <= 0.0 {
        return Err(CoreError::Config(format!(
            "{}: must be greater than 0",
            path("tick_label_divisor")
        )));
    }

    let ticks = ValidatedGaugeTicks {
        major_interval,
        minor_per_major,
        position: parse_position(value.tick_position.as_deref(), &path("tick_position"))?,
        color: require_hex_color(value.tick_color.as_deref(), &path("tick_color"))?,
        major_length: require_non_negative_f32(
            require_f32(value.tick_major_length, &path("tick_major_length"))?,
            &path("tick_major_length"),
        )?,
        minor_length: require_non_negative_f32(
            require_f32(value.tick_minor_length, &path("tick_minor_length"))?,
            &path("tick_minor_length"),
        )?,
        thickness: require_positive_f32(value.tick_thickness, &path("tick_thickness"))?,
        offset: require_non_negative_f32(
            require_f32(value.tick_offset, &path("tick_offset"))?,
            &path("tick_offset"),
        )?,
        show_labels: require_bool(value.tick_show_labels, &path("tick_show_labels"))?,
        label_divisor,
        label_decimals: value
            .tick_label_decimals
            .ok_or_else(|| CoreError::Config(format!("{}: required", path("tick_label_decimals"))))?,
        label_font_size: require_positive_f32(
            value.tick_label_font_size,
            &path("tick_label_font_size"),
        )?,
        label_color: require_hex_color(value.tick_label_color.as_deref(), &path("tick_label_color"))?,
        label_offset: require_non_negative_f32(
            require_f32(value.tick_label_offset, &path("tick_label_offset"))?,
            &path("tick_label_offset"),
        )?,
    };

    let count = ticks.marks(min, max).len();
    if count > MAX_GAUGE_TICKS {
        return Err(CoreError::Config(format!(
            "{}: {count} ticks exceed the maximum of {MAX_GAUGE_TICKS}",
            path("tick_major_interval")
        )));
    }
    Ok(Some(ticks))
}
