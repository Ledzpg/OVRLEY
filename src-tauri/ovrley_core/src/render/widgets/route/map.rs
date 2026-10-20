/// Raster tile map drawn behind the route plot.
///
/// The route is projected with Web Mercator ("degrees": x = longitude + 180, y = the
/// Mercator ordinate in degrees, so the world is 360 x 360) whenever a map is configured.
/// The zoom is the one whose tile pixels match the widget resolution; the covered tiles
/// are stitched into a mosaic and drawn once through the route's own fit transform, which
/// keeps the line exactly on the streets.
use super::super::types::{NormalizedRoutePlot, StaticLayer};
use super::fit::RouteFit;
use super::prepare::rotated_layer_bounds;
use crate::error::{CoreError, CoreResult};
use crate::normalize::ValidatedRouteMap;
use crate::raster_tiles::{expand_tile_url, fetch_tile};
use crate::render::surface::create_surface;
use skia_safe::{
    Color, Data, FilterMode, Font, Image, Matrix, MipmapMode, Paint, Rect, SamplingOptions,
};
use std::path::PathBuf;

/// World width in Mercator degrees.
const WORLD_DEGREES: f64 = 360.0;
const TILE_SIZE: u32 = 256;
const MAX_ZOOM: i32 = 19;
/// Upper bound on tiles per map; the zoom drops until the view fits.
const MAX_TILES: u32 = 64;
const MAX_MERCATOR_LATITUDE: f64 = 85.051_128_78;
const FETCH_WORKERS: usize = 6;
const ATTRIBUTION_FONT_PX: f32 = 11.0;
const ATTRIBUTION_PADDING_PX: f32 = 3.0;

