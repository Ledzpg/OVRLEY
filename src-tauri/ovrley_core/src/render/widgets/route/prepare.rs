/// Route widget preparation: sample building, geometry fitting, and static
/// layer construction.
///
/// Builds all expensive geometry and frame-position data once before the render
/// loop. Per-frame drawing then only composites cached/static pieces.
use super::super::common::{
    custom_export_range_active, normalize_optional_progress_window, static_layer_padding,
};
use super::super::marker::marker_layers_from_plot;
use super::super::polyline::draw_polyline_with_shadow;
use super::super::types::{
    NormalizedRoutePlot, RouteSample, RouteWidgetCache, StaticLayer, WidgetGeometry,
};
use super::curve::curve_route_samples;
use super::fit::RouteFit;
use super::map::{build_map_image, build_map_layer, mercator_degrees};
use super::simplify::{downsample_route_samples, simplify_route_samples};
use crate::activity::schema::{DenseActivityReport, ParsedActivity};
use crate::activity::trim::trim_activity;
use crate::debug::RenderProfiler;
use crate::error::{CoreError, CoreResult};
use crate::normalize::RenderDataRequirements;
use crate::normalize::ValidatedRoutePlot;
use crate::render::surface::create_surface;
use std::path::PathBuf;
use std::time::Instant;

const ROUTE_CURVE_FLATNESS_PX: f64 = 0.35;

/// Prepares cached geometry, static layers, and frame states for a route plot.
pub(crate) fn prepare_route_cache(
    activity: &ParsedActivity,
    dense_activity: &DenseActivityReport,
    validated: &ValidatedRoutePlot,
    scene: &crate::normalize::ValidatedSceneConfig,
    font_dirs: &[PathBuf],
    prepare_profiler: &mut RenderProfiler,
) -> CoreResult<RouteWidgetCache> {
    let prepare_started = Instant::now();
    let show_full_activity = validated.show_full_activity;
    let plot = super::normalize::normalize_route_plot(validated, scene);
    let route_samples =
        build_route_samples(activity, show_full_activity, validated.map.is_some(), scene)?;
    let geometry = prepare_profiler.measure("build_route_cache.geometry", || {
        build_route_geometry(&plot, validated, &route_samples)
    })?;
    let marker_layers = marker_layers_from_plot(
        &plot.marker_variant,
        plot.marker_variant_diameter,
        plot.marker_size,
        &plot.marker_color,
        plot.marker_opacity,
    );
    let remaining_layer = prepare_profiler.measure("build_route_cache.layers", || {
        build_route_remaining_layer(&plot, &geometry)
    })?;
    let map_layer = match &validated.map {
        Some(map) => Some(prepare_profiler.measure("build_route_cache.map", || {
            let (fit, output_scale) = route_map_fit(&plot, validated, &route_samples)?;
            let image = build_map_image(map, &fit, output_scale, plot.width, plot.height)?;
            build_map_layer(&plot, &image, &map.attribution, output_scale, font_dirs)
        })?),
        None => None,
    };
    let frame_states = prepare_profiler.measure("build_route_cache.frame_states", || {
        super::frame_state::build_route_frame_states(
            activity,
            &geometry,
            dense_activity,
            show_full_activity,
            scene,
        )
    });
    prepare_profiler.record_ms(
        "build_route_cache",
        prepare_started.elapsed().as_secs_f64() * 1000.0,
    );

    Ok(RouteWidgetCache {
        plot,
        geometry,
        frame_states,
        marker_layers,
        remaining_layer,
        map_layer,
    })
}

/// Route fit in unscaled widget space plus the factor that turns widget pixels into
/// output pixels; the raster map is drawn through exactly this transform.
pub(crate) fn route_map_fit(
    plot: &NormalizedRoutePlot,
    validated: &ValidatedRoutePlot,
    route_samples: &[RouteSample],
) -> CoreResult<(RouteFit, f64)> {
    let projected = route_samples
        .iter()
        .map(|sample| sample.point)
        .collect::<Vec<_>>();
    let fit = RouteFit::new(
        &projected,
        validated.width as f64,
        validated.height as f64,
        route_geometry_inset_px_validated(validated) as f64,
    )
    .ok_or_else(|| CoreError::Render("Route map requires course points".into()))?;
    Ok((fit, plot.width as f64 / validated.width.max(1) as f64))
}

