//! Route geometry IPC command.
//!
//! Provides `build_route_geometry_command` — a pure-geometry command that
//! accepts serialized config + activity JSON and returns pre-built route
//! widget geometry for the frontend preview. No Skia surfaces, no static
//! layers, no frame states.

use crate::activity::parse_activity_json;
use crate::error::{CoreError, CoreResult};
use crate::normalize::{parse_config_json, validate_route_plots, validate_scene_config};
use serde::Serialize;

/// Serializable geometry model for the route widget, returned over IPC.
///
/// The JS frontend consumes this directly for SVG rendering without
/// re-computing geometry locally.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteGeometryResponse {
    /// Projected (x, y) points in widget-local coordinates.
    pub points: Vec<[f32; 2]>,
    /// Per-point progress values (0.0..=1.0) for marker interpolation.
    pub progress_values: Vec<f32>,
    /// Bounding box [min_x, min_y, max_x, max_y].
    pub bbox: [f32; 4],
    /// Number of raw source route samples before simplification.
    pub source_point_count: usize,
    /// Simplification label for diagnostics.
    pub simplification: String,
    /// Widget width in scaled pixels.
    pub widget_width: u32,
    /// Widget height in scaled pixels.
    pub widget_height: u32,
}

/// Builds route widget geometry from serialized config and activity JSON.
///
/// This is a pure-geometry command: no Skia surfaces, no static layers, no
/// frame states. It performs trimming, Mercator projection, LTTB downsampling,
/// and RDP simplification only.
pub fn build_route_geometry_command(
    config_json: &str,
    parsed_activity_json: &str,
) -> CoreResult<RouteGeometryResponse> {
    let config = parse_config_json(config_json)?;
    let scene = validate_scene_config(config.scene)?;
    let course_plots = validate_route_plots(&config.plots)?;
    let activity = parse_activity_json(parsed_activity_json)?;

    let course_plot = course_plots
        .first()
        .ok_or_else(|| CoreError::Config("Config has no course_plot widget".into()))?;

    let show_full_activity = course_plot.show_full_activity;

    let route_samples = crate::render::widgets::route::prepare::build_route_samples(
        &activity,
        show_full_activity,
        course_plot.map.is_some(),
        &scene,
    )?;

    let normalized =
        crate::render::widgets::route::normalize::normalize_route_plot(course_plot, &scene);

    let geometry = crate::render::widgets::route::prepare::build_route_geometry(
        &normalized,
        course_plot,
        &route_samples,
    )?;

    Ok(RouteGeometryResponse {
        points: geometry.points.into_iter().map(|(x, y)| [x, y]).collect(),
        progress_values: geometry.progress_values,
        bbox: [
            geometry.bbox.0,
            geometry.bbox.1,
            geometry.bbox.2,
            geometry.bbox.3,
        ],
        source_point_count: geometry.source_point_count,
        simplification: geometry.simplification,
        widget_width: normalized.width,
        widget_height: normalized.height,
    })
}

/// Renders the raster map behind the first course plot as a PNG in unrotated plot space.
///
/// Returns `None` when the plot has no map configured. The image has the same pixel size
/// as the geometry response (`widget_width` x `widget_height`) and no attribution, which
/// the preview draws itself.
pub fn build_route_map_png_command(
    config_json: &str,
    parsed_activity_json: &str,
) -> CoreResult<Option<Vec<u8>>> {
    let config = parse_config_json(config_json)?;
    let scene = validate_scene_config(config.scene)?;
    let course_plots = validate_route_plots(&config.plots)?;
    let activity = parse_activity_json(parsed_activity_json)?;
    let course_plot = course_plots
        .first()
        .ok_or_else(|| CoreError::Config("Config has no course_plot widget".into()))?;
    let Some(map) = &course_plot.map else {
        return Ok(None);
    };

    let route_samples = crate::render::widgets::route::prepare::build_route_samples(
        &activity,
        course_plot.show_full_activity,
        true,
        &scene,
    )?;
    let normalized =
        crate::render::widgets::route::normalize::normalize_route_plot(course_plot, &scene);
    let (fit, output_scale) = crate::render::widgets::route::prepare::route_map_fit(
        &normalized,
        course_plot,
        &route_samples,
    )?;
    let image = crate::render::widgets::route::map::build_map_image(
        map,
        &fit,
        output_scale,
        normalized.width,
        normalized.height,
    )?;
    let png = image
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .ok_or_else(|| CoreError::Render("Failed to encode the route map PNG".into()))?;
    Ok(Some(png.as_bytes().to_vec()))
}
