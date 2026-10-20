//! Fixed gauge scale and redline: validation, cache resolution and rendered pixels.

mod common;

use ovrley_core::activity::schema::{DenseActivityReport, ParsedActivity};
use ovrley_core::debug::RenderProfiler;
use ovrley_core::normalize::raw::RenderConfig;
use ovrley_core::normalize::{validate_render_config, ValidatedRenderConfig};
use ovrley_core::paths::AppPaths;
use ovrley_core::render::widgets::types::PreparedValue;
use ovrley_core::render::{render_preview_with_report, widgets::prepare_render_assets};
use ovrley_core::MetricKind;
use std::collections::BTreeMap;

const NORMAL_FILL: [u8; 3] = [0x40, 0xe0, 0xd0];
const REDLINE: [u8; 3] = [0xff, 0x00, 0x00];

fn with_scale(mut value: serde_json::Value) -> serde_json::Value {
    value["scale_min"] = serde_json::json!(0);
    value["scale_max"] = serde_json::json!(11000);
    value["redline_value"] = serde_json::json!(8800);
    value["redline_color"] = serde_json::json!("#ff0000");
    value
}

fn rpm_linear(bars: bool) -> serde_json::Value {
    let mut value = serde_json::json!({
        "value": "rpm",
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
        "min_max_label_color": "#ffffff"
    });
    if bars {
        value["track_fill_style"] = serde_json::json!("bars");
        value["bar_count"] = serde_json::json!(10);
        value["bar_gap"] = serde_json::json!(0);
    }
    value
}

