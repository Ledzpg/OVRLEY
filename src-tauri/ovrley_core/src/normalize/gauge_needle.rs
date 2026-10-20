//! Optional needle for arc-shaped gauges.
//!
//! The needle is a short rounded bar with a translucent body that sits across the track
//! at the angle of the current value, like the pointer of an analog dial. Setting
//! `needle_length` turns it on; once on, every other needle field is required (the
//! frontend materializes the defaults). It works on automatic and fixed scales alike.
//! Lengths are in unscaled widget pixels.
//!
//! The body is filled with `needle_fill_color` (documented optional: absent means the border color
//! `needle_color`) at `needle_fill_opacity`. Setting `needle_fill_end_color` turns the fill into a
//! gradient along the needle, from the end toward the center to the outer end, and then
//! `needle_fill_end_opacity` is required (and rejected without an end color).

use super::helpers::{
    require_f32, require_hex_color, require_non_negative_f32, require_opacity, require_positive_f32,
};
use super::raw::ValueConfig;
use crate::error::{CoreError, CoreResult};

/// Validated needle configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedGaugeNeedle {
    /// Size of the needle along the radius.
    pub length: f32,
    /// Size of the needle across the radius.
    pub width: f32,
    /// Shift of the needle center from the middle of the track; positive moves outward.
    pub offset: f32,
    /// Color of the border.
    pub color: String,
    pub border_thickness: f32,
    /// Color of the body, or of its inner end when there is a gradient.
    pub fill_color: String,
    /// Opacity of the body, or of its inner end when there is a gradient (0 leaves only the border).
    pub fill_opacity: f32,
    /// Outer end of a gradient body; `None` keeps the body a single color.
    pub fill_end: Option<NeedleFillEnd>,
    pub corner_radius: f32,
}

/// Outer end of a gradient needle body.
#[derive(Clone, Debug, PartialEq)]
pub struct NeedleFillEnd {
    pub color: String,
    pub opacity: f32,
}

/// Validates the optional needle fields of an arc-shaped gauge.
///
/// Returns `None` when `needle_length` is absent.
pub(crate) fn validate_gauge_needle(
    value: &ValueConfig,
    path: &dyn Fn(&str) -> String,
) -> CoreResult<Option<ValidatedGaugeNeedle>> {
    let Some(length) = value.needle_length else {
        return Ok(None);
    };
    let length = require_positive_f32(Some(length), &path("needle_length"))?;
    let color = require_hex_color(value.needle_color.as_deref(), &path("needle_color"))?;
    let fill_color = match value.needle_fill_color.as_deref() {
        Some(fill) => require_hex_color(Some(fill), &path("needle_fill_color"))?,
        None => color.clone(),
    };
    let fill_end = match (value.needle_fill_end_color.as_deref(), value.needle_fill_end_opacity) {
        (Some(end_color), end_opacity) => Some(NeedleFillEnd {
            color: require_hex_color(Some(end_color), &path("needle_fill_end_color"))?,
            opacity: require_opacity(end_opacity, &path("needle_fill_end_opacity"))?,
        }),
        (None, None) => None,
        (None, Some(_)) => {
            return Err(CoreError::Config(format!(
                "{}: requires needle_fill_end_color",
                path("needle_fill_end_opacity")
            )))
        }
    };
    Ok(Some(ValidatedGaugeNeedle {
        length,
        width: require_positive_f32(value.needle_width, &path("needle_width"))?,
        offset: require_f32(value.needle_offset, &path("needle_offset"))?,
        color,
        border_thickness: require_non_negative_f32(
            require_f32(value.needle_border_thickness, &path("needle_border_thickness"))?,
            &path("needle_border_thickness"),
        )?,
        fill_color,
        fill_opacity: require_opacity(value.needle_fill_opacity, &path("needle_fill_opacity"))?,
        fill_end,
        corner_radius: require_non_negative_f32(
            require_f32(value.needle_corner_radius, &path("needle_corner_radius"))?,
            &path("needle_corner_radius"),
        )?,
    }))
}
