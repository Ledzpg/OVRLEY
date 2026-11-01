//! Critical level of the fuel gauge: the redline of a metric whose low end is the danger.
//!
//! The zone is everything from the start of the scale up to the configured value, and the
//! gauge fills completely (and may blink) once the value drops to it.

mod common;

use ovrley_core::activity::schema::{DenseActivityReport, ParsedActivity};
use ovrley_core::normalize::raw::RenderConfig;
use ovrley_core::normalize::{validate_render_config, ValidatedRenderConfig};
use ovrley_core::paths::AppPaths;
use ovrley_core::render::render_preview_with_report;
use ovrley_core::standard_metrics::{standard_metric_redline_zone, RedlineZone};
use ovrley_core::MetricKind;
use std::collections::BTreeMap;

const NORMAL_FILL: [u8; 3] = [0x40, 0xe0, 0xd0];
const CRITICAL: [u8; 3] = [0xff, 0x00, 0x00];

/// Fuel bar 200 x 40 at (20, 30), scale 0..21 L, critical level 3 L (14.3% along the track, x = 48.6).
fn fuel_linear(bars: bool, fill_all: bool) -> serde_json::Value {
    let mut value = serde_json::json!({
        "value": "fuel_level",
        "x": 20,
        "y": 30,
        "display_type": "linear",
        "width": 200,
        "height": 40,
        "rotation": 0,
        "orientation": "horizontal",
        "track_corner_radius": 0,
        "track_border_thickness": 0,
        "track_border_color": "#ffffff",
        "track_empty_color": "#222222",
        "track_empty_opacity": 0.5,
        "track_filled_color": "#40e0d0",
        "track_filled_opacity": 1,
        "track_fill_flat": true,
        "show_min_max_labels": false,
        "min_max_label_font": "Arial.ttf",
        "min_max_label_font_size": 12,
        "min_max_label_position": "bottom",
        "min_max_label_color": "#ffffff",
        "display_unit": "l",
        "fuel_tank_capacity": 21,
        "scale_min": 0,
        "scale_max": 21,
        "redline_value": 3,
        "redline_color": "#ff0000"
    });
    if fill_all {
        value["redline_fill_all"] = serde_json::json!(true);
    }
    if bars {
        value["track_fill_style"] = serde_json::json!("bars");
        value["bar_count"] = serde_json::json!(10);
        value["bar_gap"] = serde_json::json!(0);
    }
    value
}

fn config_for(value: serde_json::Value) -> Result<ValidatedRenderConfig, ovrley_core::CoreError> {
    let mut scene = common::builders::scene_json();
    scene["width"] = serde_json::json!(320);
    scene["height"] = serde_json::json!(240);
    validate_render_config(RenderConfig {
        scene: serde_json::from_value(scene).unwrap(),
        backdrops: vec![],
        rasters: vec![],
        labels: vec![],
        values: vec![serde_json::from_value(value).unwrap()],
        plots: serde_json::Value::Object(serde_json::Map::new()),
        extra: BTreeMap::new(),
    })
}

fn dense_fuel(fuel: Vec<Option<f64>>) -> DenseActivityReport {
    let frame_count = fuel.len();
    let mut series = common::builders::empty_dense_series();
    series.fuel_level = fuel;
    DenseActivityReport {
        frame_count,
        frame_elapsed_seconds: (0..frame_count).map(|i| i as f64).collect(),
        frame_distance_progress: vec![Some(0.0); frame_count],
        full_activity_distance: None,
        full_activity_total_ascent: None,
        full_activity_metric_ranges: ovrley_core::activity::metric_ranges::calculate_metric_range(
            &series.fuel_level,
        )
        .map(|range| (MetricKind::FuelLevel, range))
        .into_iter()
        .collect(),
        series,
    }
}

fn test_paths() -> AppPaths {
    let workspace_root = common::test_config::repo_git_root();
    AppPaths {
        repo_root: workspace_root.clone(),
        font_dirs: vec![workspace_root.join("fonts")],
        debug_render_dir: std::env::temp_dir(),
        temp_dir: std::env::temp_dir(),
        bundled_templates_dirs: vec![],
        user_templates_dir: std::env::temp_dir(),
        downloads_dir: std::env::temp_dir(),
    }
}

fn render_frame(config: &ValidatedRenderConfig, fuel: f64, name: &str) -> image::RgbaImage {
    let activity: ParsedActivity = serde_json::from_value(serde_json::json!({})).unwrap();
    let dense = dense_fuel(vec![Some(fuel), Some(fuel)]);
    let out_path = std::env::temp_dir().join(name);
    render_preview_with_report(&test_paths(), config, &activity, &dense, 0.0, &out_path).unwrap();
    let image = image::open(&out_path).unwrap().to_rgba8();
    let _ = std::fs::remove_file(out_path);
    image
}

