//! Optional needle for arc-shaped gauges.
//!
//! The needle is a short rounded bar with a translucent body that sits across the track
//! at the angle of the current value, like the pointer of an analog dial. Setting
//! `needle_length` turns it on; once on, every other needle field is required (the
//! frontend materializes the defaults). It works on automatic and fixed scales alike.
//! Lengths are in unscaled widget pixels.

use super::helpers::{
    require_f32, require_hex_color, require_non_negative_f32, require_opacity, require_positive_f32,
};
use super::raw::ValueConfig;
use crate::error::CoreResult;

/// Validated needle configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedGaugeNeedle {
    /// Size of the needle along the radius.
    pub length: f32,
    /// Size of the needle across the radius.
    pub width: f32,
    /// Shift of the needle center from the middle of the track; positive moves outward.
    pub offset: f32,
    /// Color of the border and of the translucent body.
    pub color: String,
    pub border_thickness: f32,
    /// Opacity of the body (0 leaves only the border).
    pub fill_opacity: f32,
    pub corner_radius: f32,
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
    Ok(Some(ValidatedGaugeNeedle {
        length,
        width: require_positive_f32(value.needle_width, &path("needle_width"))?,
        offset: require_f32(value.needle_offset, &path("needle_offset"))?,
        color: require_hex_color(value.needle_color.as_deref(), &path("needle_color"))?,
        border_thickness: require_non_negative_f32(
            require_f32(value.needle_border_thickness, &path("needle_border_thickness"))?,
            &path("needle_border_thickness"),
        )?,
        fill_opacity: require_opacity(value.needle_fill_opacity, &path("needle_fill_opacity"))?,
        corner_radius: require_non_negative_f32(
            require_f32(value.needle_corner_radius, &path("needle_corner_radius"))?,
            &path("needle_corner_radius"),
        )?,
    }))
}
