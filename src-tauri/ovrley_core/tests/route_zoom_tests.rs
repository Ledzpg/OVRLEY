#![recursion_limit = "512"]
//! Close-up route map: validation, position, zoom, rotation, arrow and the video render path.
//!
//! The tiles come from a generated `file://` tile pack: every tile is light gray except for a red
//! square centered on the route's first point, so where that square lands shows exactly where the
//! map is looking and how it is turned. The marker is green and the route blue.

mod common;

use ovrley_core::activity::parse_activity_json;
use ovrley_core::commands::route_zoom::build_route_zoom_frame_png_command;
use ovrley_core::normalize::raw::RenderConfig;
use ovrley_core::normalize::{validate_render_config, ValidatedRenderConfig};
use ovrley_core::paths::AppPaths;
use ovrley_core::render::{render_preview_with_report, widgets::prepare_render_assets};
use std::collections::BTreeMap;
use std::path::PathBuf;

const START: (f64, f64) = (47.6062, -122.3321);
const SECOND: (f64, f64) = (47.6097, -122.3400);
const TARGET_HALF_PX: i64 = 10;
const ZOOM: f64 = 14.0;
/// Output pixels per Mercator degree at the test zoom (scene scale 1).
const PIXELS_PER_DEGREE: f64 = 256.0 * 16384.0 / 360.0;
const WIDGET: (f32, f32, u32) = (20.0, 30.0, 300);

fn mercator(point: (f64, f64)) -> (f64, f64) {
    let y = (std::f64::consts::FRAC_PI_4 + point.0.to_radians() / 2.0).tan().ln().to_degrees();
    (point.1 + 180.0, y)
}

fn course_points() -> serde_json::Value {
    serde_json::json!([[START.0, START.1], [SECOND.0, SECOND.1], [47.6130, -122.3350], [47.6101, -122.3290], [47.6075, -122.3305]])
}

fn activity_json() -> String {
    serde_json::json!({
        "sample_elapsed_seconds": [0.0, 1.0, 2.0, 3.0, 4.0],
        "sample_distance_progress": [0.0, 0.25, 0.5, 0.75, 1.0],
        "sample_course_points": course_points(),
        "trim_start_seconds": 0.0,
        "trim_end_seconds": 4.0
    })
    .to_string()
}

fn scene() -> serde_json::Value {
    let mut scene = common::builders::scene_json();
    scene["width"] = serde_json::json!(400);
    scene["height"] = serde_json::json!(400);
    scene["start"] = serde_json::json!(0.0);
    scene["end"] = serde_json::json!(4.0);
    scene
}

fn plot(template: &str, overrides: serde_json::Value) -> serde_json::Value {
    let mut plot = serde_json::json!({
        "value": "course_zoom", "x": WIDGET.0, "y": WIDGET.1, "width": WIDGET.2, "height": WIDGET.2,
        "zoom": ZOOM, "corner_radius": 0, "rotate_with_heading": false, "show_full_activity": true,
        "map_tile_url": template, "map_attribution": "(c) Test tiles", "map_opacity": 1, "map_zoom_offset": 0,
        "completed_line_width": 3, "completed_line_color": "#0000ff", "completed_line_opacity": 1,
        "remaining_line_width": 3, "remaining_line_color": "#0000ff", "remaining_line_opacity": 1,
        "marker_size": 40, "marker_color": "#00ff00", "marker_opacity": 1,
        "marker_outline_color": "#ffffff", "marker_outline_width": 0
    });
    for (key, value) in overrides.as_object().unwrap() {
        plot[key] = value.clone();
    }
    plot
}

fn config_json(plot: serde_json::Value) -> String {
    serde_json::json!({ "scene": scene(), "values": [], "labels": [], "plots": [plot] }).to_string()
}

fn validated(plot: serde_json::Value) -> Result<ValidatedRenderConfig, ovrley_core::CoreError> {
    validate_render_config(RenderConfig {
        scene: serde_json::from_value(scene()).unwrap(),
        backdrops: vec![],
        rasters: vec![],
        labels: vec![],
        values: vec![],
        plots: serde_json::Value::Array(vec![plot]),
        extra: BTreeMap::new(),
    })
}

fn test_paths() -> AppPaths {
    let root = common::test_config::repo_git_root();
    AppPaths {
        repo_root: root.clone(),
        font_dirs: vec![root.join("fonts")],
        debug_render_dir: std::env::temp_dir(),
        temp_dir: std::env::temp_dir(),
        bundled_templates_dirs: vec![],
        user_templates_dir: std::env::temp_dir(),
        downloads_dir: std::env::temp_dir(),
    }
}

