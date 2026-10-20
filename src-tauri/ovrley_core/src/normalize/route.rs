//! Route plot validation.
//!
//! `validate_route_plot` verifies that every output-affecting route plot
//! field is explicit. Missing fields are rejected — the backend owns zero
//! render-affecting defaults. The frontend must materialise all defaults
//! before sending the config.

use super::scene::{validate_shadow_override, ShadowOverride};
use super::helpers::{
    normalize_marker_variant, require_f32, require_hex_color, require_non_negative_f32,
    require_opacity, require_positive_f32,
};
use crate::error::{CoreError, CoreResult};
use crate::normalize::raw::{find_plot_values, CoursePlotConfig};

#[derive(Clone, Debug)]
pub struct ValidatedRoutePlot {
    /// Optional per-widget shadow overrides (fall back to the scene).
    pub shadow: ShadowOverride,
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
    pub rotation: f32,
    pub simplify_tolerance_px: f32,
    pub target_density: f32,
    pub completed_line_width: f32,
    pub completed_line_color: String,
    pub completed_line_opacity: f32,
    pub remaining_line_width: f32,
    pub remaining_line_color: String,
    pub remaining_line_opacity: f32,
    pub marker_variant: String,
    pub marker_variant_diameter: f32,
    pub marker_size: f32,
    pub marker_color: String,
    pub marker_opacity: f32,
    pub show_full_activity: bool,
    /// Raster map drawn behind the route; `None` keeps the plain route plot.
    pub map: Option<ValidatedRouteMap>,
}

/// Raster tile map settings of a route plot.
#[derive(Clone, Debug)]
pub struct ValidatedRouteMap {
    /// Tile URL template with `{z}`, `{x}` and `{y}` (`https://` or `file://`).
    pub tile_url: String,
    /// Credit drawn in the corner of the map.
    pub attribution: String,
    pub opacity: f32,
    /// Added to the zoom that matches the widget resolution (sharper when positive).
    pub zoom_offset: i32,
}

/// Largest accepted zoom offset in either direction.
pub const MAX_MAP_ZOOM_OFFSET: i32 = 2;

fn validate_route_map(plot: &CoursePlotConfig, index: usize) -> CoreResult<Option<ValidatedRouteMap>> {
    let p = |f: &str| format!("plots[{index}].{f}");
    let Some(tile_url) = plot.map_tile_url.as_deref() else {
        for (field, present) in [
            ("map_attribution", plot.map_attribution.is_some()),
            ("map_opacity", plot.map_opacity.is_some()),
            ("map_zoom_offset", plot.map_zoom_offset.is_some()),
        ] {
            if present {
                return Err(CoreError::Config(format!("{}: requires map_tile_url", p(field))));
            }
        }
        return Ok(None);
    };
    validate_map_values(tile_url, plot.map_attribution.as_deref(), plot.map_opacity, plot.map_zoom_offset, &p).map(Some)
}

/// Validates the map fields shared by every plot that draws raster tiles.
pub(crate) fn validate_map_values(
    tile_url: &str,
    attribution: Option<&str>,
    opacity: Option<f32>,
    zoom_offset: Option<i32>,
    p: &dyn Fn(&str) -> String,
) -> CoreResult<ValidatedRouteMap> {
    if !(tile_url.starts_with("https://") || tile_url.starts_with("file://")) || !crate::raster_tiles::is_tile_template(tile_url) {
        return Err(CoreError::Config(format!(
            "{}: must be an https:// or file:// URL containing {{z}}, {{x}} and {{y}}",
            p("map_tile_url")
        )));
    }
    let attribution = attribution
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| CoreError::Config(format!("{}: required and must not be empty", p("map_attribution"))))?;
    let zoom_offset = zoom_offset.ok_or_else(|| CoreError::Config(format!("{}: required", p("map_zoom_offset"))))?;
    if !(-MAX_MAP_ZOOM_OFFSET..=MAX_MAP_ZOOM_OFFSET).contains(&zoom_offset) {
        return Err(CoreError::Config(format!(
            "{}: must be between {} and {MAX_MAP_ZOOM_OFFSET}",
            p("map_zoom_offset"),
            -MAX_MAP_ZOOM_OFFSET
        )));
    }
    Ok(ValidatedRouteMap {
        tile_url: tile_url.to_string(),
        attribution: attribution.to_string(),
        opacity: require_opacity(opacity, &p("map_opacity"))?,
        zoom_offset,
    })
}

