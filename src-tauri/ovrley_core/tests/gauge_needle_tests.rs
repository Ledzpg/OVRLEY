#![recursion_limit = "512"]
//! Dial needle of arc gauges: validation and the position of the drawn needle.

mod common;

use ovrley_core::activity::schema::{DenseActivityReport, ParsedActivity};
use ovrley_core::normalize::raw::RenderConfig;
use ovrley_core::normalize::{validate_render_config, ValidatedRenderConfig};
use ovrley_core::paths::AppPaths;
use ovrley_core::render::render_preview_with_report;
use ovrley_core::MetricKind;
use std::collections::BTreeMap;

fn zones() -> serde_json::Value {
    serde_json::json!([
        { "start": 0, "color": "#0000ff", "end_color": "#00ff00" },
        { "start": 7000, "color": "#00ff00" },
        { "start": 8000, "color": "#ffff00" }
    ])
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



fn needle_fields() -> serde_json::Value {
    serde_json::json!({
        "needle_length": 40, "needle_width": 10, "needle_offset": 0, "needle_color": "#ff00ff",
        "needle_border_thickness": 2, "needle_fill_opacity": 0.5, "needle_corner_radius": 2
    })
}

fn with_needle() -> serde_json::Value {
    let mut value = without(arc(), "fill_zones");
    for (key, field) in needle_fields().as_object().unwrap() {
        value[key] = field.clone();
    }
    value
}

fn is_magenta(pixel: [u8; 4]) -> bool {
    pixel[3] > 80 && pixel[0] > 150 && pixel[2] > 150 && pixel[1] < 120
}

#[test]
fn needle_fields_validate_strictly() {
    assert!(config_for(with_needle()).is_ok());
    assert!(config_for(arc()).is_ok(), "no needle_length means no needle");

    let incomplete = without(with_needle(), "needle_color");
    assert!(config_for(incomplete).err().unwrap().to_string().contains("needle_color"));
    let flat = with(with_needle(), "needle_width", serde_json::json!(0));
    assert!(config_for(flat).err().unwrap().to_string().contains("needle_width"));
    assert!(config_for(with(with_needle(), "needle_offset", serde_json::json!(-12))).is_ok(), "negative offsets move the needle inward");
    let orphan = without(with_needle(), "needle_length");
    assert!(config_for(orphan).is_ok(), "other needle fields without a length are ignored");
}

#[test]
fn the_needle_sits_across_the_track_at_the_value_angle() {
    // Arc 360x360 at (20, 20): center (200, 200), 270 degrees starting at 135 degrees.
    // Half the scale points straight up, with the track centered 170 px from the center.
    let config = config_for(with_needle()).unwrap();
    let image = render(&config, 5500.0, 0.0, "gauge_needle_top.png");
    let outside_track = image.get_pixel(200, 14).0;
    assert!(is_magenta(outside_track), "needle body sticks out above the track, got {outside_track:?}");
    assert!(image.get_pixel(200, 14).0[3] < 200, "the body is translucent, got {:?}", image.get_pixel(200, 14));
    assert_eq!(image.get_pixel(232, 14).0[3], 0, "nothing beside the needle");
    let inside_track_edge = image.get_pixel(200, 48).0;
    assert!(is_magenta(inside_track_edge) || inside_track_edge[3] > 80, "needle reaches below the track, got {inside_track_edge:?}");
}

#[test]
fn the_needle_follows_the_value() {
    let config = config_for(with_needle()).unwrap();
    // 0 rpm: angle 135 degrees, bottom-left. 11000 rpm would be 45 degrees, bottom-right.
    let at_zero = render(&config, 0.0, 0.0, "gauge_needle_zero.png");
    let position = |degrees: f32, radius: f32| -> (u32, u32) {
        let radians = degrees.to_radians();
        ((200.0 + radius * radians.cos()).round() as u32, (200.0 + radius * radians.sin()).round() as u32)
    };
    let (x, y) = position(135.0, 188.0);
    assert!(is_magenta(at_zero.get_pixel(x, y).0), "needle at the start of the scale, got {:?}", at_zero.get_pixel(x, y));
    let (other_x, other_y) = position(45.0, 188.0);
    assert_eq!(at_zero.get_pixel(other_x, other_y).0[3], 0, "not at the end of the scale");
    assert_eq!(at_zero.get_pixel(200, 14).0[3], 0, "not at the top");

    let at_end = render(&config, 11000.0, 0.0, "gauge_needle_end.png");
    // The redline fill takes over the track at 11000, but the needle still marks the end of the scale.
    assert!(is_magenta(at_end.get_pixel(other_x, other_y).0), "needle at the end, got {:?}", at_end.get_pixel(other_x, other_y));
}

#[test]
fn a_gauge_without_a_needle_draws_none() {
    let config = config_for(without(arc(), "fill_zones")).unwrap();
    let image = render(&config, 5500.0, 0.0, "gauge_needle_none.png");
    assert_eq!(image.get_pixel(200, 14).0[3], 0);
}

#[test]
fn a_hollow_needle_keeps_only_its_border() {
    let config = config_for(with(with_needle(), "needle_fill_opacity", serde_json::json!(0))).unwrap();
    let image = render(&config, 5500.0, 0.0, "gauge_needle_hollow.png");
    assert_eq!(image.get_pixel(200, 14).0[3], 0, "no body above the track");
    assert!(image.get_pixel(200, 10).0[3] > 80, "border remains at the tip");
}

fn gradient_needle(start_opacity: f64, end_color: &str, end_opacity: f64) -> serde_json::Value {
    let mut value = with_needle();
    value["needle_border_thickness"] = serde_json::json!(0);
    value["needle_fill_color"] = serde_json::json!("#ff0000");
    value["needle_fill_opacity"] = serde_json::json!(start_opacity);
    value["needle_fill_end_color"] = serde_json::json!(end_color);
    value["needle_fill_end_opacity"] = serde_json::json!(end_opacity);
    value
}

#[test]
fn needle_fill_fields_validate_strictly() {
    assert!(config_for(gradient_needle(1.0, "#0000ff", 0.0)).is_ok());

    let end_opacity_alone = without(gradient_needle(1.0, "#0000ff", 0.5), "needle_fill_end_color");
    assert!(config_for(end_opacity_alone).err().unwrap().to_string().contains("requires needle_fill_end_color"));
    let end_color_alone = without(gradient_needle(1.0, "#0000ff", 0.5), "needle_fill_end_opacity");
    assert!(config_for(end_color_alone).err().unwrap().to_string().contains("needle_fill_end_opacity"));
    let bad_color = gradient_needle(1.0, "blue", 0.5);
    assert!(config_for(bad_color).err().unwrap().to_string().contains("needle_fill_end_color"));
    let bad_fill = with(gradient_needle(1.0, "#0000ff", 0.5), "needle_fill_color", serde_json::json!("red"));
    assert!(config_for(bad_fill).err().unwrap().to_string().contains("needle_fill_color"));
    // Without a fill color the body keeps using the border color, as before the gradient existed.
    assert!(config_for(without(with_needle(), "needle_fill_color")).is_ok());
}

#[test]
fn the_needle_fill_runs_from_the_inner_to_the_outer_end() {
    // Needle at the top (5500 of 11000): it spans y = 10 (outer end) to 50 (inner end), centered at x = 200,
    // and the track covers y = 20..40, so the parts that stick out (10..20 and 40..50) show the fill alone.
    // Opaque red at the inner end fading to fully transparent blue at the outer end.
    let config = config_for(gradient_needle(1.0, "#0000ff", 0.0)).unwrap();
    let image = render(&config, 5500.0, 0.0, "gauge_needle_gradient.png");
    let inner = image.get_pixel(200, 47).0;
    let middle = image.get_pixel(200, 42).0;
    let outer = image.get_pixel(200, 14).0;
    assert!(inner[3] > 200 && inner[0] > 200 && inner[2] < 60, "inner end is opaque red, got {inner:?}");
    assert!((160..=235).contains(&middle[3]), "a bit further out it is more transparent, got {middle:?}");
    assert!(outer[3] < 70, "outer end is almost transparent, got {outer:?}");
    assert!(inner[3] > middle[3] && middle[3] > outer[3], "opacity falls toward the outer end");
}

#[test]
fn each_end_has_its_own_color_and_opacity() {
    // Red at 40% on the inner end, opaque blue on the outer end.
    let config = config_for(gradient_needle(0.4, "#0000ff", 1.0)).unwrap();
    let image = render(&config, 5500.0, 0.0, "gauge_needle_two_colors.png");
    let inner = image.get_pixel(200, 48).0;
    let outer = image.get_pixel(200, 12).0;
    assert!((70..=130).contains(&inner[3]) && inner[0] > inner[2], "inner end is a faint red, got {inner:?}");
    assert!(outer[3] > 200 && outer[2] > outer[0], "outer end is opaque blue, got {outer:?}");
}

#[test]
fn a_fully_transparent_gradient_draws_nothing_but_the_border() {
    let mut value = gradient_needle(0.0, "#0000ff", 0.0);
    value["needle_border_thickness"] = serde_json::json!(0);
    let image = render(&config_for(value).unwrap(), 5500.0, 0.0, "gauge_needle_invisible.png");
    assert_eq!(image.get_pixel(200, 15).0[3], 0);
    assert_eq!(image.get_pixel(200, 45).0[3], 0);
}

#[test]
fn a_solid_fill_can_have_its_own_color() {
    let mut value = with_needle();
    value["needle_fill_color"] = serde_json::json!("#00ff00");
    value["needle_fill_opacity"] = serde_json::json!(1.0);
    let image = render(&config_for(value).unwrap(), 5500.0, 0.0, "gauge_needle_fill_color.png");
    let body = image.get_pixel(200, 45).0;
    assert!(body[1] > 200 && body[0] < 60 && body[2] < 60, "body is green while the border stays magenta, got {body:?}");
}