/// Writes tiles for zooms 14 and 15 around the start point; returns the `file://` template.
fn write_tile_pack(name: &str) -> (PathBuf, String) {
    let root = std::env::temp_dir().join(format!("ovrley-{name}-{}", std::process::id()));
    let (lat, lon) = START;
    let mercator_y = (std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan().ln().to_degrees();
    for zoom in [14u32, 15] {
        let tiles = 2f64.powi(zoom as i32);
        let world = 256.0 * tiles;
        let target_x = ((lon + 180.0) / 360.0 * world).round() as i64;
        let target_y = ((0.5 - mercator_y / 360.0) * world).round() as i64;
        let (center_x, center_y) = (target_x / 256, target_y / 256);
        for tile_x in center_x - 5..=center_x + 5 {
            for tile_y in center_y - 5..=center_y + 5 {
                let image = image::RgbaImage::from_fn(256, 256, |u, v| {
                    let (world_x, world_y) = (tile_x * 256 + u as i64, tile_y * 256 + v as i64);
                    if (world_x - target_x).abs() <= TARGET_HALF_PX && (world_y - target_y).abs() <= TARGET_HALF_PX {
                        image::Rgba([255, 0, 0, 255])
                    } else {
                        image::Rgba([221, 221, 221, 255])
                    }
                });
                let dir = root.join(zoom.to_string()).join(tile_x.to_string());
                std::fs::create_dir_all(&dir).unwrap();
                image.save(dir.join(format!("{tile_y}.png"))).unwrap();
            }
        }
    }
    let template = format!("file:///{}/{{z}}/{{x}}/{{y}}.png", root.to_string_lossy().replace('\\', "/").trim_start_matches('/'));
    (root, template)
}

fn preview(template: &str, overrides: serde_json::Value, progress: f32) -> image::RgbaImage {
    let png = build_route_zoom_frame_png_command(&test_paths(), &config_json(plot(template, overrides)), &activity_json(), progress).unwrap();
    image::load_from_memory(&png).unwrap().to_rgba8()
}

fn centroid(image: &image::RgbaImage, wanted: impl Fn(&image::Rgba<u8>) -> bool) -> (f64, f64, usize) {
    let (mut sum_x, mut sum_y, mut count) = (0.0, 0.0, 0usize);
    for (x, y, pixel) in image.enumerate_pixels() {
        if wanted(pixel) {
            sum_x += x as f64;
            sum_y += y as f64;
            count += 1;
        }
    }
    (sum_x / count.max(1) as f64, sum_y / count.max(1) as f64, count)
}

fn is_red(pixel: &image::Rgba<u8>) -> bool {
    pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60 && pixel[3] > 200
}

fn is_green(pixel: &image::Rgba<u8>) -> bool {
    pixel[1] > 200 && pixel[0] < 60 && pixel[2] < 60 && pixel[3] > 200
}

/// Midway between the first two route points: where the preview at progress 0.125 looks.
struct Midway {
    /// Screen offset (north-up, pixels) of the route start from the view center.
    start_offset: (f64, f64),
    /// Bearing of the first segment, degrees clockwise from north.
    heading: f64,
}

fn midway() -> Midway {
    let (a, b) = (mercator(START), mercator(SECOND));
    let middle = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    Midway {
        start_offset: ((a.0 - middle.0) * PIXELS_PER_DEGREE, -(a.1 - middle.1) * PIXELS_PER_DEGREE),
        heading: (b.0 - a.0).atan2(b.1 - a.1).to_degrees().rem_euclid(360.0),
    }
}

fn widget_center() -> (f64, f64) {
    (f64::from(WIDGET.0) + f64::from(WIDGET.2) / 2.0, f64::from(WIDGET.1) + f64::from(WIDGET.2) / 2.0)
}

#[test]
fn the_preview_is_widget_sized_and_centered_on_the_current_position() {
    let (root, template) = write_tile_pack("zoom-center");
    // At the very start the position is the red target: it sits under the center.
    let image = preview(&template, serde_json::json!({ "marker_size": 14 }), 0.0);
    assert_eq!((image.width(), image.height()), (300, 300));
    let (x, y, count) = centroid(&image, is_red);
    assert!(count > 100, "the target is visible, found {count} red pixels");
    assert!((x - 150.0).abs() < 3.0 && (y - 150.0).abs() < 3.0, "target at ({x:.1}, {y:.1}), expected the center");
    // The marker is drawn over the center and the map covers the whole widget.
    assert!(centroid(&image, is_green).2 > 10);
    let corner = image.get_pixel(3, 3);
    assert!(corner[3] > 200 && corner[0] > 150, "map covers the widget, got {corner:?}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_map_moves_under_the_marker_by_the_zoom_scale() {
    let (root, template) = write_tile_pack("zoom-move");
    let image = preview(&template, serde_json::json!({}), 0.125);
    let (x, y, count) = centroid(&image, is_red);
    assert!(count > 100, "the start target is still in view, found {count} red pixels");
    let offset = midway().start_offset;
    assert!(
        (x - (150.0 + offset.0)).abs() < 3.0 && (y - (150.0 + offset.1)).abs() < 3.0,
        "target at ({x:.1}, {y:.1}), expected ({:.1}, {:.1})",
        150.0 + offset.0,
        150.0 + offset.1
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_closer_zoom_moves_things_proportionally_and_detail_does_not() {
    let (root, template) = write_tile_pack("zoom-scale");
    let near = |image: &image::RgbaImage| {
        let (x, y, _) = centroid(image, is_red);
        (x - 150.0, y - 150.0)
    };
    let base = near(&preview(&template, serde_json::json!({}), 0.125));
    // Detail offset 1 downloads tiles one level deeper but shows the same ground.
    let detailed = near(&preview(&template, serde_json::json!({ "map_zoom_offset": 1 }), 0.125));
    assert!((base.0 - detailed.0).abs() < 3.0 && (base.1 - detailed.1).abs() < 3.0, "{base:?} vs {detailed:?}");
    // One zoom level less shows twice the ground: the target is half as far from the center.
    let (root_far, template_far) = write_tile_pack("zoom-scale-far");
    let wider = near(&preview(&template_far, serde_json::json!({ "zoom": 13.0, "map_zoom_offset": 1 }), 0.125));
    assert!((wider.0 - base.0 / 2.0).abs() < 3.0 && (wider.1 - base.1 / 2.0).abs() < 3.0, "{wider:?} vs half of {base:?}");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(root_far);
}

#[test]
fn the_arrow_points_along_the_direction_of_travel_on_a_north_up_map() {
    let (root, template) = write_tile_pack("zoom-arrow");
    let image = preview(&template, serde_json::json!({}), 0.125);
    assert!(centroid(&image, is_green).2 > 200);
    // The tip reaches farther from the center than the notched back does: measured along the
    // direction of travel, the arrow extends more ahead of the center than behind it.
    let heading = midway().heading.to_radians();
    let ahead = (heading.sin(), -heading.cos());
    let (mut max_along, mut min_along) = (f64::MIN, f64::MAX);
    for (x, y, pixel) in image.enumerate_pixels() {
        if is_green(pixel) {
            let along = (x as f64 - 150.0) * ahead.0 + (y as f64 - 150.0) * ahead.1;
            max_along = max_along.max(along);
            min_along = min_along.min(along);
        }
    }
    assert!(max_along + min_along > 2.0, "arrow reaches {max_along:.1} ahead and {:.1} behind, expected more ahead", -min_along);
    // Sideways it is symmetric.
    let side = (-ahead.1, ahead.0);
    let (mut left, mut right) = (f64::MIN, f64::MAX);
    for (x, y, pixel) in image.enumerate_pixels() {
        if is_green(pixel) {
            let across = (x as f64 - 150.0) * side.0 + (y as f64 - 150.0) * side.1;
            left = left.max(across);
            right = right.min(across);
        }
    }
    assert!((left + right).abs() < 2.5, "arrow is symmetric across its axis, got {left:.1} and {right:.1}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_heading_up_map_turns_under_a_marker_that_points_up() {
    let (root, template) = write_tile_pack("zoom-heading-up");
    let image = preview(&template, serde_json::json!({ "rotate_with_heading": true }), 0.125);
    // The arrow points up: its tip reaches higher above the center than its back reaches below.
    let rows = image.enumerate_pixels().filter(|(_, _, pixel)| is_green(pixel)).map(|(_, y, _)| y as f64);
    let (top, bottom) = rows.fold((f64::MAX, f64::MIN), |(top, bottom), y| (top.min(y), bottom.max(y)));
    assert!(150.0 - top > bottom - 150.0 + 2.0, "tip {:.1} above, back {:.1} below", 150.0 - top, bottom - 150.0);
    let (x, _, _) = centroid(&image, is_green);
    assert!((x - 150.0).abs() < 2.0, "arrow is centered horizontally, got {x:.1}");
    // The map is turned by the heading, counter-clockwise, so travel is up.
    let offset = midway().start_offset;
    let angle = (-midway().heading).to_radians();
    let expected = (offset.0 * angle.cos() - offset.1 * angle.sin(), offset.0 * angle.sin() + offset.1 * angle.cos());
    let (red_x, red_y, count) = centroid(&image, is_red);
    assert!(count > 100, "the target is in view, found {count} red pixels");
    assert!(
        (red_x - (150.0 + expected.0)).abs() < 4.0 && (red_y - (150.0 + expected.1)).abs() < 4.0,
        "target at ({red_x:.1}, {red_y:.1}), expected ({:.1}, {:.1})",
        150.0 + expected.0,
        150.0 + expected.1
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_route_is_drawn_in_two_styles_around_the_marker() {
    let (root, template) = write_tile_pack("zoom-lines");
    let overrides = serde_json::json!({
        "completed_line_color": "#ff00ff", "remaining_line_color": "#00ffff", "marker_size": 4, "marker_opacity": 0
    });
    let image = preview(&template, overrides, 0.125);
    let magenta = |pixel: &image::Rgba<u8>| pixel[0] > 200 && pixel[2] > 200 && pixel[1] < 60 && pixel[3] > 200;
    let cyan = |pixel: &image::Rgba<u8>| pixel[1] > 200 && pixel[2] > 200 && pixel[0] < 60 && pixel[3] > 200;
    let (finished_x, finished_y, finished) = centroid(&image, magenta);
    let (remaining_x, remaining_y, remaining) = centroid(&image, cyan);
    assert!(finished > 30 && remaining > 30, "both parts are visible: {finished} and {remaining} pixels");
    // Midway on the first segment: the finished part is behind (toward the start, bottom-right of
    // the center) and the rest is ahead (up-left).
    assert!(finished_x > 150.0 && finished_y > 150.0, "finished part at ({finished_x:.1}, {finished_y:.1})");
    assert!(remaining_x < 150.0 + 40.0 && remaining_y < 150.0 + 40.0 && remaining_x != finished_x, "remaining part at ({remaining_x:.1}, {remaining_y:.1})");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_credit_and_opacity_apply() {
    let (root, template) = write_tile_pack("zoom-credit");
    let image = preview(&template, serde_json::json!({ "map_opacity": 0.5 }), 0.0);
    let corner = image.get_pixel(3, 3);
    assert!((100..=140).contains(&corner[3]), "half-transparent map, got {corner:?}");
    // The credit pill is opaque white in the bottom-right corner, over the map.
    let pill = image.get_pixel(296, 296);
    assert!(pill[3] > 150 && pill[0] > 200, "credit background, got {pill:?}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn settings_validate_strictly() {
    let template = "https://tiles.example/{z}/{x}/{y}.png";
    assert!(validated(plot(template, serde_json::json!({}))).is_ok());
    let bad = |overrides: serde_json::Value, field: &str| {
        let error = validated(plot(template, overrides)).err().unwrap().to_string();
        assert!(error.contains(field), "{field}: {error}");
    };
    bad(serde_json::json!({ "zoom": 25 }), "zoom");
    bad(serde_json::json!({ "zoom": 3 }), "zoom");
    bad(serde_json::json!({ "map_tile_url": "http://tiles.example/{z}/{x}/{y}.png" }), "map_tile_url");
    bad(serde_json::json!({ "map_attribution": " " }), "map_attribution");
    bad(serde_json::json!({ "map_zoom_offset": 3 }), "map_zoom_offset");
    bad(serde_json::json!({ "marker_size": 0 }), "marker_size");
    bad(serde_json::json!({ "completed_line_color": "blue" }), "completed_line_color");
    bad(serde_json::json!({ "corner_radius": -1 }), "corner_radius");
    for field in [
        "zoom", "corner_radius", "rotate_with_heading", "show_full_activity", "map_tile_url", "map_attribution", "map_opacity",
        "map_zoom_offset", "completed_line_width", "completed_line_color", "completed_line_opacity", "remaining_line_width",
        "remaining_line_color", "remaining_line_opacity", "marker_size", "marker_color", "marker_opacity", "marker_outline_color",
        "marker_outline_width",
    ] {
        let mut missing = plot(template, serde_json::json!({}));
        missing.as_object_mut().unwrap().remove(field);
        let error = validated(missing).err().unwrap_or_else(|| panic!("{field} must be required")).to_string();
        assert!(error.contains(field), "{field}: {error}");
    }
}

#[test]
fn the_video_path_prepares_every_frame_and_draws_the_same_view() {
    let (root, template) = write_tile_pack("zoom-video");
    let config = validated(plot(&template, serde_json::json!({ "marker_size": 14 }))).unwrap();
    let activity = parse_activity_json(&activity_json()).unwrap();
    let mut dense = common::builders::dense_report_with(|_| {});
    dense.frame_count = 3;
    dense.frame_elapsed_seconds = vec![0.0, 0.5, 1.0];
    dense.frame_distance_progress = vec![Some(0.0), Some(0.125), Some(0.25)];
    let mut profiler = ovrley_core::debug::RenderProfiler::default();
    let assets = prepare_render_assets(&test_paths(), &config, &activity, &dense, &mut profiler).unwrap();
    drop(assets);

    // Frame 1 (progress 0.125) is the frame at 1/30 s.
    let out = std::env::temp_dir().join(format!("route_zoom_video_{}.png", std::process::id()));
    render_preview_with_report(&test_paths(), &config, &activity, &dense, 1.0 / 30.0, &out).unwrap();
    let image = image::open(&out).unwrap().to_rgba8();
    let _ = std::fs::remove_file(out);
    let (x, y, count) = centroid(&image, is_red);
    assert!(count > 100, "the target is visible in the rendered frame, found {count} red pixels");
    let (center_x, center_y) = widget_center();
    let offset = midway().start_offset;
    assert!(
        (x - (center_x + offset.0)).abs() < 3.0 && (y - (center_y + offset.1)).abs() < 3.0,
        "target at ({x:.1}, {y:.1}), expected ({:.1}, {:.1})",
        center_x + offset.0,
        center_y + offset.1
    );
    assert!(centroid(&image, is_green).2 > 10, "the arrow is drawn");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_route_that_needs_too_many_tiles_is_refused() {
    // About 110 km of route at street zoom would need thousands of tiles.
    let activity = serde_json::json!({
        "sample_elapsed_seconds": [0.0, 1.0, 2.0],
        "sample_distance_progress": [0.0, 0.5, 1.0],
        "sample_course_points": [[47.0, -122.0], [47.5, -122.0], [48.0, -122.0]],
        "trim_start_seconds": 0.0,
        "trim_end_seconds": 2.0
    });
    let config = validated(plot("file:///nowhere/{z}/{x}/{y}.png", serde_json::json!({ "zoom": 17.0 }))).unwrap();
    let activity = parse_activity_json(&activity.to_string()).unwrap();
    let mut dense = common::builders::dense_report_with(|_| {});
    dense.frame_count = 5000;
    dense.frame_elapsed_seconds = (0..5000).map(|frame| f64::from(frame) / 30.0).collect();
    dense.frame_distance_progress = (0..5000).map(|frame| Some(f64::from(frame) / 4999.0)).collect();
    let mut profiler = ovrley_core::debug::RenderProfiler::default();
    let error = prepare_render_assets(&test_paths(), &config, &activity, &dense, &mut profiler).err().expect("too many tiles").to_string();
    assert!(error.contains("lower the map zoom"), "{error}");
}

#[test]
fn an_unreachable_tile_source_fails_loudly() {
    let error = build_route_zoom_frame_png_command(
        &test_paths(),
        &config_json(plot("file:///definitely/not/here/{z}/{x}/{y}.png", serde_json::json!({}))),
        &activity_json(),
        0.5,
    )
    .err()
    .expect("missing tiles")
    .to_string();
    assert!(error.contains("definitely"), "{error}");
}

#[test]
fn a_config_without_the_widget_is_an_error() {
    let config = serde_json::json!({ "scene": scene(), "values": [], "labels": [], "plots": [] }).to_string();
    let error = build_route_zoom_frame_png_command(&test_paths(), &config, &activity_json(), 0.0).err().unwrap().to_string();
    assert!(error.contains("course_zoom"), "{error}");
}

#[test]
fn the_editor_defaults_are_a_valid_widget() {
    // The frontend materializes these defaults when the widget is added from the menu.
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(common::test_config::repo_git_root().join("assets/standard-widgets.json")).unwrap()).unwrap();
    let defaults = manifest["plot"]["definitions"]["course_zoom"]["defaults"].clone();
    assert_eq!(defaults["value"], "course_zoom");
    validated(defaults).expect("the default close-up map must pass the renderer's validation");
}
