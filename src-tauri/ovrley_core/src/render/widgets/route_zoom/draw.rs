//! Per-frame drawing of the close-up map.
//!
//! Coordinates are drawn relative to the view center (tile pixels minus the center, subtracted in
//! f64): absolute tile pixels reach the hundreds of millions at high zooms, more than f32 resolves.

use super::super::marker::draw_marker_path;
use super::super::polyline::draw_polyline;
use super::super::route::map::map_sampling;
use super::prepare::{tiles_for_state, view_radius, RouteZoomCache, ZoomFrameState, CHUNK_POINTS};
use super::geometry::TILE_SIZE;
use super::tiles::tiles_around;
use crate::error::CoreResult;
use crate::render::surface::create_surface;
use skia_safe::{Canvas, ClipOp, Color, Image, Paint, Point, RRect, Rect};

/// Draws the close-up map for one video frame.
pub(crate) fn draw_route_zoom_widget(canvas: &Canvas, cache: &RouteZoomCache, frame_index: usize) -> bool {
    let Some(state) = cache.frame_states.get(frame_index.min(cache.frame_states.len().saturating_sub(1))) else {
        return false;
    };
    draw_frame(canvas, cache, state, (cache.plot.x, cache.plot.y));
    true
}

/// Renders one frame on its own transparent surface the size of the widget (for the editor
/// preview), downloading the tiles it needs first.
pub(crate) fn render_route_zoom_frame(cache: &mut RouteZoomCache, state: &ZoomFrameState) -> CoreResult<Image> {
    let wanted = tiles_for_state(cache, state);
    cache.store.ensure(&wanted)?;
    let mut surface = create_surface(cache.plot.width, cache.plot.height)?;
    surface.canvas().clear(Color::TRANSPARENT);
    draw_frame(surface.canvas(), cache, state, (0.0, 0.0));
    Ok(surface.image_snapshot())
}

fn draw_frame(canvas: &Canvas, cache: &RouteZoomCache, state: &ZoomFrameState, origin: (f32, f32)) {
    let plot = &cache.plot;
    let (width, height) = (plot.width as f32, plot.height as f32);
    let center = Point::new(width / 2.0, height / 2.0);
    let scale = cache.scale.draw_scale as f32;
    let rotation = if plot.rotate_with_heading { -state.heading } else { 0.0 };

    canvas.save();
    canvas.translate(origin);
    let corner = plot.corner_radius.min(width.min(height) / 2.0);
    canvas.clip_rrect(RRect::new_rect_xy(Rect::from_xywh(0.0, 0.0, width, height), corner, corner), ClipOp::Intersect, true);

    // The map layer: tiles, then the route, turned and scaled around the widget center.
    canvas.save();
    canvas.translate(center);
    canvas.rotate(rotation, None);
    canvas.scale((scale, scale));
    draw_tiles(canvas, cache, state);
    draw_route(canvas, cache, state, scale);
    canvas.restore();

    // The marker stays at the center; on a north-up map it turns to the direction of travel.
    let marker_heading = if plot.rotate_with_heading { 0.0 } else { state.heading };
    draw_marker_path(
        canvas,
        center,
        marker_heading,
        plot.marker_size,
        &plot.marker_color,
        plot.marker_opacity,
        &plot.marker_outline_color,
        plot.marker_outline_width,
    );
    canvas.draw_image(&cache.credit, (0.0, 0.0), None);
    canvas.restore();
}

fn draw_tiles(canvas: &Canvas, cache: &RouteZoomCache, state: &ZoomFrameState) {
    let slack = 0.5 / cache.scale.draw_scale as f32;
    let mut paint = Paint::default();
    paint.set_anti_alias(false);
    canvas.save_layer_alpha_f(None, cache.plot.map.opacity);
    for (x, y) in tiles_around((state.x, state.y), view_radius(cache), cache.scale.tile_zoom) {
        let Some(tile) = cache.store.get(x, y) else { continue };
        let left = (f64::from(x) * TILE_SIZE - state.x) as f32;
        let top = (f64::from(y) * TILE_SIZE - state.y) as f32;
        // A hair of overlap hides the seams between tiles drawn at fractional positions.
        let target = Rect::from_xywh(left - slack, top - slack, TILE_SIZE as f32 + slack * 2.0, TILE_SIZE as f32 + slack * 2.0);
        canvas.draw_image_rect_with_sampling_options(tile, None, target, map_sampling(), &paint);
    }
    canvas.restore();
}

/// Draws the route parts near the view, finished part under the marker's feet in its own style.
fn draw_route(canvas: &Canvas, cache: &RouteZoomCache, state: &ZoomFrameState, scale: f32) {
    let plot = &cache.plot;
    let radius = view_radius(cache);
    let (lo, hi) = ((state.x - radius, state.y - radius), (state.x + radius, state.y + radius));
    let relative = |point: (f64, f64)| ((point.0 - state.x) as f32, (point.1 - state.y) as f32);
    let marker = (0.0f32, 0.0f32);
    let next = state.next;

    // Visible index ranges, merged across neighbouring chunks.
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for (index, chunk) in cache.chunks.iter().enumerate() {
        if chunk.max.0 < lo.0 || chunk.min.0 > hi.0 || chunk.max.1 < lo.1 || chunk.min.1 > hi.1 {
            continue;
        }
        let start = index * CHUNK_POINTS;
        let end = ((index + 1) * CHUNK_POINTS).min(cache.route_px.len() - 1);
        match ranges.last_mut() {
            Some(last) if start <= last.1 => last.1 = end,
            _ => ranges.push((start, end)),
        }
    }

    let remaining_width = plot.remaining_line_width / scale;
    let completed_width = plot.completed_line_width / scale;
    let mut remaining = Vec::new();
    let mut completed = Vec::new();
    for (start, end) in ranges {
        let points = |from: usize, to: usize| cache.route_px[from..=to].iter().map(|point| relative(*point));
        if end < next {
            completed.push(points(start, end).collect::<Vec<_>>());
        } else if start >= next {
            remaining.push(points(start, end).collect::<Vec<_>>());
        } else {
            completed.push(points(start, next - 1).chain(std::iter::once(marker)).collect());
            remaining.push(std::iter::once(marker).chain(points(next, end)).collect());
        }
    }
    // The remaining route first, so the finished part is drawn over it where they meet.
    for line in &remaining {
        draw_polyline(canvas, line, &plot.remaining_line_color, remaining_width, plot.remaining_line_opacity);
    }
    for line in &completed {
        draw_polyline(canvas, line, &plot.completed_line_color, completed_width, plot.completed_line_opacity);
    }
}