/// Projects a latitude/longitude to Mercator degrees (x east, y north).
pub(crate) fn mercator_degrees(lat: f64, lon: f64) -> (f64, f64) {
    let lat = lat
        .clamp(-MAX_MERCATOR_LATITUDE, MAX_MERCATOR_LATITUDE)
        .to_radians();
    (
        lon + 180.0,
        (std::f64::consts::FRAC_PI_4 + lat / 2.0)
            .tan()
            .ln()
            .to_degrees(),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TileWindow {
    zoom: u32,
    x0: u32,
    y0: u32,
    columns: u32,
    rows: u32,
}

/// Mercator degrees of the window corners, as (min_x, max_x, min_y, max_y).
fn visible_mercator(fit: &RouteFit, output_scale: f64, out_w: u32, out_h: u32) -> (f64, f64, f64, f64) {
    let corners = [
        (0.0, 0.0),
        (out_w as f64, 0.0),
        (0.0, out_h as f64),
        (out_w as f64, out_h as f64),
    ]
    .map(|(x, y)| fit.unapply((x / output_scale, y / output_scale)));
    let xs = corners.map(|corner| corner.0);
    let ys = corners.map(|corner| corner.1);
    (
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ys.iter().copied().fold(f64::INFINITY, f64::min),
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}

/// Picks the zoom whose tile pixels match the output resolution (plus the offset), lowered
/// until the visible window needs at most `MAX_TILES` tiles.
fn choose_window(
    fit: &RouteFit,
    output_scale: f64,
    out_w: u32,
    out_h: u32,
    zoom_offset: i32,
) -> TileWindow {
    let pixels_per_degree = fit.scale * output_scale;
    let matching = (pixels_per_degree * WORLD_DEGREES / TILE_SIZE as f64).log2().round() as i32;
    let (min_x, max_x, min_y, max_y) = visible_mercator(fit, output_scale, out_w, out_h);
    let mut zoom = (matching + zoom_offset).clamp(0, MAX_ZOOM) as u32;
    loop {
        let tiles_per_side = 2f64.powi(zoom as i32);
        let last = tiles_per_side - 1.0;
        let column = |mx: f64| ((mx / WORLD_DEGREES) * tiles_per_side).floor().clamp(0.0, last) as u32;
        let row = |my: f64| ((0.5 - my / WORLD_DEGREES) * tiles_per_side).floor().clamp(0.0, last) as u32;
        let (x0, x1) = (column(min_x), column(max_x));
        let (y0, y1) = (row(max_y), row(min_y));
        let window = TileWindow {
            zoom,
            x0,
            y0,
            columns: x1 - x0 + 1,
            rows: y1 - y0 + 1,
        };
        if window.columns * window.rows <= MAX_TILES || zoom == 0 {
            return window;
        }
        zoom -= 1;
    }
}

/// Downloads every tile of the window (a few at a time) and decodes it.
fn load_tiles(map: &ValidatedRouteMap, window: &TileWindow) -> CoreResult<Vec<(u32, u32, Image)>> {
    let coordinates = (0..window.rows)
        .flat_map(|row| (0..window.columns).map(move |column| (column, row)))
        .collect::<Vec<_>>();
    let chunk = coordinates.len().div_ceil(FETCH_WORKERS).max(1);
    let fetched = std::thread::scope(|scope| {
        coordinates
            .chunks(chunk)
            .map(|group| {
                scope.spawn(move || {
                    group
                        .iter()
                        .map(|&(column, row)| {
                            let url = expand_tile_url(&map.tile_url, window.zoom, window.x0 + column, window.y0 + row);
                            fetch_tile(&url).map(|bytes| (column, row, url, bytes))
                        })
                        .collect::<CoreResult<Vec<_>>>()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("map tile worker panicked"))
            .collect::<CoreResult<Vec<_>>>()
    })?;
    fetched
        .into_iter()
        .flatten()
        .map(|(column, row, url, bytes)| {
            Image::from_encoded(Data::new_copy(&bytes))
                .map(|image| (column, row, image))
                .ok_or_else(|| CoreError::Render(format!("map tile {url}: not a decodable image")))
        })
        .collect()
}

/// Renders the map in plot space (`out_w` x `out_h`, no rotation, no attribution).
///
/// `fit` is the route fit in unscaled widget space; `output_scale` is the scene scale that
/// turns widget pixels into output pixels.
pub(crate) fn build_map_image(
    map: &ValidatedRouteMap,
    fit: &RouteFit,
    output_scale: f64,
    out_w: u32,
    out_h: u32,
) -> CoreResult<Image> {
    let window = choose_window(fit, output_scale, out_w, out_h, map.zoom_offset);
    let tiles = load_tiles(map, &window)?;

    let mut mosaic = create_surface(window.columns * TILE_SIZE, window.rows * TILE_SIZE)?;
    for (column, row, tile) in &tiles {
        mosaic
            .canvas()
            .draw_image(tile, ((column * TILE_SIZE) as f32, (row * TILE_SIZE) as f32), None);
    }
    let mosaic = mosaic.image_snapshot();

    // Mosaic pixel -> output pixel. Mosaic x grows east and y grows south, like widget space.
    let degrees_per_pixel = WORLD_DEGREES / (TILE_SIZE as f64 * 2f64.powi(window.zoom as i32));
    let scale = degrees_per_pixel * fit.scale * output_scale;
    let origin_x = (window.x0 * TILE_SIZE) as f64;
    let origin_y = (window.y0 * TILE_SIZE) as f64;
    let translate_x = (origin_x * degrees_per_pixel - fit.min_x) * fit.scale * output_scale + fit.offset_x * output_scale;
    let translate_y = output_scale * (fit.height - fit.offset_y)
        - output_scale * fit.scale * (WORLD_DEGREES / 2.0 - origin_y * degrees_per_pixel - fit.min_y);

    let mut surface = create_surface(out_w, out_h)?;
    surface.canvas().clear(Color::TRANSPARENT);
    surface.canvas().concat(&Matrix::new_all(
        scale as f32,
        0.0,
        translate_x as f32,
        0.0,
        scale as f32,
        translate_y as f32,
        0.0,
        0.0,
        1.0,
    ));
    let mut paint = Paint::default();
    paint.set_alpha_f(map.opacity);
    surface.canvas().draw_image_with_sampling_options(
        &mosaic,
        (0.0, 0.0),
        SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear),
        Some(&paint),
    );
    Ok(surface.image_snapshot())
}

/// Wraps the plot-space map into a static layer rotated with the widget, with the credit
/// drawn in the bottom-right corner of the unrotated widget box.
pub(crate) fn build_map_layer(
    plot: &NormalizedRoutePlot,
    image: &Image,
    attribution: &str,
    output_scale: f64,
    font_dirs: &[PathBuf],
) -> CoreResult<StaticLayer> {
    let (min_x, min_y, layer_width, layer_height) = rotated_layer_bounds(plot, 0.0);
    let mut surface = create_surface(layer_width, layer_height)?;
    surface.canvas().clear(Color::TRANSPARENT);
    surface.canvas().save();
    surface.canvas().translate((-min_x, -min_y));
    surface.canvas().rotate(plot.rotation, None);
    surface.canvas().draw_image(image, (0.0, 0.0), None);
    draw_attribution(surface.canvas(), plot, attribution, output_scale as f32, font_dirs)?;
    surface.canvas().restore();
    Ok(StaticLayer {
        image: surface.image_snapshot(),
        x: min_x,
        y: min_y,
    })
}

fn draw_attribution(
    canvas: &skia_safe::Canvas,
    plot: &NormalizedRoutePlot,
    attribution: &str,
    output_scale: f32,
    font_dirs: &[PathBuf],
) -> CoreResult<()> {
    let font: Font = crate::render::text::resolve_font(font_dirs, None, ATTRIBUTION_FONT_PX * output_scale)?;
    let padding = ATTRIBUTION_PADDING_PX * output_scale;
    let (text_width, _) = font.measure_str(attribution, None);
    let (_, metrics) = font.metrics();
    let box_width = (text_width + padding * 2.0).min(plot.width as f32);
    let box_height = (metrics.descent - metrics.ascent + padding * 2.0).min(plot.height as f32);
    let left = plot.width as f32 - box_width;
    let top = plot.height as f32 - box_height;

    let mut background = Paint::default();
    background.set_anti_alias(true);
    background.set_color(Color::from_argb(190, 255, 255, 255));
    canvas.draw_rect(Rect::from_xywh(left, top, box_width, box_height), &background);

    let mut text = Paint::default();
    text.set_anti_alias(true);
    text.set_color(Color::from_rgb(51, 51, 51));
    canvas.draw_str(attribution, (left + padding, top + padding - metrics.ascent), &font, &text);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mercator_matches_known_points() {
        let (x, y) = mercator_degrees(0.0, 0.0);
        assert!((x - 180.0).abs() < 1e-9 && y.abs() < 1e-9);
        // Latitude 85.0511 is the top of the square world: 180 degrees.
        let (_, top) = mercator_degrees(MAX_MERCATOR_LATITUDE, 0.0);
        assert!((top - 180.0).abs() < 1e-3, "got {top}");
        let (_, below) = mercator_degrees(-30.0, 0.0);
        let (_, above) = mercator_degrees(30.0, 0.0);
        assert!((below + above).abs() < 1e-9);
    }

    fn fit_for(span_degrees: f64, width: f64, height: f64) -> RouteFit {
        // A route centered on (lon 10, lat 0) spanning `span_degrees`.
        let (cx, cy) = mercator_degrees(0.0, 10.0);
        let half = span_degrees / 2.0;
        RouteFit::new(&[(cx - half, cy - half), (cx + half, cy + half)], width, height, 0.0).unwrap()
    }

    #[test]
    fn zoom_follows_the_route_extent() {
        // 600 px for 0.1 degrees -> 6000 px/degree -> 6000*360/256 = 8437 px/turn -> zoom 13.
        let window = choose_window(&fit_for(0.1, 600.0, 600.0), 1.0, 600, 600, 0);
        assert_eq!(window.zoom, 13);
        let sharper = choose_window(&fit_for(0.1, 600.0, 600.0), 1.0, 600, 600, 1);
        assert_eq!(sharper.zoom, 14);
        let wider = choose_window(&fit_for(2.0, 600.0, 600.0), 1.0, 600, 600, 0);
        assert!(wider.zoom < window.zoom);
    }

    #[test]
    fn the_window_never_exceeds_the_tile_budget() {
        let window = choose_window(&fit_for(0.02, 1920.0, 1080.0), 1.0, 1920, 1080, 2);
        assert!(window.columns * window.rows <= MAX_TILES);
    }

    #[test]
    fn the_window_covers_the_center_tile() {
        let window = choose_window(&fit_for(0.1, 600.0, 600.0), 1.0, 600, 600, 0);
        let tiles = 2f64.powi(window.zoom as i32);
        let center_x = ((10.0 + 180.0) / 360.0 * tiles) as u32;
        let center_y = (0.5 * tiles) as u32;
        assert!((window.x0..window.x0 + window.columns).contains(&center_x));
        assert!((window.y0..window.y0 + window.rows).contains(&center_y));
    }
}
