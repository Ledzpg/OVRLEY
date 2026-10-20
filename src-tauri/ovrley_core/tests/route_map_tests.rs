#![recursion_limit = "512"]
//! Raster tile map behind the route plot: validation, Web Mercator alignment and failures.
//!
//! The tiles come from a generated `file://` tile pack, so the tests need no network. Every
//! tile is light gray except for a red square centered on the route's first point; the
//! square must end up exactly under the route's start in the rendered frame.

mod common;

use ovrley_core::activity::parse_activity_json;
use ovrley_core::commands::route_geometry::{build_route_geometry_command, build_route_map_png_command};
use ovrley_core::normalize::raw::RenderConfig;
use ovrley_core::normalize::{validate_render_config, ValidatedRenderConfig};
use ovrley_core::paths::AppPaths;
use ovrley_core::render::{render_preview_with_report, widgets::prepare_render_assets};
use std::collections::BTreeMap;
use std::path::PathBuf;

const START: (f64, f64) = (47.6062, -122.3321);
const TARGET_HALF_PX: i64 = 10;

fn course_points() -> serde_json::Value {
    serde_json::json!([
        [47.6062, -122.3321],
        [47.6097, -122.3400],
        [47.6130, -122.3350],
        [47.6101, -122.3290],
        [47.6075, -122.3305]
    ])
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

fn plot(map: serde_json::Value) -> serde_json::Value {
    let mut plot = serde_json::json!({
        "value": "course", "x": 0, "y": 0, "width": 400, "height": 400, "rotation": 0,
        "simplify_tolerance_px": 0.5, "target_density": 1.0, "show_full_activity": true,
        "remaining_line_width": 3, "remaining_line_color": "#0000ff", "remaining_line_opacity": 1,
        "completed_line_width": 3, "completed_line_color": "#0000ff", "completed_line_opacity": 1,
        "marker_variant": "circle", "marker_variant_diameter": 10, "marker_size": 2,
        "marker_color": "#00ff00", "marker_opacity": 1
    });
    for (key, value) in map.as_object().unwrap() {
        plot[key] = value.clone();
    }
    plot
}

fn map_fields(template: &str) -> serde_json::Value {
    serde_json::json!({
        "map_tile_url": template, "map_attribution": "(c) Test tiles", "map_opacity": 1, "map_zoom_offset": 0
    })
}

fn config_json(plot: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "scene": scene(), "values": [], "labels": [], "plots": [plot] })
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

