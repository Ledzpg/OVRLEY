#![recursion_limit = "512"]
//! Dial ticks and numerals, and the redline on vertical linear gauges.

mod common;

use ovrley_core::activity::schema::{DenseActivityReport, ParsedActivity};
use ovrley_core::debug::RenderProfiler;
use ovrley_core::normalize::raw::RenderConfig;
use ovrley_core::normalize::{validate_render_config, TickPosition, ValidatedGaugeTicks, ValidatedRenderConfig};
use ovrley_core::paths::AppPaths;
use ovrley_core::render::widgets::types::PreparedValue;
use ovrley_core::render::{render_preview_with_report, widgets::prepare_render_assets};
use ovrley_core::MetricKind;
use std::collections::BTreeMap;

fn dial() -> serde_json::Value {
    serde_json::json!({
        "value": "rpm", "x": 20, "y": 20, "display_type": "arc",
        "width": 360, "height": 360, "rotation": 0, "arc_angle": 270,
        "inner_widget_offset_x": 0, "inner_widget_offset_y": 20,
        "track_thickness": 8, "track_corner_radius": 0, "track_border_thickness": 0,
        "track_border_color": "#ffffff", "track_empty_color": "#000000", "track_empty_opacity": 0,
        "track_filled_color": "#00ff00", "track_filled_opacity": 1, "track_fill_flat": true,
        "show_min_max_labels": false, "min_max_label_font": "Arial.ttf", "min_max_label_font_size": 12,
        "min_max_label_color": "#ffffff",
        "font": "Arial.ttf", "font_size": 40, "color": "#ffffff", "opacity": 1,
        "show_units": false, "unit_color": "#ffffff", "display_unit": "rpm", "prefix": "", "suffix": "", "decimals": 0,
        "scale_min": 0, "scale_max": 8000, "redline_value": 6500, "redline_color": "#ff0000",
        "tick_major_interval": 1000, "tick_minor_count": 4, "tick_position": "inside",
        "tick_color": "#ffffff", "tick_major_length": 14, "tick_minor_length": 7, "tick_thickness": 3,
        "tick_offset": 8, "tick_show_labels": false, "tick_label_divisor": 1000, "tick_label_decimals": 0,
        "tick_label_font_size": 26, "tick_label_color": "#ffffff", "tick_label_offset": 6
    })
}

fn with(mut value: serde_json::Value, key: &str, new: serde_json::Value) -> serde_json::Value {
    value[key] = new;
    value
}

