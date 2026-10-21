#![recursion_limit = "512"]
//! Fill color bands (solid and gradient) and the full redline fill, steady or blinking.

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

fn zones() -> serde_json::Value {
    serde_json::json!([
        { "start": 0, "color": "#0000ff", "end_color": "#00ff00" },
        { "start": 7000, "color": "#00ff00" },
        { "start": 8000, "color": "#ffff00" }
    ])
}

fn linear() -> serde_json::Value {
    serde_json::json!({
        "value": "rpm", "x": 20, "y": 30, "display_type": "linear", "width": 200, "height": 40, "rotation": 0,
        "orientation": "horizontal", "track_corner_radius": 0, "track_border_thickness": 0,
        "track_border_color": "#ffffff", "track_empty_color": "#222222", "track_empty_opacity": 0.5,
        "track_filled_color": "#ffffff", "track_filled_opacity": 1, "track_fill_flat": true,
        "show_min_max_labels": false, "min_max_label_font": "Arial.ttf", "min_max_label_font_size": 12,
        "min_max_label_position": "bottom", "min_max_label_color": "#ffffff",
        "scale_min": 0, "scale_max": 11000, "redline_value": 10000, "redline_color": "#ff0000",
        "redline_fill_all": true, "redline_blink_hz": 2, "fill_zones": zones()
    })
}

fn arc() -> serde_json::Value {
    serde_json::json!({
        "value": "rpm", "x": 20, "y": 20, "display_type": "arc", "width": 360, "height": 360, "rotation": 0,
        "arc_angle": 270, "inner_widget_offset_x": 0, "inner_widget_offset_y": 0,
        "track_thickness": 20, "track_corner_radius": 0, "track_border_thickness": 0,
        "track_border_color": "#ffffff", "track_empty_color": "#222222", "track_empty_opacity": 0.5,
        "track_filled_color": "#ffffff", "track_filled_opacity": 1, "track_fill_flat": true,
        "show_min_max_labels": false, "min_max_label_font": "Arial.ttf", "min_max_label_font_size": 12,
        "min_max_label_color": "#ffffff", "font": "Arial.ttf", "font_size": 40, "color": "#ffffff", "opacity": 1,
        "show_units": false, "unit_color": "#ffffff", "display_unit": "rpm", "prefix": "", "suffix": "", "decimals": 0,
        "scale_min": 0, "scale_max": 11000, "redline_value": 10000, "redline_color": "#ff0000",
        "redline_fill_all": true, "fill_zones": zones()
    })
}

fn with(mut value: serde_json::Value, key: &str, new: serde_json::Value) -> serde_json::Value {
    value[key] = new;
    value
}

