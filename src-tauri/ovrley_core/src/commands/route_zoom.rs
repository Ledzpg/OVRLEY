//! Close-up route map preview command.
//!
//! Renders one frame of the close-up map (tiles, route, arrow and credit) as a PNG the size of
//! the widget, for the editor preview. The frame is chosen by distance progress (0-1) the same
//! way the route preview places its marker; tiles are fetched only for that frame.

use crate::activity::parse_activity_json;
use crate::error::{CoreError, CoreResult};
use crate::normalize::{parse_config_json, validate_route_zoom_plots, validate_scene_config};
use crate::paths::AppPaths;
use crate::render::widgets::route_zoom::{frame_state_at_progress, prepare_route_zoom_preview, render_route_zoom_frame};

/// Renders the close-up map of the first `course_zoom` plot at a distance progress.
pub fn build_route_zoom_frame_png_command(
    paths: &AppPaths,
    config_json: &str,
    parsed_activity_json: &str,
    progress01: f32,
) -> CoreResult<Vec<u8>> {
    let config = parse_config_json(config_json)?;
    let scene = validate_scene_config(config.scene)?;
    let plots = validate_route_zoom_plots(&config.plots)?;
    let activity = parse_activity_json(parsed_activity_json)?;
    let plot = plots
        .first()
        .ok_or_else(|| CoreError::Config("Config has no course_zoom widget".into()))?;

    let mut cache = prepare_route_zoom_preview(&activity, plot, &scene, &paths.font_dirs)?;
    let state = frame_state_at_progress(&cache, progress01);
    let image = render_route_zoom_frame(&mut cache, &state)?;
    let png = image
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .ok_or_else(|| CoreError::Render("Failed to encode the close-up map PNG".into()))?;
    Ok(png.as_bytes().to_vec())
}
