//! Preparation of the close-up map: normalized settings, the route in tile pixels, the position
//! and heading of every frame, and the tiles those frames show.

use super::super::common::{
    custom_export_range_active, frame_progress_values, relative_distance_frame_progress_values,
};
use super::super::route::map::build_map_attribution_layer;
use super::super::route::prepare::build_route_samples;
use super::super::types::RouteSample;
use super::geometry::{
    blend_heading, heading_at, heading_for_preview, heading_reach, position_at, to_tile_px, view_scale, ViewScale,
};
use super::tiles::{plan_tiles, tiles_around, TileStore};
use crate::activity::schema::{DenseActivityReport, ParsedActivity};
use crate::debug::RenderProfiler;
use crate::error::{CoreError, CoreResult};
use crate::normalize::{ValidatedRouteMap, ValidatedRouteZoomPlot, ValidatedSceneConfig};
use skia_safe::Image;
use std::path::PathBuf;

/// Points per culling chunk of the route polyline.
pub(super) const CHUNK_POINTS: usize = 32;
/// Time constant (seconds) of the smoothing applied to the heading over frames.
const HEADING_SMOOTHING_SECONDS: f64 = 0.35;
const DEFAULT_FRAME_SECONDS: f64 = 1.0 / 30.0;

/// Close-up map settings scaled to output pixels.
#[derive(Clone, Debug)]
pub(crate) struct ZoomPlot {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) corner_radius: f32,
    pub(crate) rotate_with_heading: bool,
    pub(crate) map: ValidatedRouteMap,
    pub(crate) completed_line_width: f32,
    pub(crate) completed_line_color: String,
    pub(crate) completed_line_opacity: f32,
    pub(crate) remaining_line_width: f32,
    pub(crate) remaining_line_color: String,
    pub(crate) remaining_line_opacity: f32,
    pub(crate) marker_size: f32,
    pub(crate) marker_color: String,
    pub(crate) marker_opacity: f32,
    pub(crate) marker_outline_color: String,
    pub(crate) marker_outline_width: f32,
}

/// Where one frame looks: the position (tile pixels), the direction of travel and the index of the
/// route sample the marker is heading toward.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ZoomFrameState {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) heading: f32,
    pub(crate) next: usize,
}

/// Bounding box of a chunk of route points (tile pixels).
#[derive(Clone, Copy, Debug)]
pub(super) struct ChunkBox {
    pub(super) min: (f64, f64),
    pub(super) max: (f64, f64),
}

/// Prepared close-up map.
#[derive(Clone, Debug)]
pub(crate) struct RouteZoomCache {
    pub(crate) plot: ZoomPlot,
    pub(crate) scale: ViewScale,
    pub(crate) samples: Vec<RouteSample>,
    pub(super) route_px: Vec<(f64, f64)>,
    pub(super) chunks: Vec<ChunkBox>,
    pub(crate) store: TileStore,
    pub(crate) credit: Image,
    pub(crate) frame_states: Vec<ZoomFrameState>,
}

fn normalize(plot: &ValidatedRouteZoomPlot, scale: f32) -> ZoomPlot {
    let scaled = |value: f32| value * scale;
    ZoomPlot {
        x: plot.x,
        y: plot.y,
        width: ((plot.width as f32) * scale).round().max(1.0) as u32,
        height: ((plot.height as f32) * scale).round().max(1.0) as u32,
        corner_radius: scaled(plot.corner_radius),
        rotate_with_heading: plot.rotate_with_heading,
        map: plot.map.clone(),
        completed_line_width: scaled(plot.completed_line_width),
        completed_line_color: plot.completed_line_color.clone(),
        completed_line_opacity: plot.completed_line_opacity,
        remaining_line_width: scaled(plot.remaining_line_width),
        remaining_line_color: plot.remaining_line_color.clone(),
        remaining_line_opacity: plot.remaining_line_opacity,
        marker_size: scaled(plot.marker_size),
        marker_color: plot.marker_color.clone(),
        marker_opacity: plot.marker_opacity,
        marker_outline_color: plot.marker_outline_color.clone(),
        marker_outline_width: scaled(plot.marker_outline_width),
    }
}

/// Builds the cache without frames or tiles: enough to look at one position.
fn prepare_core(
    samples: Vec<RouteSample>,
    validated: &ValidatedRouteZoomPlot,
    scene: &ValidatedSceneConfig,
    font_dirs: &[PathBuf],
) -> CoreResult<RouteZoomCache> {
    if samples.len() < 2 {
        return Err(CoreError::Render("Close-up map requires at least two valid course points".into()));
    }
    let output_scale = f64::from(scene.presentation.scale.max(0.1));
    let plot = normalize(validated, scene.presentation.scale.max(0.1));
    let view = view_scale(validated.zoom, validated.map.zoom_offset, output_scale);
    let route_px = samples
        .iter()
        .map(|sample| to_tile_px(sample.point.0, sample.point.1, view.tile_zoom))
        .collect::<Vec<_>>();
    let chunks = route_px
        .chunks(CHUNK_POINTS)
        .enumerate()
        .map(|(index, points)| {
            // Overlap by one point so a segment crossing a chunk boundary belongs to both chunks.
            let end = route_px.get(index * CHUNK_POINTS + points.len()).copied();
            let mut min = (f64::INFINITY, f64::INFINITY);
            let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for point in points.iter().copied().chain(end) {
                min = (min.0.min(point.0), min.1.min(point.1));
                max = (max.0.max(point.0), max.1.max(point.1));
            }
            ChunkBox { min, max }
        })
        .collect();
    let credit = build_map_attribution_layer(plot.width, plot.height, &plot.map.attribution, output_scale as f32, font_dirs)?;
    Ok(RouteZoomCache {
        store: TileStore::new(&plot.map.tile_url, view.tile_zoom),
        plot,
        scale: view,
        samples,
        route_px,
        chunks,
        credit,
        frame_states: Vec::new(),
    })
}