fn without(mut value: serde_json::Value, key: &str) -> serde_json::Value {
    value.as_object_mut().unwrap().remove(key);
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

fn dense_rpm(rpm: f64, frame_seconds: f64) -> DenseActivityReport {
    let mut series = common::builders::empty_dense_series();
    series.rpm = vec![Some(rpm), Some(rpm)];
    DenseActivityReport {
        frame_count: 2,
        frame_elapsed_seconds: vec![frame_seconds, frame_seconds + 1.0],
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

fn render(config: &ValidatedRenderConfig, rpm: f64, seconds: f64, name: &str) -> image::RgbaImage {
    let activity: ParsedActivity = serde_json::from_value(serde_json::json!({})).unwrap();
    let out = std::env::temp_dir().join(name);
    render_preview_with_report(&test_paths(), config, &activity, &dense_rpm(rpm, seconds), 0.0, &out).unwrap();
    let image = image::open(&out).unwrap().to_rgba8();
    let _ = std::fs::remove_file(out);
    image
}

fn near(pixel: [u8; 4], expected: [u8; 3], tolerance: i32) -> bool {
    (0..3).all(|c| (pixel[c] as i32 - expected[c] as i32).abs() <= tolerance)
}

/// x of a value on the horizontal linear track (x = 20..220 for 0..11000 rpm).
fn x_at(value: f64) -> u32 {
    (20.0 + 200.0 * value / 11000.0).round() as u32
}

#[test]
fn zones_validate_strictly() {
    assert!(config_for(linear()).is_ok());
    assert!(config_for(arc()).is_ok());

    let no_scale = without(without(without(linear(), "scale_min"), "scale_max"), "redline_value");
    let no_scale = without(without(no_scale, "redline_fill_all"), "redline_blink_hz");
    assert!(config_for(no_scale).err().unwrap().to_string().contains("fill zones require a fixed scale"));

    let late_first = with(linear(), "fill_zones", serde_json::json!([{ "start": 500, "color": "#ffffff" }]));
    assert!(config_for(late_first).err().unwrap().to_string().contains("first zone must start at scale_min"));

    let unordered = with(
        linear(),
        "fill_zones",
        serde_json::json!([{ "start": 0, "color": "#ffffff" }, { "start": 5000, "color": "#ffffff" }, { "start": 5000, "color": "#ffffff" }]),
    );
    assert!(config_for(unordered).err().unwrap().to_string().contains("previous zone start"));

    let past_max = with(linear(), "fill_zones", serde_json::json!([{ "start": 0, "color": "#ffffff" }, { "start": 11000, "color": "#ffffff" }]));
    assert!(config_for(past_max).err().unwrap().to_string().contains("below scale_max"));

    let bad_color = with(linear(), "fill_zones", serde_json::json!([{ "start": 0, "color": "blue" }]));
    assert!(config_for(bad_color).err().unwrap().to_string().contains("fill_zones[0].color"));

    let empty = with(linear(), "fill_zones", serde_json::json!([]));
    assert!(config_for(empty).err().unwrap().to_string().contains("fill_zones"));
}

#[test]
fn full_redline_fill_and_blink_validate() {
    let fast = with(linear(), "redline_blink_hz", serde_json::json!(50));
    assert!(config_for(fast).err().unwrap().to_string().contains("redline_blink_hz"));
    let zero = with(linear(), "redline_blink_hz", serde_json::json!(0));
    assert!(config_for(zero).err().unwrap().to_string().contains("redline_blink_hz"));

    let orphan = without(without(without(linear(), "redline_value"), "redline_color"), "fill_zones");
    assert!(config_for(orphan).err().unwrap().to_string().contains("requires redline_value"));
}

#[test]
fn linear_zones_draw_a_gradient_a_solid_band_and_yellow() {
    let config = config_for(with(linear(), "redline_fill_all", serde_json::json!(false))).unwrap();
    // 9000 rpm: below the redline, so the zones are visible up to x = 183.
    let image = render(&config, 9000.0, 0.0, "gauge_zones_linear.png");
    let at = |value: f64| image.get_pixel(x_at(value), 50).0;

    let near_start = at(500.0);
    assert!(near_start[2] > 200 && near_start[1] < 60, "start of the gradient is blue, got {near_start:?}");
    let middle = at(3500.0);
    assert!(middle[1] > 100 && middle[2] > 100 && middle[0] < 30, "middle of the gradient blends blue and green, got {middle:?}");
    let end = at(6800.0);
    assert!(end[1] > 230 && end[2] < 40, "end of the gradient is green, got {end:?}");
    assert!(near(at(7500.0), [0, 255, 0], 12), "solid band is green, got {:?}", at(7500.0));
    assert!(near(at(8500.0), [255, 255, 0], 12), "yellow band, got {:?}", at(8500.0));
}

#[test]
fn reaching_the_redline_fills_everything_in_red() {
    let config = config_for(with(linear(), "redline_blink_hz", serde_json::Value::Null)).unwrap();
    let image = render(&config, 10500.0, 0.0, "gauge_zones_overrun.png");
    for value in [500.0, 4000.0, 7500.0, 9000.0, 10800.0] {
        let pixel = image.get_pixel(x_at(value), 50).0;
        assert!(near(pixel, [255, 0, 0], 12), "{value} rpm must be red, got {pixel:?}");
    }
    // Just below the redline the zones are still shown.
    let below = render(&config, 9900.0, 0.0, "gauge_zones_below.png");
    assert!(near(below.get_pixel(x_at(8500.0), 50).0, [255, 255, 0], 12));
}

#[test]
fn the_full_redline_fill_blinks_with_the_frame_time() {
    let config = config_for(linear()).unwrap(); // 2 flashes per second: 0.25 s on, 0.25 s off
    let on = render(&config, 10500.0, 0.1, "gauge_blink_on.png");
    assert!(near(on.get_pixel(x_at(4000.0), 50).0, [255, 0, 0], 12), "visible phase is red");

    let off = render(&config, 10500.0, 0.3, "gauge_blink_off.png");
    let pixel = off.get_pixel(x_at(4000.0), 50).0;
    assert!(
        (pixel[0] as i32 - 0x22).abs() <= 3 && (100..=160).contains(&pixel[3]),
        "hidden phase shows only the empty track, got {pixel:?}"
    );

    let on_again = render(&config, 10500.0, 0.55, "gauge_blink_on_again.png");
    assert!(near(on_again.get_pixel(x_at(4000.0), 50).0, [255, 0, 0], 12));
}

#[test]
fn arc_zones_follow_the_track_angle() {
    let config = config_for(with(arc(), "redline_fill_all", serde_json::json!(false))).unwrap();
    let assets = prepare_render_assets(
        &test_paths(),
        &config,
        &serde_json::from_value::<ParsedActivity>(serde_json::json!({})).unwrap(),
        &dense_rpm(9000.0, 0.0),
        &mut RenderProfiler::default(),
    )
    .unwrap();
    let PreparedValue::ArcGauge(widget) = &assets.values()[0] else {
        panic!("expected an arc gauge");
    };
    let cache = widget.cache.as_ref().unwrap();
    let sample = |value: f64| {
        let angle = (cache.start_angle + cache.sweep_angle * (value / 11000.0) as f32).to_radians();
        (
            (cache.x + cache.center_x + cache.radius * angle.cos()).round() as u32,
            (cache.y + cache.center_y + cache.radius * angle.sin()).round() as u32,
        )
    };
    let image = render(&config, 9000.0, 0.0, "gauge_zones_arc.png");
    let color = |value: f64| {
        let (x, y) = sample(value);
        image.get_pixel(x, y).0
    };
    let start = color(400.0);
    assert!(start[2] > 200 && start[1] < 60, "blue at the start, got {start:?}");
    assert!(near(color(7500.0), [0, 255, 0], 14), "solid green band, got {:?}", color(7500.0));
    assert!(near(color(8500.0), [255, 255, 0], 14), "yellow band, got {:?}", color(8500.0));

    let overrun = render(&config_for(arc()).unwrap(), 10500.0, 0.0, "gauge_zones_arc_overrun.png");
    let (x, y) = sample(2000.0);
    assert!(near(overrun.get_pixel(x, y).0, [255, 0, 0], 14), "arc fills completely in red at the redline");
}

#[test]
fn gauges_without_zones_keep_their_single_fill_color() {
    let plain = without(without(without(linear(), "fill_zones"), "redline_fill_all"), "redline_blink_hz");
    let config = config_for(plain).unwrap();
    let image = render(&config, 9000.0, 0.0, "gauge_zones_none.png");
    for value in [500.0, 4000.0, 8500.0] {
        assert!(near(image.get_pixel(x_at(value), 50).0, [255, 255, 255], 6));
    }
}

#[test]
fn a_gradient_in_the_last_band_reaches_the_redline_color_at_the_redline() {
    let bands = serde_json::json!([
        { "start": 0, "color": "#0000ff", "end_color": "#00ff00" },
        { "start": 7000, "color": "#00ff00" },
        { "start": 8000, "color": "#ffff00", "end_color": "#ff0000" }
    ]);
    let config = config_for(with(with(linear(), "fill_zones", bands), "redline_fill_all", serde_json::json!(false))).unwrap();
    // 9990 rpm is just below the redline, so the whole 8000..10000 band is visible.
    let image = render(&config, 9990.0, 0.0, "gauge_zones_last_gradient.png");
    let at = |value: f64| image.get_pixel(x_at(value), 50).0;

    assert!(near(at(8100.0), [255, 242, 0], 12), "starts at yellow, got {:?}", at(8100.0));
    let middle = at(9000.0);
    assert!(middle[0] > 240 && (110..=145).contains(&middle[1]) && middle[2] < 10, "halfway to the redline is orange, got {middle:?}");
    // Sampled a little inside the fill edge, where antialiasing no longer blends with the empty track.
    let end = at(9800.0);
    assert!(end[0] > 240 && end[1] < 40, "close to the redline it is almost the redline color, got {end:?}");
}

#[test]
fn a_gradient_that_follows_the_next_band_ends_in_that_bands_color() {
    // The first band blends into the second band's color (green) and the last one into the redline (red).
    let bands = serde_json::json!([
        { "start": 0, "color": "#0000ff", "end_follows_next": true },
        { "start": 7000, "color": "#00ff00" },
        { "start": 8000, "color": "#ffff00", "end_follows_next": true }
    ]);
    let config = config_for(with(with(linear(), "fill_zones", bands), "redline_fill_all", serde_json::json!(false))).unwrap();
    let image = render(&config, 9990.0, 0.0, "gauge_zones_follows_next.png");
    let at = |value: f64| image.get_pixel(x_at(value), 50).0;

    let near_start = at(500.0);
    assert!(near_start[2] > 200 && near_start[1] < 60, "start of the first band is blue, got {near_start:?}");
    let end = at(6800.0);
    assert!(end[1] > 230 && end[2] < 40, "the first band ends in the second band's green, got {end:?}");
    let last_end = at(9800.0);
    assert!(last_end[0] > 240 && last_end[1] < 40, "the last band ends in the redline color, got {last_end:?}");
}

#[test]
fn following_the_next_band_is_the_same_as_naming_its_color() {
    let followed = serde_json::json!([
        { "start": 0, "color": "#0000ff", "end_follows_next": true },
        { "start": 7000, "color": "#00ff00" }
    ]);
    let named = serde_json::json!([
        { "start": 0, "color": "#0000ff", "end_color": "#00ff00" },
        { "start": 7000, "color": "#00ff00" }
    ]);
    let render_with = |bands: serde_json::Value, name: &str| {
        let config = config_for(with(with(linear(), "fill_zones", bands), "redline_fill_all", serde_json::json!(false))).unwrap();
        render(&config, 9000.0, 0.0, name)
    };

    let a = render_with(followed, "gauge_zones_same_a.png");
    let b = render_with(named, "gauge_zones_same_b.png");

    assert_eq!(a.as_raw(), b.as_raw());
}

#[test]
fn a_band_cannot_have_both_an_end_color_and_follow_the_next_band() {
    let bands = serde_json::json!([
        { "start": 0, "color": "#0000ff", "end_color": "#00ff00", "end_follows_next": true },
        { "start": 7000, "color": "#00ff00" }
    ]);

    let error = config_for(with(linear(), "fill_zones", bands)).err().unwrap().to_string();

    assert!(error.contains("fill_zones[0].end_color: cannot be combined with end_follows_next"), "got {error}");
}
