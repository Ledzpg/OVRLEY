//! Static track layers and dynamic fill rendering for arc gauges.
//!
//! Static rendering caches empty tracks, borders, shadows, labels, and units.
//! Dynamic rendering uses the same canonical track geometry for continuous
//! reveals and rounded segmented wedges, so fill cannot change the silhouette.

use super::draw_arc_labels;
use super::geometry::ArcGaugeGeometry;
use super::inner_widget::draw_static_unit;
use super::path::ArcTrackSpec;
use super::segment::{draw_segment, inner_segment_geometry, segment_geometries};
use crate::error::CoreResult;
use crate::normalize::{ValidatedArcGaugeWidget, ValidatedSceneConfig};
use crate::render::text::{parse_color, ResolvedTextStyle};
use crate::render::widgets::common::normalize_shadow_style_validated;
use crate::render::widgets::types::{ArcGaugeCache, ResolvedRedline};
use super::super::metric::{bar_is_redline, stop_color_at};
use crate::types::TrackFillStyle;
use skia_safe::{image_filters, paint::Style, BlendMode, Canvas, ClipOp, Paint};
use std::path::PathBuf;

/// Builds the padded, cached image containing every frame-invariant element.
pub(super) fn draw_static_layer(
    canvas: &Canvas,
    gauge: &ValidatedArcGaugeWidget,
    scene: &ValidatedSceneConfig,
    geometry: ArcGaugeGeometry,
    scale: f32,
    font_dirs: &[PathBuf],
    min_value: f64,
    max_value: f64,
    unit_text: Option<&str>,
    text_style: &ResolvedTextStyle,
    altitude_offset_m: f64,
    redline: Option<&ResolvedRedline>,
) -> CoreResult<()> {
    let track_thickness = gauge.track_thickness * scale;
    let border_thickness = gauge.track_border_thickness * scale;
    let shadow = if border_thickness > 0.0 {
        normalize_shadow_style_validated(
            &scene.presentation.shadow_color,
            scene.presentation.shadow_strength_for(&gauge.inner_value.shadow),
            scene.presentation.shadow_distance_for(&gauge.inner_value.shadow),
            scale,
        )
    } else {
        None
    };
    let shadow_filter = shadow.and_then(|shadow| {
        image_filters::drop_shadow_only(
            (shadow.offset_x, shadow.offset_y),
            (shadow.strength, shadow.strength),
            parse_color(&shadow.color, text_style.opacity),
            None,
            None,
            None,
        )
    });

    match crate::normalize::scale_bar_geometry(gauge.bar_geometry, scale) {
        Some(bar_geometry) => {
            for (index, segment) in segment_geometries(geometry, bar_geometry)
                .into_iter()
                .enumerate()
            {
                let empty_color = match redline {
                    Some(redline)
                        if bar_is_redline(index as u32, bar_geometry.count, redline.fraction) =>
                    {
                        redline.color.as_str()
                    }
                    _ => gauge.track_empty_color.as_str(),
                };
                draw_static_track(
                    canvas,
                    gauge,
                    segment,
                    track_thickness,
                    border_thickness,
                    scale,
                    text_style,
                    shadow_filter.as_ref(),
                    empty_color,
                    None,
                );
            }
        }
        None => draw_static_track(
            canvas,
            gauge,
            geometry,
            track_thickness,
            border_thickness,
            scale,
            text_style,
            shadow_filter.as_ref(),
            gauge.track_empty_color.as_str(),
            redline,
        ),
    }

    if gauge.show_min_max_labels {
        draw_arc_labels(
            canvas,
            gauge,
            geometry,
            scale,
            font_dirs,
            min_value,
            max_value,
            text_style,
            altitude_offset_m,
        )?;
    }
    if gauge.ticks.is_some() {
        super::ticks::draw_ticks(
            canvas,
            gauge,
            geometry,
            value_track_spec(gauge, geometry, scale),
            scale,
            font_dirs,
            (min_value, max_value),
            redline,
            text_style,
        )?;
    }
    if let Some(unit_text) = unit_text {
        draw_static_unit(
            canvas, gauge, geometry, scale, font_dirs, unit_text, text_style,
        )?;
    }
    Ok(())
}

