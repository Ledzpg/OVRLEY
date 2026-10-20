//! Projection, view scale and movement along the route for the close-up map.

use super::super::types::RouteSample;

pub(crate) const TILE_SIZE: f64 = 256.0;
const MAX_TILE_ZOOM: i32 = 19;
/// Distance (in output pixels) over which the direction of travel is measured.
const HEADING_REACH_PX: f64 = 40.0;

/// Size of the world in pixels at a tile zoom.
pub(crate) fn world_px(tile_zoom: u32) -> f64 {
    TILE_SIZE * 2f64.powi(tile_zoom as i32)
}

/// Projects Mercator degrees (see `route::map::mercator_degrees`) to world pixels at a tile zoom.
pub(crate) fn to_tile_px(x: f64, y: f64, tile_zoom: u32) -> (f64, f64) {
    let world = world_px(tile_zoom);
    (x / 360.0 * world, (0.5 - y / 360.0) * world)
}

/// How the view relates to the tiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ViewScale {
    /// Zoom level of the tiles that are downloaded.
    pub(crate) tile_zoom: u32,
    /// Output pixels per tile pixel.
    pub(crate) draw_scale: f64,
    /// Output pixels per Mercator degree.
    pub(crate) pixels_per_degree: f64,
}

/// Picks the tile zoom whose pixels match the output resolution (plus the detail offset) and the
/// scale that makes the tiles show the requested view zoom.
pub(crate) fn view_scale(zoom: f32, detail_offset: i32, output_scale: f64) -> ViewScale {
    let zoom = f64::from(zoom);
    let matching = (zoom + output_scale.log2()).round() as i32;
    let tile_zoom = (matching + detail_offset).clamp(0, MAX_TILE_ZOOM) as u32;
    ViewScale {
        tile_zoom,
        draw_scale: 2f64.powf(zoom - f64::from(tile_zoom)) * output_scale,
        pixels_per_degree: TILE_SIZE * 2f64.powf(zoom) * output_scale / 360.0,
    }
}