/// Builds widget-space geometry for the route path.
pub(crate) fn build_route_geometry(
    plot: &NormalizedRoutePlot,
    validated: &ValidatedRoutePlot,
    route_samples: &[RouteSample],
) -> CoreResult<WidgetGeometry> {
    if route_samples.len() < 2 {
        return Err(CoreError::Render(
            "Route plot requires at least two valid course points".into(),
        ));
    }
    let projected = route_samples
        .iter()
        .map(|sample| sample.point)
        .collect::<Vec<_>>();
    // Match the editor preview: simplify in unscaled widget space, then map the
    // surviving points into the scaled render box for Skia drawing.
    let stroke_inset = route_geometry_inset_px_validated(validated);
    let fit = RouteFit::new(
        &projected,
        validated.width as f64,
        validated.height as f64,
        stroke_inset as f64,
    )
    .ok_or_else(|| {
        CoreError::Render("Route plot requires at least two valid course points".into())
    })?;
    let fitted = projected
        .iter()
        .map(|point| fit.apply(*point))
        .collect::<Vec<_>>();
    let fitted_samples = route_samples
        .iter()
        .zip(fitted.iter())
        .map(|(sample, point)| RouteSample {
            point: *point,
            progress01: sample.progress01,
        })
        .collect::<Vec<_>>();
    let target_count = ((validated.width as f32) * plot.target_density)
        .round()
        .max(2.0) as usize;
    let downsampled =
        downsample_route_samples(&fitted_samples, target_count.min(fitted_samples.len()));
    let geometry_samples =
        simplify_route_samples(&downsampled, plot.simplify_tolerance_px.max(0.05));
    let curved_samples = curve_route_samples(&geometry_samples, ROUTE_CURVE_FLATNESS_PX);
    let scaled_points = curved_samples
        .iter()
        .map(|sample| {
            (
                (sample.point.0 * (plot.width as f64 / validated.width.max(1) as f64)) as f32,
                (sample.point.1 * (plot.height as f64 / validated.height.max(1) as f64)) as f32,
            )
        })
        .collect::<Vec<_>>();

    Ok(WidgetGeometry {
        bbox: (0.0, 0.0, plot.width as f32, plot.height as f32),
        progress_values: curved_samples
            .iter()
            .map(|sample| sample.progress01)
            .collect(),
        elapsed_fractions: Vec::new(),
        elevation_data_range: None,
        points: scaled_points,
        source_point_count: route_samples.len(),
        simplification: format!(
            "lttb_density_{:.2}_rdp_px_{:.2}_catmull_rom_flatness_px_{ROUTE_CURVE_FLATNESS_PX:.2}",
            plot.target_density, plot.simplify_tolerance_px
        ),
    })
}

/// Builds the cached static layer for the not-yet-completed route.
fn build_route_remaining_layer(
    plot: &NormalizedRoutePlot,
    geometry: &WidgetGeometry,
) -> CoreResult<Option<StaticLayer>> {
    if geometry.points.len() < 2 {
        return Ok(None);
    }

    let padding = static_layer_padding(
        plot.remaining_line_width,
        plot.remaining_line_shadow.as_ref(),
    );
    let (min_x, min_y, layer_width, layer_height) = rotated_layer_bounds(plot, padding as f32);
    let mut surface = create_surface(layer_width, layer_height)?;
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    surface.canvas().save();
    surface.canvas().translate((-min_x, -min_y));
    surface.canvas().rotate(plot.rotation, None);
    draw_polyline_with_shadow(
        surface.canvas(),
        &geometry.points,
        &plot.remaining_line_color,
        plot.remaining_line_width,
        plot.remaining_line_opacity,
        plot.remaining_line_shadow.as_ref(),
    );
    surface.canvas().restore();

    Ok(Some(StaticLayer {
        image: surface.image_snapshot(),
        x: min_x,
        y: min_y,
    }))
}

