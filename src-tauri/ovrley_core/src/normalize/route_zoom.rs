//! Close-up route map validation.
//!
//! The close-up map (`value: "course_zoom"`) follows the current position at a fixed zoom,
//! like the zoomed map of a navigation app. Every output-affecting field is required.

use super::helpers::{
    require_bool, require_f32, require_hex_color, require_non_negative_f32, require_opacity,
    require_positive_f32,
};
use super::raw::{find_plot_values, CourseZoomPlotConfig};
use super::route::{validate_map_values, ValidatedRouteMap};
use crate::error::{CoreError, CoreResult};

/// Lowest and highest accepted view zoom (web map zoom levels).
pub const MIN_ZOOM_VIEW: f32 = 8.0;
pub const MAX_ZOOM_VIEW: f32 = 20.0;

#[derive(Clone, Debug)]
pub struct ValidatedRouteZoomPlot {
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
    /// View zoom: how much ground the widget shows (higher = closer).
    pub zoom: f32,
    pub corner_radius: f32,
    /// Rotate the map so the direction of travel points up; otherwise north stays up.
    pub rotate_with_heading: bool,
    pub show_full_activity: bool,
    pub map: ValidatedRouteMap,
    pub completed_line_width: f32,
    pub completed_line_color: String,
    pub completed_line_opacity: f32,
    pub remaining_line_width: f32,
    pub remaining_line_color: String,
    pub remaining_line_opacity: f32,
    /// Length of the arrow marker.
    pub marker_size: f32,
    pub marker_color: String,
    pub marker_opacity: f32,
    pub marker_outline_color: String,
    pub marker_outline_width: f32,
}

pub fn validate_route_zoom_plots(plots: &serde_json::Value) -> CoreResult<Vec<ValidatedRouteZoomPlot>> {
    find_plot_values(plots, "course_zoom")
        .into_iter()
        .map(|(index, value)| {
            serde_json::from_value::<CourseZoomPlotConfig>(value.clone())
                .map_err(|e| CoreError::Config(format!("course_zoom plot config: {e}")))
                .and_then(|plot| validate_route_zoom_plot(&plot, index))
        })
        .collect()
}

pub fn validate_route_zoom_plot(plot: &CourseZoomPlotConfig, index: usize) -> CoreResult<ValidatedRouteZoomPlot> {
    let p = |f: &str| format!("plots[{index}].{f}");
    let zoom = require_f32(plot.zoom, &p("zoom"))?;
    if !(MIN_ZOOM_VIEW..=MAX_ZOOM_VIEW).contains(&zoom) {
        return Err(CoreError::Config(format!(
            "{}: must be between {MIN_ZOOM_VIEW} and {MAX_ZOOM_VIEW}",
            p("zoom")
        )));
    }
    let tile_url = plot
        .map_tile_url
        .as_deref()
        .ok_or_else(|| CoreError::Config(format!("{}: required", p("map_tile_url"))))?;
    Ok(ValidatedRouteZoomPlot {
        x: plot.x,
        y: plot.y,
        width: plot.width,
        height: plot.height,
        zoom,
        corner_radius: require_non_negative_f32(
            require_f32(plot.corner_radius, &p("corner_radius"))?,
            &p("corner_radius"),
        )?,
        rotate_with_heading: require_bool(plot.rotate_with_heading, &p("rotate_with_heading"))?,
        show_full_activity: require_bool(plot.show_full_activity, &p("show_full_activity"))?,
        map: validate_map_values(
            tile_url,
            plot.map_attribution.as_deref(),
            plot.map_opacity,
            plot.map_zoom_offset,
            &p,
        )?,
        completed_line_width: require_positive_f32(plot.completed_line_width, &p("completed_line_width"))?,
        completed_line_color: require_hex_color(plot.completed_line_color.as_deref(), &p("completed_line_color"))?,
        completed_line_opacity: require_opacity(plot.completed_line_opacity, &p("completed_line_opacity"))?,
        remaining_line_width: require_positive_f32(plot.remaining_line_width, &p("remaining_line_width"))?,
        remaining_line_color: require_hex_color(plot.remaining_line_color.as_deref(), &p("remaining_line_color"))?,
        remaining_line_opacity: require_opacity(plot.remaining_line_opacity, &p("remaining_line_opacity"))?,
        marker_size: require_positive_f32(plot.marker_size, &p("marker_size"))?,
        marker_color: require_hex_color(plot.marker_color.as_deref(), &p("marker_color"))?,
        marker_opacity: require_opacity(plot.marker_opacity, &p("marker_opacity"))?,
        marker_outline_color: require_hex_color(plot.marker_outline_color.as_deref(), &p("marker_outline_color"))?,
        marker_outline_width: require_non_negative_f32(
            require_f32(plot.marker_outline_width, &p("marker_outline_width"))?,
            &p("marker_outline_width"),
        )?,
    })
}