/// Radius (tile pixels) of the circle that contains the widget however the map is turned.
pub(crate) fn view_radius(cache: &RouteZoomCache) -> f64 {
    let half_diagonal = f64::from(cache.plot.width).hypot(f64::from(cache.plot.height)) / 2.0;
    half_diagonal / cache.scale.draw_scale + 2.0
}

/// Tiles a frame needs.
pub(crate) fn tiles_for_state(cache: &RouteZoomCache, state: &ZoomFrameState) -> Vec<(u32, u32)> {
    tiles_around((state.x, state.y), view_radius(cache), cache.scale.tile_zoom)
}

/// Prepares the close-up map of a video render: every frame's state and every tile they show.
pub(crate) fn prepare_route_zoom_cache(
    activity: &ParsedActivity,
    dense_activity: &DenseActivityReport,
    validated: &ValidatedRouteZoomPlot,
    scene: &ValidatedSceneConfig,
    font_dirs: &[PathBuf],
    prepare_profiler: &mut RenderProfiler,
) -> CoreResult<RouteZoomCache> {
    prepare_profiler.measure("build_route_zoom_cache", || {
        let samples = build_route_samples(activity, validated.show_full_activity, true, scene)?;
        let mut cache = prepare_core(samples, validated, scene, font_dirs)?;
        let frame_progress = if custom_export_range_active(scene) && !validated.show_full_activity {
            relative_distance_frame_progress_values(activity, dense_activity, scene)
                .unwrap_or_else(|| frame_progress_values(activity, dense_activity, scene))
        } else {
            frame_progress_values(activity, dense_activity, scene)
        };
        cache.frame_states = build_frame_states(&cache, &frame_progress, &dense_activity.frame_elapsed_seconds);
        let tiles = plan_tiles(
            cache.frame_states.iter().map(|state| (state.x, state.y)),
            view_radius(&cache),
            cache.scale.tile_zoom,
        )?;
        cache.store.ensure(&tiles)?;
        Ok(cache)
    })
}

/// Prepares the close-up map for single preview frames: no frame list and no tile download until
/// a frame asks for its tiles.
pub(crate) fn prepare_route_zoom_preview(
    activity: &ParsedActivity,
    validated: &ValidatedRouteZoomPlot,
    scene: &ValidatedSceneConfig,
    font_dirs: &[PathBuf],
) -> CoreResult<RouteZoomCache> {
    let samples = build_route_samples(activity, validated.show_full_activity, true, scene)?;
    prepare_core(samples, validated, scene, font_dirs)
}

/// State of one preview frame at a distance progress (heading measured without smoothing).
pub(crate) fn frame_state_at_progress(cache: &RouteZoomCache, progress: f32) -> ZoomFrameState {
    let (x, y, next) = position_at(&cache.samples, progress.clamp(0.0, 1.0));
    let heading = heading_for_preview(&cache.samples, (x, y), next, &cache.scale);
    let (tile_x, tile_y) = to_tile_px(x, y, cache.scale.tile_zoom);
    ZoomFrameState { x: tile_x, y: tile_y, heading, next }
}

/// Position and (smoothed) heading of every frame.
fn build_frame_states(cache: &RouteZoomCache, frame_progress: &[f32], frame_seconds: &[f64]) -> Vec<ZoomFrameState> {
    let reach = heading_reach(&cache.scale);
    let mut heading = None::<f32>;
    frame_progress
        .iter()
        .enumerate()
        .map(|(frame, progress)| {
            let (x, y, next) = position_at(&cache.samples, progress.clamp(0.0, 1.0));
            let target = heading_at(&cache.samples, (x, y), next, reach);
            let dt = match (frame_seconds.get(frame), frame.checked_sub(1).and_then(|prev| frame_seconds.get(prev))) {
                (Some(now), Some(before)) if now > before => now - before,
                _ => DEFAULT_FRAME_SECONDS,
            };
            let amount = (1.0 - (-dt / HEADING_SMOOTHING_SECONDS).exp()) as f32;
            // Standing still keeps the last heading; the first frame starts at its own direction.
            let current = match (heading, target) {
                (Some(previous), Some(target)) => blend_heading(previous, target, amount),
                (Some(previous), None) => previous,
                (None, Some(target)) => target,
                (None, None) => 0.0,
            };
            heading = Some(current);
            let (tile_x, tile_y) = to_tile_px(x, y, cache.scale.tile_zoom);
            ZoomFrameState { x: tile_x, y: tile_y, heading: current, next }
        })
        .collect()
}