/// The filled track as the value reveals it (inside the border, with the fill's end cap). Ticks and
/// the needle map values to angles through it, so they land exactly where a fill of that value ends.
pub(super) fn value_track_spec(
    gauge: &ValidatedArcGaugeWidget,
    geometry: ArcGaugeGeometry,
    scale: f32,
) -> ArcTrackSpec {
    let track_thickness = gauge.track_thickness * scale;
    let border_thickness = gauge.track_border_thickness * scale;
    let (spec_geometry, spec_thickness, spec_corner) = if border_thickness > 0.0 {
        (
            inner_segment_geometry(geometry, border_thickness),
            (track_thickness - border_thickness * 2.0).max(0.0),
            (gauge.track_corner_radius * scale - border_thickness).max(0.0),
        )
    } else {
        (geometry, track_thickness, gauge.track_corner_radius * scale)
    };
    let end_corner = if gauge.track_fill_flat { 0.0 } else { spec_corner };
    ArcTrackSpec::full(spec_geometry, spec_thickness, spec_corner).with_end_corner_radius(end_corner)
}

/// Screen angle of a fill fraction: cap-aware for continuous fills, plain sweep for bars.
pub(super) fn value_angle(
    gauge: &ValidatedArcGaugeWidget,
    geometry: ArcGaugeGeometry,
    scale: f32,
    fraction: f32,
) -> f32 {
    match gauge.track_fill_style {
        TrackFillStyle::Fill => value_track_spec(gauge, geometry, scale).angle_at(fraction),
        TrackFillStyle::Bars => geometry.start_angle + geometry.sweep_angle * fraction,
    }
}

fn draw_static_track(
    canvas: &Canvas,
    gauge: &ValidatedArcGaugeWidget,
    geometry: ArcGaugeGeometry,
    track_thickness: f32,
    border_thickness: f32,
    scale: f32,
    text_style: &ResolvedTextStyle,
    shadow_filter: Option<&skia_safe::ImageFilter>,
    empty_color: &str,
    zone_redline: Option<&ResolvedRedline>,
) {
    let inner_corner_radius = gauge.track_corner_radius * scale;
    let border_color = parse_color(&gauge.track_border_color, text_style.opacity);

    if let Some(shadow_filter) = shadow_filter {
        draw_segment(
            canvas,
            geometry,
            track_thickness,
            inner_corner_radius,
            &segment_border_paint(border_color, Some(shadow_filter.clone())),
        );
    }
    if border_thickness > 0.0 {
        draw_segment(
            canvas,
            geometry,
            track_thickness,
            inner_corner_radius,
            &segment_border_paint(border_color, None),
        );
        let inner_geo = inner_segment_geometry(geometry, border_thickness);
        let inner_thickness = (track_thickness - border_thickness * 2.0).max(0.0);
        let inner_cnr = (inner_corner_radius - border_thickness).max(0.0);
        draw_segment(
            canvas,
            inner_geo,
            inner_thickness,
            inner_cnr,
            &clear_track_paint(),
        );
        let end_corner = if gauge.track_fill_flat { 0.0 } else { inner_cnr };
        let spec = ArcTrackSpec::full(inner_geo, inner_thickness, inner_cnr)
            .with_end_corner_radius(end_corner);
        paint_empty_track(
            canvas,
            inner_geo,
            inner_thickness,
            inner_cnr,
            gauge.track_empty_opacity * text_style.opacity,
            empty_color,
            zone_redline.map(|redline| (spec, redline)),
        );
    } else {
        let end_corner = if gauge.track_fill_flat {
            0.0
        } else {
            inner_corner_radius
        };
        let spec = ArcTrackSpec::full(geometry, track_thickness, inner_corner_radius)
            .with_end_corner_radius(end_corner);
        paint_empty_track(
            canvas,
            geometry,
            track_thickness,
            inner_corner_radius,
            gauge.track_empty_opacity * text_style.opacity,
            empty_color,
            zone_redline.map(|redline| (spec, redline)),
        );
    }
}