pub fn validate_route_plots(plots: &serde_json::Value) -> CoreResult<Vec<ValidatedRoutePlot>> {
    find_plot_values(plots, "course")
        .into_iter()
        .map(|(index, value)| {
            serde_json::from_value::<CoursePlotConfig>(value.clone())
                .map_err(|e| CoreError::Config(format!("course plot config: {e}")))
                .and_then(|plot| validate_route_plot(&plot, index))
        })
        .collect()
}

pub fn validate_route_plot(
    plot: &CoursePlotConfig,
    index: usize,
) -> CoreResult<ValidatedRoutePlot> {
    let p = |f: &str| format!("plots[{index}].{f}");

    let simplify_tolerance_px =
        require_f32(plot.simplify_tolerance_px, &p("simplify_tolerance_px"))?;
    require_non_negative_f32(simplify_tolerance_px, &p("simplify_tolerance_px"))?;
    let target_density = require_f32(plot.target_density, &p("target_density"))?;
    if !(0.1..=2.0).contains(&target_density) {
        return Err(CoreError::Config(format!(
            "{}: must be between 0.1 and 2.0",
            p("target_density")
        )));
    }

    let completed_line_width =
        require_positive_f32(plot.completed_line_width, &p("completed_line_width"))?;
    let completed_line_color = require_hex_color(
        plot.completed_line_color.as_deref(),
        &p("completed_line_color"),
    )?;
    let completed_line_opacity =
        require_opacity(plot.completed_line_opacity, &p("completed_line_opacity"))?;

    let remaining_line_width =
        require_positive_f32(plot.remaining_line_width, &p("remaining_line_width"))?;
    let remaining_line_color = require_hex_color(
        plot.remaining_line_color.as_deref(),
        &p("remaining_line_color"),
    )?;
    let remaining_line_opacity =
        require_opacity(plot.remaining_line_opacity, &p("remaining_line_opacity"))?;

    let marker_size = require_positive_f32(plot.marker_size, &p("marker_size"))?;
    let marker_color = require_hex_color(plot.marker_color.as_deref(), &p("marker_color"))?;
    let marker_opacity = require_opacity(plot.marker_opacity, &p("marker_opacity"))?;
    let marker_variant = normalize_marker_variant(
        plot.marker_variant
            .as_deref()
            .ok_or_else(|| CoreError::Config(format!("{}: required", p("marker_variant"))))?,
    );
    let marker_variant_diameter =
        require_positive_f32(plot.marker_variant_diameter, &p("marker_variant_diameter"))?;

    Ok(ValidatedRoutePlot {
        shadow: validate_shadow_override(plot.shadow_strength, plot.shadow_distance, &format!("plots[{index}]"))?,
        x: plot.x,
        y: plot.y,
        width: plot.width,
        height: plot.height,
        rotation: plot.rotation,
        simplify_tolerance_px,
        target_density,
        completed_line_width,
        completed_line_color,
        completed_line_opacity,
        remaining_line_width,
        remaining_line_color,
        remaining_line_opacity,
        marker_variant,
        marker_variant_diameter,
        marker_size,
        marker_color,
        marker_opacity,
        show_full_activity: plot
            .show_full_activity
            .ok_or_else(|| CoreError::Config(format!("{}: required", p("show_full_activity"))))?,
        map: validate_route_map(plot, index)?,
    })
}