fn config_for(value: serde_json::Value) -> Result<ValidatedRenderConfig, ovrley_core::CoreError> {
    let mut scene = common::builders::scene_json();
    scene["width"] = serde_json::json!(400);
    scene["height"] = serde_json::json!(400);
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

fn dense_rpm(rpm: f64) -> DenseActivityReport {
    let mut series = common::builders::empty_dense_series();
    series.rpm = vec![Some(rpm), Some(rpm)];
    DenseActivityReport {
        frame_count: 2,
        frame_elapsed_seconds: vec![0.0, 1.0],
        frame_distance_progress: vec![Some(0.0); 2],
        full_activity_distance: None,
        full_activity_total_ascent: None,
        full_activity_metric_ranges: std::iter::once((MetricKind::Rpm, (0.0, 11000.0))).collect(),
        series,
    }
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

fn render_frame(config: &ValidatedRenderConfig, rpm: f64, name: &str) -> image::RgbaImage {
    let activity: ParsedActivity = serde_json::from_value(serde_json::json!({})).unwrap();
    let out = std::env::temp_dir().join(name);
    render_preview_with_report(&test_paths(), config, &activity, &dense_rpm(rpm), 0.0, &out).unwrap();
    let image = image::open(&out).unwrap().to_rgba8();
    let _ = std::fs::remove_file(out);
    image
}

fn ticks(major_interval: f64, minor: u32) -> ValidatedGaugeTicks {
    ValidatedGaugeTicks {
        major_interval,
        minor_per_major: minor,
        position: TickPosition::Inside,
        color: "#ffffff".into(),
        major_length: 10.0,
        minor_length: 5.0,
        thickness: 2.0,
        offset: 4.0,
        show_labels: true,
        label_divisor: 1000.0,
        label_decimals: 0,
        label_font_size: 20.0,
        label_color: "#ffffff".into(),
        label_offset: 4.0,
    }
}

#[test]
fn marks_are_anchored_at_multiples_of_the_major_interval() {
    let marks = ticks(1000.0, 4).marks(0.0, 8000.0);
    assert_eq!(marks.len(), 41);
    assert_eq!(marks.iter().filter(|m| m.major).count(), 9);
    assert!(marks[0].major && marks[0].value == 0.0);
    assert!(!marks[1].major && (marks[1].value - 200.0).abs() < 1e-6);
    assert!(marks[5].major && (marks[5].value - 1000.0).abs() < 1e-6);

    // A scale that does not start on a multiple only draws marks inside it.
    let offset = ticks(10.0, 1).marks(45.0, 80.0);
    assert!(offset.iter().all(|m| m.value >= 45.0 && m.value <= 80.0));
    assert_eq!(offset.first().unwrap().value, 45.0);
    assert!(offset.iter().filter(|m| m.major).all(|m| (m.value % 10.0).abs() < 1e-9));
}

#[test]
fn ticks_validate_strictly() {
    assert!(config_for(dial()).is_ok());

    let no_scale = {
        let mut v = dial();
        v.as_object_mut().unwrap().remove("scale_min");
        v.as_object_mut().unwrap().remove("scale_max");
        v.as_object_mut().unwrap().remove("redline_value");
        v
    };
    let error = config_for(no_scale).err().unwrap().to_string();
    assert!(error.contains("ticks require a fixed scale"), "{error}");

    let mut missing = dial();
    missing.as_object_mut().unwrap().remove("tick_color");
    assert!(config_for(missing).err().unwrap().to_string().contains("tick_color"));

    let bad_position = with(dial(), "tick_position", serde_json::json!("sideways"));
    assert!(config_for(bad_position).err().unwrap().to_string().contains("tick_position"));

    let zero_interval = with(dial(), "tick_major_interval", serde_json::json!(0));
    assert!(config_for(zero_interval).err().unwrap().to_string().contains("tick_major_interval"));

    let too_many = with(with(dial(), "tick_major_interval", serde_json::json!(10)), "tick_minor_count", serde_json::json!(9));
    assert!(config_for(too_many).err().unwrap().to_string().contains("exceed"));

    let too_many_minor = with(dial(), "tick_minor_count", serde_json::json!(12));
    assert!(config_for(too_many_minor).err().unwrap().to_string().contains("tick_minor_count"));
}

#[test]
fn dial_ticks_are_drawn_on_the_scale_and_recolored_past_the_redline() {
    let config = config_for(dial()).unwrap();
    let assets = prepare_render_assets(
        &test_paths(),
        &config,
        &serde_json::from_value::<ParsedActivity>(serde_json::json!({})).unwrap(),
        &dense_rpm(0.0),
        &mut RenderProfiler::default(),
    )
    .unwrap();
    let PreparedValue::ArcGauge(widget) = &assets.values()[0] else {
        panic!("expected an arc gauge");
    };
    let cache = widget.cache.as_ref().unwrap();
    // Inside ticks start at radius - thickness/2 - offset and run inward; sample the middle of a major tick.
    let sample = |value: f64, length: f32| {
        let fraction = (value / 8000.0) as f32;
        let angle = (cache.start_angle + cache.sweep_angle * fraction).to_radians();
        let radius = cache.radius - 4.0 - 8.0 - length * 0.5;
        (
            (cache.x + cache.center_x + radius * angle.cos()).round() as u32,
            (cache.y + cache.center_y + radius * angle.sin()).round() as u32,
        )
    };
    let image = render_frame(&config, 0.0, "gauge_ticks_dial.png");

    let (x, y) = sample(2000.0, 14.0);
    assert!(image.get_pixel(x, y).0[3] > 200, "major tick at 2000 must be drawn");
    assert!(image.get_pixel(x, y).0[0] > 200 && image.get_pixel(x, y).0[1] > 200, "tick below the redline is white");

    let (x, y) = sample(7000.0, 14.0);
    let pixel = image.get_pixel(x, y).0;
    assert!(pixel[0] > 200 && pixel[1] < 40 && pixel[3] > 200, "tick past the redline is red, got {pixel:?}");

    // Midway between two minor ticks (1100) nothing is drawn at the minor-tick depth.
    let fraction = (1100.0 / 8000.0) as f32;
    let angle = (cache.start_angle + cache.sweep_angle * fraction).to_radians();
    let radius = cache.radius - 4.0 - 8.0 - 3.0;
    let (x, y) = (
        (cache.x + cache.center_x + radius * angle.cos()).round() as u32,
        (cache.y + cache.center_y + radius * angle.sin()).round() as u32,
    );
    assert_eq!(image.get_pixel(x, y).0[3], 0, "no tick between marks");
}

#[test]
fn dial_numerals_are_drawn_when_enabled() {
    let with_labels = config_for(with(dial(), "tick_show_labels", serde_json::json!(true))).unwrap();
    let without = config_for(dial()).unwrap();
    let count = |image: &image::RgbaImage| image.pixels().filter(|p| p.0[3] > 0).count();
    let labelled = count(&render_frame(&with_labels, 0.0, "gauge_ticks_labels.png"));
    let plain = count(&render_frame(&without, 0.0, "gauge_ticks_plain.png"));
    assert!(labelled > plain + 200, "numerals must add visible pixels ({labelled} vs {plain})");
}

#[test]
fn vertical_linear_gauge_puts_the_redline_zone_at_the_top() {
    let mut value = serde_json::json!({
        "value": "rpm", "x": 20, "y": 30, "display_type": "linear", "width": 40, "height": 200, "rotation": 0,
        "orientation": "vertical", "track_corner_radius": 0, "track_border_thickness": 0,
        "track_border_color": "#ffffff", "track_empty_color": "#222222", "track_empty_opacity": 0.5,
        "track_filled_color": "#40e0d0", "track_filled_opacity": 1, "track_fill_flat": true,
        "show_min_max_labels": false, "min_max_label_font": "Arial.ttf", "min_max_label_font_size": 12,
        "min_max_label_position": "left", "min_max_label_color": "#ffffff",
        "scale_min": 0, "scale_max": 11000, "redline_value": 8800, "redline_color": "#ff0000"
    });
    value["redline_color"] = serde_json::json!("#ff0000");
    let config = config_for(value).unwrap();

    // Full: the top 20% (y = 30..70) is the redline, the rest is the normal fill.
    let full = render_frame(&config, 11000.0, "gauge_vertical_full.png");
    let top = full.get_pixel(40, 45).0;
    let bottom = full.get_pixel(40, 200).0;
    assert!(top[0] > 240 && top[1] < 10, "top of a full vertical gauge is red, got {top:?}");
    assert!(bottom[0] < 100 && bottom[1] > 200, "bottom is the normal fill, got {bottom:?}");

    // Empty: the same zone is tinted, the lower part stays grey.
    let empty = render_frame(&config, 0.0, "gauge_vertical_empty.png");
    let top = empty.get_pixel(40, 45).0;
    let bottom = empty.get_pixel(40, 200).0;
    assert!(top[0] > 200 && top[1] < 40 && (100..=160).contains(&top[3]), "got {top:?}");
    assert!((bottom[0] as i32 - 0x22).abs() <= 3 && (100..=160).contains(&bottom[3]), "got {bottom:?}");
}