fn assert_color(image: &image::RgbaImage, x: u32, y: u32, expected: [u8; 3]) {
    let pixel = image.get_pixel(x, y).0;
    for channel in 0..3 {
        assert!(
            (pixel[channel] as i32 - expected[channel] as i32).abs() <= 3,
            "pixel ({x},{y}) was {pixel:?}, expected rgb {expected:?}"
        );
    }
}

/// Reports whether the pixel is the tinted (half-transparent) empty track in red.
/// Reports whether the pixel is the tinted (half-transparent) empty track in red.
fn assert_red_tinted_empty(image: &image::RgbaImage, x: u32, y: u32) {
    let pixel = image.get_pixel(x, y).0;
    assert!(
        pixel[0] > 200 && pixel[1] < 40 && pixel[2] < 40 && (100..=160).contains(&pixel[3]),
        "pixel ({x},{y}) was {pixel:?}, expected a half-transparent red empty track"
    );
}

fn assert_grey_empty(image: &image::RgbaImage, x: u32, y: u32) {
    let pixel = image.get_pixel(x, y).0;
    assert!(
        (pixel[0] as i32 - 0x22).abs() <= 3 && (100..=160).contains(&pixel[3]),
        "pixel ({x},{y}) was {pixel:?}, expected the grey empty track"
    );
}

#[test]
fn only_the_fuel_level_marks_its_low_end_as_the_danger() {
    assert_eq!(standard_metric_redline_zone(MetricKind::FuelLevel), RedlineZone::Below);
    assert_eq!(standard_metric_redline_zone(MetricKind::Rpm), RedlineZone::Above);
    assert_eq!(
        standard_metric_redline_zone(MetricKind::CoolantTemperature),
        RedlineZone::Above
    );
}

#[test]
fn the_fill_inside_the_critical_zone_is_red_and_the_rest_keeps_its_color() {
    let config = config_for(fuel_linear(false, false)).unwrap();
    let full = render_frame(&config, 21.0, "critical_level_full.png");
    assert_color(&full, 30, 50, CRITICAL);
    assert_color(&full, 100, 50, NORMAL_FILL);
    assert_color(&full, 215, 50, NORMAL_FILL);

    // A fill that stops inside the zone is red from its first pixel.
    let low = render_frame(&config, 2.0, "critical_level_low.png");
    assert_color(&low, 30, 50, CRITICAL);
    assert_grey_empty(&low, 100, 50);
}

#[test]
fn the_empty_track_shows_the_critical_zone_at_the_start() {
    let config = config_for(fuel_linear(false, false)).unwrap();
    let empty = render_frame(&config, 0.0, "critical_level_empty.png");
    assert_red_tinted_empty(&empty, 30, 50);
    assert_grey_empty(&empty, 100, 50);
    assert_grey_empty(&empty, 215, 50);
}

#[test]
fn reaching_the_critical_level_fills_the_whole_track_in_red() {
    let config = config_for(fuel_linear(false, true)).unwrap();
    let above = render_frame(&config, 10.0, "critical_level_above.png");
    assert_color(&above, 30, 50, CRITICAL);
    assert_color(&above, 100, 50, NORMAL_FILL);

    let at_level = render_frame(&config, 3.0, "critical_level_at.png");
    assert_color(&at_level, 30, 50, CRITICAL);
    assert_color(&at_level, 100, 50, CRITICAL);
    assert_color(&at_level, 215, 50, CRITICAL);

    let below = render_frame(&config, 1.0, "critical_level_below.png");
    assert_color(&below, 215, 50, CRITICAL);
}

#[test]
fn segmented_bars_in_the_critical_zone_are_red() {
    let config = config_for(fuel_linear(true, false)).unwrap();
    let full = render_frame(&config, 21.0, "critical_level_bars_full.png");
    // Ten bars of 20 px: bars 0 and 1 start below 14.3%, bar 2 starts after it.
    assert_color(&full, 20 + 10, 50, CRITICAL);
    assert_color(&full, 20 + 20 + 10, 50, CRITICAL);
    assert_color(&full, 20 + 2 * 20 + 10, 50, NORMAL_FILL);

    let empty = render_frame(&config, 0.0, "critical_level_bars_empty.png");
    assert_red_tinted_empty(&empty, 20 + 10, 50);
    assert_grey_empty(&empty, 20 + 2 * 20 + 10, 50);
}
