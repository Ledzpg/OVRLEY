//! Optional fixed scale and redline for gauge widgets.
//!
//! A gauge normally scales itself to the minimum and maximum values observed
//! in the activity. `scale_min`/`scale_max` replace that automatic range with a
//! fixed one (for example a tachometer drawn from 0 to 11 000 rpm), and
//! `redline_value` marks the point from which the track uses its own color.
//! All values are expressed in the metric's canonical unit (rpm, °C).
//!
//! Absence of every field is the documented default: the gauge keeps its
//! automatic range and has no redline. Anything present is validated once here.

use super::helpers::{require_finite_f64, require_hex_color};
use super::raw::ValueConfig;
use crate::error::{CoreError, CoreResult};

/// Validated redline marker.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedRedline {
    /// Canonical-unit value at which the redline zone starts.
    pub value: f64,
    /// Hex color used for the redline zone.
    pub color: String,
}

/// Validated scale override and redline of a gauge.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidatedGaugeScale {
    /// Fixed `(min, max)` range, or `None` to follow the observed range.
    pub range: Option<(f64, f64)>,
    /// Optional redline zone.
    pub redline: Option<ValidatedRedline>,
}

/// Validates the optional scale and redline fields of a gauge value config.
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
        None => None,
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
            Some(ValidatedRedline {
                value: redline_value,
                color,
            })
        }
    };

    Ok(ValidatedGaugeScale { range, redline })
}