/// Writes tiles for zooms 8-17 around the start point; returns the `file://` template.
fn write_tile_pack(name: &str) -> (PathBuf, String) {
    let root = std::env::temp_dir().join(format!("ovrley-{name}-{}", std::process::id()));
    let (lat, lon) = START;
    let mercator_y = (std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan().ln().to_degrees();
    for zoom in 8..=17u32 {
        let tiles = 2f64.powi(zoom as i32);
        let world = 256.0 * tiles;
        let target_x = ((lon + 180.0) / 360.0 * world).round() as i64;
        let target_y = ((0.5 - mercator_y / 360.0) * world).round() as i64;
        let (center_x, center_y) = (target_x / 256, target_y / 256);
        for tile_x in center_x - 3..=center_x + 3 {
            for tile_y in center_y - 3..=center_y + 3 {
                let image = image::RgbaImage::from_fn(256, 256, |u, v| {
                    let (world_x, world_y) = (tile_x * 256 + u as i64, tile_y * 256 + v as i64);
                    let inside = (world_x - target_x).abs() <= TARGET_HALF_PX && (world_y - target_y).abs() <= TARGET_HALF_PX;
                    if inside {
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

fn render(config: &ValidatedRenderConfig) -> Result<image::RgbaImage, ovrley_core::CoreError> {
    let activity = parse_activity_json(&activity_json()).unwrap();
    let dense = common::builders::dense_report_with(|_| {});
    let out = std::env::temp_dir().join(format!("route_map_{}.png", std::process::id()));
    let mut profiler = ovrley_core::debug::RenderProfiler::default();
    let _ = prepare_render_assets(&test_paths(), config, &activity, &dense, &mut profiler)?;
    render_preview_with_report(&test_paths(), config, &activity, &dense, 0.0, &out)?;
    let image = image::open(&out).unwrap().to_rgba8();
    let _ = std::fs::remove_file(out);
    Ok(image)
}

fn is_red(pixel: &image::Rgba<u8>) -> bool {
    pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60 && pixel[3] > 200
}

/// Centroid and count of the red target pixels in the frame.
fn red_target(image: &image::RgbaImage) -> (f64, f64, usize) {
    let (mut sum_x, mut sum_y, mut count) = (0.0, 0.0, 0usize);
    for (x, y, pixel) in image.enumerate_pixels() {
        if is_red(pixel) {
            sum_x += x as f64;
            sum_y += y as f64;
            count += 1;
        }
    }
    (sum_x / count.max(1) as f64, sum_y / count.max(1) as f64, count)
}

fn start_in_widget(plot: serde_json::Value) -> (f64, f64) {
    let response = build_route_geometry_command(&config_json(plot).to_string(), &activity_json()).unwrap();
    (response.points[0][0] as f64, response.points[0][1] as f64)
}

#[test]
fn the_map_target_lands_under_the_route_start() {
    let (root, template) = write_tile_pack("map-align");
    let plot = plot(map_fields(&template));
    let image = render(&validated(plot.clone()).unwrap()).unwrap();

    let (x, y, count) = red_target(&image);
    assert!(count > 100, "the map target must be visible, found {count} red pixels");
    let (start_x, start_y) = start_in_widget(plot);
    assert!(
        (x - start_x).abs() < 2.5 && (y - start_y).abs() < 2.5,
        "target at ({x:.1}, {y:.1}) but the route starts at ({start_x:.1}, {start_y:.1})"
    );
    // Away from the target the map is the light gray of the tiles, not transparent.
    let corner = image.get_pixel(3, 3);
    assert!(corner[3] > 200 && corner[0] > 150, "map covers the widget, got {corner:?}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_map_png_command_matches_the_render() {
    let (root, template) = write_tile_pack("map-png");
    let config = config_json(plot(map_fields(&template)));
    let png = build_route_map_png_command(&config.to_string(), &activity_json())
        .unwrap()
        .expect("map configured");
    let image = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!((image.width(), image.height()), (400, 400));
    let (x, y, count) = red_target(&image);
    assert!(count > 100);
    let (start_x, start_y) = start_in_widget(plot(map_fields(&template)));
    assert!((x - start_x).abs() < 2.5 && (y - start_y).abs() < 2.5);

    let without = config_json(plot(serde_json::json!({})));
    assert!(build_route_map_png_command(&without.to_string(), &activity_json()).unwrap().is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn opacity_fades_the_map() {
    let (root, template) = write_tile_pack("map-opacity");
    let mut fields = map_fields(&template);
    fields["map_opacity"] = serde_json::json!(0.5);
    let image = render(&validated(plot(fields)).unwrap()).unwrap();
    let corner = image.get_pixel(3, 3);
    assert!((100..=140).contains(&corner[3]), "half-transparent map, got {corner:?}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_route_without_a_map_keeps_the_plain_plot() {
    let image = render(&validated(plot(serde_json::json!({}))).unwrap()).unwrap();
    assert_eq!(red_target(&image).2, 0);
    assert_eq!(image.get_pixel(3, 3)[3], 0, "no background without a map");
}

#[test]
fn map_settings_validate_strictly() {
    let template = "https://tiles.example/{z}/{x}/{y}.png";
    assert!(validated(plot(map_fields(template))).is_ok());

    let mut fields = map_fields(template);
    fields.as_object_mut().unwrap().remove("map_attribution");
    assert!(validated(plot(fields)).err().unwrap().to_string().contains("map_attribution"));

    let mut fields = map_fields(template);
    fields["map_attribution"] = serde_json::json!("   ");
    assert!(validated(plot(fields)).err().unwrap().to_string().contains("map_attribution"));

    let mut fields = map_fields(template);
    fields["map_zoom_offset"] = serde_json::json!(3);
    assert!(validated(plot(fields)).err().unwrap().to_string().contains("map_zoom_offset"));

    let mut fields = map_fields(template);
    fields.as_object_mut().unwrap().remove("map_opacity");
    assert!(validated(plot(fields)).err().unwrap().to_string().contains("map_opacity"));

    for bad in ["http://tiles.example/{z}/{x}/{y}.png", "https://tiles.example/{z}/{x}.png", "tiles/{z}/{x}/{y}.png"] {
        let error = validated(plot(map_fields(bad))).err().unwrap().to_string();
        assert!(error.contains("map_tile_url"), "{bad}: {error}");
    }

    // The other map fields without a tile URL are a contract violation, not a silent no-op.
    let orphan = serde_json::json!({ "map_opacity": 0.5 });
    assert!(validated(plot(orphan)).err().unwrap().to_string().contains("requires map_tile_url"));
}

#[test]
fn an_unreachable_tile_source_fails_loudly() {
    let activity = parse_activity_json(&activity_json()).unwrap();
    let dense = common::builders::dense_report_with(|_| {});
    let mut profiler = ovrley_core::debug::RenderProfiler::default();

    let missing = validated(plot(map_fields("file:///definitely/not/here/{z}/{x}/{y}.png"))).unwrap();
    let error = prepare_render_assets(&test_paths(), &missing, &activity, &dense, &mut profiler)
        .err()
        .expect("missing tile file")
        .to_string();
    assert!(error.contains("definitely"), "{error}");

    let remote = validated(plot(map_fields("https://tiles.example/{z}/{x}/{y}.png"))).unwrap();
    let error = prepare_render_assets(&test_paths(), &remote, &activity, &dense, &mut profiler)
        .err()
        .expect("no tile source installed")
        .to_string();
    assert!(error.contains("no raster tile source is installed"), "{error}");
}