/// Paints the empty track, tinting the redline zone when one is configured.
fn paint_empty_track(
    canvas: &Canvas,
    geometry: ArcGaugeGeometry,
    thickness: f32,
    corner_radius: f32,
    opacity: f32,
    empty_color: &str,
    redline: Option<(ArcTrackSpec, &ResolvedRedline)>,
) {
    let Some((spec, redline)) = redline else {
        draw_segment(
            canvas,
            geometry,
            thickness,
            corner_radius,
            &track_paint(parse_color(empty_color, opacity), None),
        );
        return;
    };
    let (normal_zone, red_zone) = spec.redline_zones(redline.fraction);
    for (zone, color) in [(&normal_zone, empty_color), (&red_zone, redline.color.as_str())] {
        canvas.save();
        canvas.clip_path(zone, ClipOp::Intersect, true);
        draw_segment(
            canvas,
            geometry,
            thickness,
            corner_radius,
            &track_paint(parse_color(color, opacity), None),
        );
        canvas.restore();
    }
}

/// Dispatches the current fill to the continuous or segmented painting model.
///
/// `overrun` paints the track completely in the redline color (the value reached the redline).
pub(super) fn draw_fill(
    canvas: &Canvas,
    cache: &ArcGaugeCache,
    geometry: ArcGaugeGeometry,
    fill01: f32,
    overrun: bool,
) {
    match cache.track_fill_style {
        TrackFillStyle::Bars => draw_segmented_fill(canvas, cache, geometry, fill01, overrun),
        TrackFillStyle::Fill => draw_continuous_fill(canvas, cache, geometry, fill01, overrun),
    }
}

fn draw_segmented_fill(
    canvas: &Canvas,
    cache: &ArcGaugeCache,
    geometry: ArcGaugeGeometry,
    fill01: f32,
    overrun: bool,
) {
    let bars = cache
        .bar_geometry
        .expect("bars fill style must carry resolved bar geometry");
    let filled_count = super::super::metric::bar_fill_count(fill01, bars.count);
    let overrun_paint = overrun
        .then(|| {
            cache
                .redline
                .as_ref()
                .map(|redline| fill_paint_with_color(cache, &redline.color))
        })
        .flatten();
    let paint = fill_paint(cache);
    let redline_paint = cache
        .redline
        .as_ref()
        .map(|redline| fill_paint_with_color(cache, &redline.color));
    let inner_thickness = (cache.track_thickness - cache.track_border_thickness * 2.0).max(0.0);
    let inner_corner = (cache.track_corner_radius - cache.track_border_thickness).max(0.0);
    for (index, segment) in segment_geometries(geometry, bars)
        .into_iter()
        .enumerate()
        .take(filled_count)
    {
        let inner_geo = inner_segment_geometry(segment, cache.track_border_thickness);
        let zone_paint;
        let paint = match (&cache.redline, &redline_paint) {
            _ if overrun_paint.is_some() => overrun_paint.as_ref().expect("checked above"),
            (Some(redline), Some(redline_paint))
                if bar_is_redline(index as u32, bars.count, redline.fraction) =>
            {
                redline_paint
            }
            _ => match &cache.fill_stops {
                Some(stops) => {
                    let color = stop_color_at(stops, (index as f32 + 0.5) / bars.count as f32);
                    zone_paint = track_paint(color, None);
                    &zone_paint
                }
                None => &paint,
            },
        };
        draw_segment(canvas, inner_geo, inner_thickness, inner_corner, paint);
    }
}