fn rpm_arc() -> serde_json::Value {
    serde_json::json!({
        "value": "rpm",
        "x": 20,
        "y": 30,
        "display_type": "arc",
        "width": 160,
        "height": 160,
        "rotation": 0,
        "arc_angle": 180,
        "inner_widget_offset_x": 0,
        "inner_widget_offset_y": 0,
        "track_thickness": 14,
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
        "min_max_label_color": "#ffffff",
        "font": "Arial.ttf",
        "font_size": 30,
        "color": "#ffffff",
        "opacity": 1,
        "show_units": false,
        "unit_color": "#ffffff",
        "display_unit": "rpm",
        "prefix": "",
        "suffix": "",
        "decimals": 0
    })
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

fn dense_rpm(rpm: Vec<Option<f64>>) -> DenseActivityReport {
    let frame_count = rpm.len();
    let mut series = common::builders::empty_dense_series();
    series.rpm = rpm;
    DenseActivityReport {
        frame_count,
        frame_elapsed_seconds: (0..frame_count).map(|i| i as f64).collect(),
        frame_distance_progress: vec![Some(0.0); frame_count],
        full_activity_distance: None,
        full_activity_total_ascent: None,
        full_activity_metric_ranges: ovrley_core::activity::metric_ranges::calculate_metric_range(
            &series.rpm,
        )
        .map(|range| (MetricKind::Rpm, range))
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

fn render_frame(config: &ValidatedRenderConfig, rpm: f64, name: &str) -> image::RgbaImage {
    let activity: ParsedActivity = serde_json::from_value(serde_json::json!({})).unwrap();
    let dense = dense_rpm(vec![Some(rpm), Some(rpm)]);
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
fn scale_requires_both_bounds_and_an_increasing_range() {
    let mut only_min = rpm_linear(false);
    only_min["scale_min"] = serde_json::json!(0);
    let error = config_for(only_min).err().unwrap().to_string();
    assert!(error.contains("scale_max: required when scale_min is set"), "{error}");

    let mut inverted = rpm_linear(false);
    inverted["scale_min"] = serde_json::json!(5000);
    inverted["scale_max"] = serde_json::json!(1000);
    let error = config_for(inverted).err().unwrap().to_string();
    assert!(error.contains("scale_max: must be greater than"), "{error}");
}

#[test]
fn redline_must_sit_inside_the_scale_and_carry_a_color() {
    let mut outside = with_scale(rpm_linear(false));
    outside["redline_value"] = serde_json::json!(12000);
    let error = config_for(outside).err().unwrap().to_string();
    assert!(error.contains("redline_value: must be between"), "{error}");

    let mut no_color = with_scale(rpm_linear(false));
    no_color.as_object_mut().unwrap().remove("redline_color");
    let error = config_for(no_color).err().unwrap().to_string();
    assert!(error.contains("redline_color"), "{error}");
}

#[test]
fn gauges_without_scale_fields_keep_the_automatic_range() {
    for value in [rpm_linear(false), rpm_arc()] {
        let config = config_for(value).unwrap();
        let assets = prepare_render_assets(
            &test_paths(),
            &config,
            &serde_json::from_value::<ParsedActivity>(serde_json::json!({})).unwrap(),
            &dense_rpm(vec![Some(2000.0), Some(4000.0), Some(6000.0)]),
            &mut RenderProfiler::default(),
        )
        .unwrap();
        let fills: Vec<f32> = match &assets.values()[0] {
            PreparedValue::LinearGauge(widget) => {
                let cache = widget.cache.as_ref().unwrap();
                assert!(cache.redline.is_none());
                cache.frame_states.iter().map(|state| state.fill01).collect()
            }
            PreparedValue::ArcGauge(widget) => {
                let cache = widget.cache.as_ref().unwrap();
                assert!(cache.redline.is_none());
                cache.frame_states.iter().map(|state| state.fill01).collect()
            }
            other => panic!("expected a gauge, got {other:?}"),
        };
        assert_eq!(fills, vec![0.0, 0.5, 1.0]);
    }
}

#[test]
fn fixed_scale_ignores_the_observed_range_and_places_the_redline() {
    for value in [with_scale(rpm_linear(false)), with_scale(rpm_arc())] {
        let config = config_for(value).unwrap();
        let assets = prepare_render_assets(
            &test_paths(),
            &config,
            &serde_json::from_value::<ParsedActivity>(serde_json::json!({})).unwrap(),
            &dense_rpm(vec![Some(2000.0), Some(5500.0), Some(11000.0)]),
            &mut RenderProfiler::default(),
        )
        .unwrap();
        let (fills, redline) = match &assets.values()[0] {
            PreparedValue::LinearGauge(widget) => {
                let cache = widget.cache.as_ref().unwrap();
                (
                    cache.frame_states.iter().map(|state| state.fill01).collect::<Vec<_>>(),
                    cache.redline.clone().unwrap(),
                )
            }
            PreparedValue::ArcGauge(widget) => {
                let cache = widget.cache.as_ref().unwrap();
                (
                    cache.frame_states.iter().map(|state| state.fill01).collect::<Vec<_>>(),
                    cache.redline.clone().unwrap(),
                )
            }
            other => panic!("expected a gauge, got {other:?}"),
        };
        assert!((fills[0] - 2000.0 / 11000.0).abs() < 1e-6);
        assert_eq!(fills[1], 0.5);
        assert_eq!(fills[2], 1.0);
        assert!((redline.fraction - 0.8).abs() < 1e-6);
        assert_eq!(redline.color, "#ff0000");
    }
}

#[test]
fn linear_fill_changes_color_at_the_redline() {
    let config = config_for(with_scale(rpm_linear(false))).unwrap();
    let full = render_frame(&config, 11000.0, "gauge_scale_linear_full.png");
    // Track spans x = 20..220 at y = 30..70; the redline starts 80% along it (x = 180).
    assert_color(&full, 100, 50, NORMAL_FILL);
    assert_color(&full, 170, 50, NORMAL_FILL);
    assert_color(&full, 190, 50, REDLINE);
    assert_color(&full, 215, 50, REDLINE);
}

#[test]
fn linear_empty_track_shows_the_redline_zone() {
    let config = config_for(with_scale(rpm_linear(false))).unwrap();
    let empty = render_frame(&config, 0.0, "gauge_scale_linear_empty.png");
    assert_grey_empty(&empty, 100, 50);
    assert_grey_empty(&empty, 170, 50);
    assert_red_tinted_empty(&empty, 190, 50);
    assert_red_tinted_empty(&empty, 215, 50);
}

#[test]
fn partial_fill_below_the_redline_stays_in_the_normal_color() {
    let config = config_for(with_scale(rpm_linear(false))).unwrap();
    let image = render_frame(&config, 5500.0, "gauge_scale_linear_partial.png");
    assert_color(&image, 60, 50, NORMAL_FILL);
    assert_grey_empty(&image, 150, 50);
    assert_red_tinted_empty(&image, 200, 50);
}

#[test]
fn segmented_bars_switch_color_from_the_redline_bar() {
    let config = config_for(with_scale(rpm_linear(true))).unwrap();
    let full = render_frame(&config, 11000.0, "gauge_scale_bars_full.png");
    // Ten bars of 20 px: bars 0-7 are normal, bars 8-9 start at or past 80%.
    assert_color(&full, 20 + 7 * 20 + 10, 50, NORMAL_FILL);
    assert_color(&full, 20 + 8 * 20 + 10, 50, REDLINE);
    assert_color(&full, 20 + 9 * 20 + 10, 50, REDLINE);

    let empty = render_frame(&config, 0.0, "gauge_scale_bars_empty.png");
    assert_grey_empty(&empty, 20 + 7 * 20 + 10, 50);
    assert_red_tinted_empty(&empty, 20 + 8 * 20 + 10, 50);
}

#[test]
fn arc_fill_and_empty_track_use_the_redline_color_past_the_threshold() {
    let config = config_for(with_scale(rpm_arc())).unwrap();
    let assets = prepare_render_assets(
        &test_paths(),
        &config,
        &serde_json::from_value::<ParsedActivity>(serde_json::json!({})).unwrap(),
        &dense_rpm(vec![Some(0.0)]),
        &mut RenderProfiler::default(),
    )
    .unwrap();
    let PreparedValue::ArcGauge(widget) = &assets.values()[0] else {
        panic!("expected an arc gauge");
    };
    let cache = widget.cache.as_ref().unwrap();
    let point = |fraction: f32| {
        let angle = (cache.start_angle + cache.sweep_angle * fraction).to_radians();
        (
            (cache.x + cache.center_x + cache.radius * angle.cos()).round() as u32,
            (cache.y + cache.center_y + cache.radius * angle.sin()).round() as u32,
        )
    };
    let (normal_x, normal_y) = point(0.5);
    let (red_x, red_y) = point(0.95);

    let full = render_frame(&config, 11000.0, "gauge_scale_arc_full.png");
    assert_color(&full, normal_x, normal_y, NORMAL_FILL);
    assert_color(&full, red_x, red_y, REDLINE);

    let empty = render_frame(&config, 0.0, "gauge_scale_arc_empty.png");
    assert_grey_empty(&empty, normal_x, normal_y);
    assert_red_tinted_empty(&empty, red_x, red_y);
}
