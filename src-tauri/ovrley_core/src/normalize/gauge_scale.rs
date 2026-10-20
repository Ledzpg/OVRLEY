//! Optional fixed scale, redline and fill color bands for gauge widgets.
//!
//! A gauge normally scales itself to the minimum and maximum values observed
//! in the activity. `scale_min`/`scale_max` replace that automatic range with a
//! fixed one (for example a tachometer drawn from 0 to 11 000 rpm), and
//! `redline_value` marks the point from which the track uses its own color.
//! `fill_zones` colors the fill along the scale with solid or gradient bands, and
//! `redline_fill_all` / `redline_blink_hz` make the gauge fill completely (and blink)
//! once the value reaches the redline. All values are expressed in the metric's
//! canonical unit (rpm, °C).
//!
//! Absence of every field is the documented default: the gauge keeps its
//! automatic range and has no redline. Anything present is validated once here.

use super::helpers::{require_finite_f64, require_hex_color};
use super::raw::ValueConfig;
use crate::error::{CoreError, CoreResult};

/// Most bands a fill may have.
pub const MAX_FILL_ZONES: usize = 12;
/// Fastest allowed redline blink, in flashes per second.
pub const MAX_REDLINE_BLINK_HZ: f64 = 20.0;

/// Validated redline marker.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedRedline {
    /// Canonical-unit value at which the redline zone starts.
    pub value: f64,
    /// Hex color used for the redline zone.
    pub color: String,
    /// Fill the whole track in the redline color while the value is at or past the redline.
    pub fill_all: bool,
    /// Blink rate of that full fill in flashes per second; `None` keeps it steady.
    pub blink_hz: Option<f64>,
}

/// Validated color band of the fill.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedFillZone {
    /// Canonical-unit value where the band starts.
    pub start: f64,
    pub color: String,
    /// Gradient target reached at the start of the next band (or the scale end).
    pub end_color: Option<String>,
}

/// Validated scale override, redline and fill bands of a gauge.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidatedGaugeScale {
    /// Fixed `(min, max)` range, or `None` to follow the observed range.
    pub range: Option<(f64, f64)>,
    /// Optional redline zone.
    pub redline: Option<ValidatedRedline>,
    /// Fill color bands in ascending order; empty means the single configured fill color.
    pub fill_zones: Vec<ValidatedFillZone>,
}

/// Validates the optional scale, redline and fill-band fields of a gauge value config.
pub(crate) fn validate_gauge_scale(
    value: &ValueConfig,
    path: &dyn Fn(&str) -> String,
) -> CoreResult<ValidatedGaugeScale> {
    let range = match (value.scale_min, value.scale_max) {
        (None, None) => None,
        (Some(min), Some(max)) => {
            let min = require_finite_f64(min, &path("scale_min"))?;
            let max = require_finite_f64(max, &path("scale_max"))?;
            if max <= min {
                return Err(CoreError::Config(format!(
                    "{}: must be greater than {}",
                    path("scale_max"),
                    path("scale_min")
                )));
            }
            Some((min, max))
        }
        (Some(_), None) => {
            return Err(CoreError::Config(format!(
                "{}: required when scale_min is set",
                path("scale_max")
            )))
        }
        (None, Some(_)) => {
            return Err(CoreError::Config(format!(
                "{}: required when scale_max is set",
                path("scale_min")
            )))
        }
    };

    let redline = match value.redline_value {
        None => {
            if value.redline_fill_all == Some(true) || value.redline_blink_hz.is_some() {
                return Err(CoreError::Config(format!(
                    "{}: requires redline_value",
                    path("redline_fill_all")
                )));
            }
            None
        }
        Some(redline_value) => {
            let redline_value = require_finite_f64(redline_value, &path("redline_value"))?;
            if let Some((min, max)) = range {
                if redline_value <= min || redline_value >= max {
                    return Err(CoreError::Config(format!(
                        "{}: must be between scale_min and scale_max",
                        path("redline_value")
                    )));
                }
            }
            let color = require_hex_color(value.redline_color.as_deref(), &path("redline_color"))?;
            let blink_hz = match value.redline_blink_hz {
                None => None,
                Some(hz) => {
                    let hz = require_finite_f64(hz, &path("redline_blink_hz"))?;
                    if hz <= 0.0 || hz > MAX_REDLINE_BLINK_HZ {
                        return Err(CoreError::Config(format!(
                            "{}: must be greater than 0 and at most {MAX_REDLINE_BLINK_HZ}",
                            path("redline_blink_hz")
                        )));
                    }
                    Some(hz)
                }
            };
            Some(ValidatedRedline {
                value: redline_value,
                color,
                fill_all: value.redline_fill_all.unwrap_or(false),
                blink_hz,
            })
        }
    };

    let fill_zones = validate_fill_zones(value, path, range)?;
    Ok(ValidatedGaugeScale {
        range,
        redline,
        fill_zones,
    })
}

fn validate_fill_zones(
    value: &ValueConfig,
    path: &dyn Fn(&str) -> String,
    range: Option<(f64, f64)>,
) -> CoreResult<Vec<ValidatedFillZone>> {
    let Some(zones) = value.fill_zones.as_ref() else {
        return Ok(Vec::new());
    };
    let (min, max) = range.ok_or_else(|| {
        CoreError::Config(format!(
            "{}: fill zones require a fixed scale (scale_min and scale_max)",
            path("fill_zones")
        ))
    })?;
    if zones.is_empty() || zones.len() > MAX_FILL_ZONES {
        return Err(CoreError::Config(format!(
            "{}: expected 1 to {MAX_FILL_ZONES} zones, got {}",
            path("fill_zones"),
            zones.len()
        )));
    }
    let mut validated = Vec::with_capacity(zones.len());
    let mut previous: Option<f64> = None;
    for (index, zone) in zones.iter().enumerate() {
        let zone_path = |field: &str| path(&format!("fill_zones[{index}].{field}"));
        let start = require_finite_f64(zone.start, &zone_path("start"))?;
        match previous {
            None if start != min => {
                return Err(CoreError::Config(format!(
                    "{}: the first zone must start at scale_min ({min})",
                    zone_path("start")
                )))
            }
            Some(before) if start <= before => {
                return Err(CoreError::Config(format!(
                    "{}: must be greater than the previous zone start ({before})",
                    zone_path("start")
                )))
            }
            _ => {}
        }
        if start >= max {
            return Err(CoreError::Config(format!(
                "{}: must be below scale_max ({max})",
                zone_path("start")
            )));
        }
        previous = Some(start);
        validated.push(ValidatedFillZone {
            start,
            color: require_hex_color(Some(zone.color.as_str()), &zone_path("color"))?,
            end_color: zone
                .end_color
                .as_deref()
                .map(|color| require_hex_color(Some(color), &zone_path("end_color")))
                .transpose()?,
        });
    }
    Ok(validated)
}