/// Bounds of a widget-sized layer (grown by `padding`) after rotating it with the widget:
/// returns the top-left corner and the pixel size of the rotated bounding box.
pub(super) fn rotated_layer_bounds(plot: &NormalizedRoutePlot, padding: f32) -> (f32, f32, u32, u32) {
    let radians = plot.rotation.to_radians();
    let (sin, cos) = radians.sin_cos();
    let corners = [
        (-padding, -padding),
        (plot.width as f32 + padding, -padding),
        (-padding, plot.height as f32 + padding),
        (plot.width as f32 + padding, plot.height as f32 + padding),
    ];
    let xs = corners.map(|(x, y)| x * cos - y * sin);
    let ys = corners.map(|(x, y)| x * sin + y * cos);
    let min_x = xs.iter().copied().fold(f32::INFINITY, f32::min).floor();
    let max_x = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max).ceil();
    let min_y = ys.iter().copied().fold(f32::INFINITY, f32::min).floor();
    let max_y = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max).ceil();
    (
        min_x,
        min_y,
        (max_x - min_x).max(1.0) as u32,
        (max_y - min_y).max(1.0) as u32,
    )
}

fn route_geometry_inset_px_validated(plot: &ValidatedRoutePlot) -> f32 {
    let line_inset = (plot.remaining_line_width.max(plot.completed_line_width) * 0.5).max(0.0);
    let marker_inset = plot.marker_size.max(plot.marker_variant_diameter * 0.5);
    let base_inset = marker_inset.max(line_inset) + 1.0;
    base_inset.min(plot.width.min(plot.height) as f32 * 0.45)
}

/// Selects full-activity or trimmed course samples for route geometry.
///
/// `show_full_activity` overrides custom export trimming so the full route
/// remains visible while progress can still follow the selected scene.
pub(crate) fn build_route_samples(
    activity: &ParsedActivity,
    show_full_activity: bool,
    mercator: bool,
    scene: &crate::normalize::ValidatedSceneConfig,
) -> CoreResult<Vec<RouteSample>> {
    if show_full_activity || !custom_export_range_active(scene) {
        return Ok(project_course_samples(
            &activity.sample_course_points,
            &activity.sample_distance_progress,
            mercator,
        ));
    }

    let trimmed = trim_activity(
        activity,
        scene.start,
        scene.end,
        &RenderDataRequirements {
            distance_progress: true,
            course: true,
            ..RenderDataRequirements::default()
        },
    )?;

    Ok(project_course_samples_with_optional_progress(
        &trimmed.course,
        &trimmed.sample_distance_progress,
        mercator,
    ))
}

/// Projects latitude/longitude samples into local 2D route samples.
///
/// Uses an equirectangular approximation centered on mean latitude, which is
/// sufficient for small activity routes and avoids a heavy projection crate. With a
/// raster map behind the route, Web Mercator is used instead so the route matches the tiles.
fn project_course_samples(
    course_points: &[(Option<f64>, Option<f64>)],
    progress_values: &[f64],
    mercator: bool,
) -> Vec<RouteSample> {
    let valid_points = course_points
        .iter()
        .enumerate()
        .filter_map(|(index, (lat, lon))| match (*lat, *lon) {
            (Some(lat), Some(lon)) if lat.is_finite() && lon.is_finite() => Some((index, lat, lon)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if valid_points.is_empty() {
        return Vec::new();
    }
    let mean_latitude =
        valid_points.iter().map(|(_, lat, _)| lat).sum::<f64>() / valid_points.len() as f64;
    let mean_latitude_radians = mean_latitude.to_radians();
    valid_points
        .into_iter()
        .map(|(index, lat, lon)| RouteSample {
            point: if mercator {
                mercator_degrees(lat, lon)
            } else {
                (lon * mean_latitude_radians.cos(), lat)
            },
            progress01: progress_values
                .get(index)
                .copied()
                .filter(|value| value.is_finite())
                .unwrap_or_else(|| {
                    index as f64 / course_points.len().saturating_sub(1).max(1) as f64
                })
                .clamp(0.0, 1.0) as f32,
        })
        .collect()
}

/// Projects trimmed course samples after normalizing optional progress values.
fn project_course_samples_with_optional_progress(
    course_points: &[(Option<f64>, Option<f64>)],
    progress_values: &[Option<f64>],
    mercator: bool,
) -> Vec<RouteSample> {
    let normalized_progress =
        normalize_optional_progress_window(progress_values).unwrap_or_else(|| {
            (0..course_points.len())
                .map(|index| {
                    if course_points.len() > 1 {
                        index as f64 / (course_points.len() - 1) as f64
                    } else {
                        0.0
                    }
                })
                .collect::<Vec<_>>()
        });

    project_course_samples(course_points, &normalized_progress, mercator)
}