fn draw_continuous_fill(
    canvas: &Canvas,
    cache: &ArcGaugeCache,
    geometry: ArcGaugeGeometry,
    fill01: f32,
    overrun: bool,
) {
    let bt = cache.track_border_thickness;
    let inner_thickness = (cache.track_thickness - bt * 2.0).max(0.0);
    let inner_geometry = inner_segment_geometry(geometry, bt);
    let inner_corner = (cache.track_corner_radius - bt).max(0.0);
    let end_corner = if cache.track_fill_flat {
        0.0
    } else {
        inner_corner
    };
    let reveal_spec = ArcTrackSpec::full(inner_geometry, inner_thickness, inner_corner)
        .with_end_corner_radius(end_corner);
    let Some(reveal) = reveal_spec.reveal_clip(fill01) else {
        return;
    };
    let Some(reveal_path) = reveal.path(&reveal_spec) else {
        return;
    };

    if let Some(redline) = cache.redline.as_ref().filter(|_| overrun) {
        canvas.save();
        canvas.clip_path(&reveal_path, ClipOp::Intersect, true);
        draw_segment(
            canvas,
            inner_geometry,
            inner_thickness,
            inner_corner,
            &fill_paint_with_color(cache, &redline.color),
        );
        canvas.restore();
        return;
    }
    let normal_paint = zone_fill_paint(cache, &reveal_spec);
    let split = cache
        .redline
        .as_ref()
        .filter(|redline| fill01 > redline.fraction);
    match split {
        None => {
            canvas.save();
            canvas.clip_path(&reveal_path, ClipOp::Intersect, true);
            draw_segment(
                canvas,
                inner_geometry,
                inner_thickness,
                inner_corner,
                &normal_paint,
            );
            canvas.restore();
        }
        Some(redline) => {
            let (normal_zone, red_zone) = reveal_spec.redline_zones(redline.fraction);
            let zones = [
                (&normal_zone, normal_paint),
                (&red_zone, fill_paint_with_color(cache, &redline.color)),
            ];
            for (zone, paint) in zones {
                canvas.save();
                canvas.clip_path(&reveal_path, ClipOp::Intersect, true);
                canvas.clip_path(zone, ClipOp::Intersect, true);
                draw_segment(canvas, inner_geometry, inner_thickness, inner_corner, &paint);
                canvas.restore();
            }
        }
    }
}

/// Paint for the normal (below the redline) fill: a sweep gradient along the track when the
/// gauge has color bands, otherwise the single configured fill color.
fn zone_fill_paint(cache: &ArcGaugeCache, spec: &ArcTrackSpec) -> Paint {
    let Some(stops) = cache.fill_stops.as_ref() else {
        return fill_paint(cache);
    };
    let (start, end) = (spec.angle_at(0.0), spec.angle_at(1.0));
    // A sweep gradient needs an increasing angle range; mirror the stops for counter-clockwise tracks.
    let reversed = end < start;
    let ordered: Vec<(f32, skia_safe::Color)> = if reversed {
        stops
            .iter()
            .rev()
            .map(|stop| (1.0 - stop.fraction, stop.color))
            .collect()
    } else {
        stops.iter().map(|stop| (stop.fraction, stop.color)).collect()
    };
    let colors: Vec<skia_safe::Color4f> = ordered
        .iter()
        .map(|(_, color)| skia_safe::Color4f::from(*color))
        .collect();
    let positions: Vec<f32> = ordered.iter().map(|(position, _)| *position).collect();
    let angles = if reversed { (end, start) } else { (start, end) };
    let center = (spec.geometry.center_x, spec.geometry.center_y);
    let gradient = skia_safe::gradient::Gradient::new(
        skia_safe::gradient::Colors::new(
            &colors,
            Some(positions.as_slice()),
            skia_safe::TileMode::Clamp,
            None,
        ),
        skia_safe::gradient::Interpolation::default(),
    );
    let shader = skia_safe::gradient::shaders::sweep_gradient(center, angles, &gradient, None);
    let mut paint = track_paint(skia_safe::Color::WHITE, None);
    match shader {
        Some(shader) => {
            paint.set_shader(shader);
            paint
        }
        None => fill_paint(cache),
    }
}

fn fill_paint(cache: &ArcGaugeCache) -> Paint {
    fill_paint_with_color(cache, &cache.track_filled_color)
}

fn fill_paint_with_color(cache: &ArcGaugeCache, color: &str) -> Paint {
    track_paint(
        parse_color(color, cache.track_filled_opacity * cache.text_style.opacity),
        None,
    )
}

fn track_paint(color: skia_safe::Color, image_filter: Option<skia_safe::ImageFilter>) -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(Style::Fill);
    paint.set_color(color);
    if let Some(image_filter) = image_filter {
        paint.set_image_filter(image_filter);
    }
    paint
}

fn segment_border_paint(
    color: skia_safe::Color,
    image_filter: Option<skia_safe::ImageFilter>,
) -> Paint {
    let mut paint = track_paint(color, image_filter);
    paint.set_style(Style::Fill);
    paint
}

fn clear_track_paint() -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(Style::Fill);
    paint.set_blend_mode(BlendMode::Clear);
    paint
}