/// Position along the route at a distance progress, with the index of the sample the marker is
/// heading toward (`samples.len()` once the end is reached).
pub(crate) fn position_at(samples: &[RouteSample], progress: f32) -> (f64, f64, usize) {
    let next = samples.partition_point(|sample| sample.progress01 < progress);
    if next == 0 {
        return (samples[0].point.0, samples[0].point.1, 0);
    }
    if next >= samples.len() {
        let last = samples[samples.len() - 1].point;
        return (last.0, last.1, samples.len());
    }
    let (from, to) = (samples[next - 1], samples[next]);
    let span = f64::from(to.progress01 - from.progress01);
    let mix = if span > 1e-9 { f64::from(progress - from.progress01) / span } else { 0.0 };
    (
        from.point.0 + (to.point.0 - from.point.0) * mix,
        from.point.1 + (to.point.1 - from.point.1) * mix,
        next,
    )
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Bearing (degrees clockwise from north) of the route around a position, measured between the
/// route points about `reach` Mercator degrees behind and ahead. `None` when the route does not
/// move there (standing still).
pub(crate) fn heading_at(samples: &[RouteSample], position: (f64, f64), next: usize, reach: f64) -> Option<f32> {
    let mut behind = position;
    let mut walked = 0.0;
    for index in (0..next.min(samples.len())).rev() {
        walked += distance(behind, samples[index].point);
        behind = samples[index].point;
        if walked >= reach {
            break;
        }
    }
    let mut ahead = position;
    let mut walked = 0.0;
    for sample in samples.iter().skip(next) {
        walked += distance(ahead, sample.point);
        ahead = sample.point;
        if walked >= reach {
            break;
        }
    }
    let (dx, dy) = (ahead.0 - behind.0, ahead.1 - behind.1);
    if dx.abs() + dy.abs() < reach * 1e-3 {
        return None;
    }
    Some(dx.atan2(dy).to_degrees().rem_euclid(360.0) as f32)
}

/// Heading of a single preview frame (no smoothing over time; north when standing still).
pub(crate) fn heading_for_preview(samples: &[RouteSample], position: (f64, f64), next: usize, scale: &ViewScale) -> f32 {
    heading_at(samples, position, next, HEADING_REACH_PX / scale.pixels_per_degree).unwrap_or(0.0)
}

/// Reach used by the per-frame heading, exposed for preparation.
pub(crate) fn heading_reach(scale: &ViewScale) -> f64 {
    HEADING_REACH_PX / scale.pixels_per_degree
}

/// Moves `current` toward `target` along the shortest way round (degrees).
pub(crate) fn blend_heading(current: f32, target: f32, amount: f32) -> f32 {
    let delta = (target - current + 540.0).rem_euclid(360.0) - 180.0;
    (current + delta * amount).rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(points: &[(f64, f64)]) -> Vec<RouteSample> {
        let last = (points.len() - 1) as f32;
        points
            .iter()
            .enumerate()
            .map(|(index, point)| RouteSample { point: *point, progress01: index as f32 / last })
            .collect()
    }

    #[test]
    fn the_view_zoom_sets_the_scale_and_the_detail_offset_only_the_tiles() {
        let plain = view_scale(15.0, 0, 1.0);
        assert_eq!(plain.tile_zoom, 15);
        assert!((plain.draw_scale - 1.0).abs() < 1e-9);
        let sharper = view_scale(15.0, 1, 1.0);
        assert_eq!(sharper.tile_zoom, 16);
        assert!((sharper.draw_scale - 0.5).abs() < 1e-9, "tiles are drawn smaller, the view does not change");
        assert_eq!(plain.pixels_per_degree, sharper.pixels_per_degree);
        // A closer zoom shows less ground: twice the pixels per degree.
        assert!((view_scale(16.0, 0, 1.0).pixels_per_degree / plain.pixels_per_degree - 2.0).abs() < 1e-9);
        // The scene scale grows the widget, so the same view takes proportionally more pixels.
        let doubled = view_scale(15.0, 0, 2.0);
        assert_eq!(doubled.tile_zoom, 16);
        assert!((doubled.pixels_per_degree / plain.pixels_per_degree - 2.0).abs() < 1e-9);
    }

    #[test]
    fn the_tile_zoom_is_clamped_to_the_map() {
        assert_eq!(view_scale(20.0, 2, 1.0).tile_zoom, 19);
        assert_eq!(view_scale(8.0, -2, 0.25).tile_zoom, 4);
        assert!(view_scale(20.0, 2, 1.0).draw_scale > 1.0, "missing detail is upscaled, not dropped");
    }

    #[test]
    fn projection_puts_the_origin_in_the_middle_of_the_world() {
        let (x, y) = to_tile_px(180.0, 0.0, 2);
        assert_eq!((x, y), (512.0, 512.0));
        let (x, y) = to_tile_px(0.0, 180.0, 0);
        assert_eq!((x, y), (0.0, 0.0));
    }

    #[test]
    fn the_position_follows_the_progress() {
        let samples = route(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        assert_eq!(position_at(&samples, 0.0), (0.0, 0.0, 0));
        let (x, y, next) = position_at(&samples, 0.25);
        assert!((x - 5.0).abs() < 1e-9 && y.abs() < 1e-9 && next == 1);
        let (x, y, next) = position_at(&samples, 0.75);
        assert!((x - 10.0).abs() < 1e-9 && (y - 5.0).abs() < 1e-9 && next == 2);
        assert_eq!(position_at(&samples, 1.0).2, 2, "the last sample is still the target at full progress");
        assert_eq!(position_at(&samples, 2.0), (10.0, 10.0, 3), "past the end the marker stays on the last sample");
    }

    #[test]
    fn the_heading_is_the_bearing_of_the_route() {
        let east = route(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]);
        let (x, y, next) = position_at(&east, 0.5);
        assert!((heading_at(&east, (x, y), next, 0.3).unwrap() - 90.0).abs() < 1e-3);
        let north = route(&[(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)]);
        let (x, y, next) = position_at(&north, 0.5);
        assert!(heading_at(&north, (x, y), next, 0.3).unwrap().abs() < 1e-3);
        let south_west = route(&[(2.0, 2.0), (1.0, 1.0), (0.0, 0.0)]);
        let (x, y, next) = position_at(&south_west, 0.5);
        assert!((heading_at(&south_west, (x, y), next, 0.3).unwrap() - 225.0).abs() < 1e-3);
    }

    #[test]
    fn standing_still_has_no_heading() {
        let still = route(&[(1.0, 1.0), (1.0, 1.0), (1.0, 1.0)]);
        let (x, y, next) = position_at(&still, 0.5);
        assert_eq!(heading_at(&still, (x, y), next, 0.3), None);
    }

    #[test]
    fn headings_blend_the_short_way_round() {
        assert!((blend_heading(350.0, 10.0, 0.5) - 0.0).abs() < 1e-4 || (blend_heading(350.0, 10.0, 0.5) - 360.0).abs() < 1e-4);
        assert!((blend_heading(10.0, 350.0, 0.5) - 0.0).abs() < 1e-4 || (blend_heading(10.0, 350.0, 0.5) - 360.0).abs() < 1e-4);
        assert!((blend_heading(0.0, 90.0, 1.0) - 90.0).abs() < 1e-4);
    }
}
