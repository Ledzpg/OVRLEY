//! Fuel level, battery voltage and estimated range: CSV import, display conversion and tank capacity contract.

mod common;

use ovrley_core::activity::csv::parse_csv_activity_reader;
use ovrley_core::normalize::validate_value_widget;
use ovrley_core::normalize::raw::ValueConfig;
use ovrley_core::render::format::{format_validated_metric_parts, MetricDisplayContent};
use std::io::Cursor;

fn parse(csv: &str) -> ovrley_core::activity::schema::ParsedActivity {
    parse_csv_activity_reader(Cursor::new(csv), "test.csv")
        .unwrap()
        .parsed_activity
}

fn value_config(overrides: serde_json::Value) -> ValueConfig {
    let mut value = common::builders::speed_value_json();
    for (key, item) in overrides.as_object().unwrap() {
        value[key] = item.clone();
    }
    serde_json::from_value(value).unwrap()
}

fn fuel_config(display_unit: &str, capacity: Option<f64>) -> serde_json::Value {
    let mut config = serde_json::json!({ "value": "fuel_level", "display_unit": display_unit, "decimals": 1 });
    if let Some(capacity) = capacity {
        config["fuel_tank_capacity"] = serde_json::json!(capacity);
    }
    config
}

fn shown(config: serde_json::Value, fuel: f64) -> (String, Option<String>) {
    let validated = validate_value_widget(value_config(config), 0).unwrap();
    let dense = common::builders::dense_report_with(|series| series.fuel_level = vec![Some(fuel)]);
    match format_validated_metric_parts(&validated, &dense, 0).unwrap().content {
        MetricDisplayContent::Standard { value_text, unit_text } => (value_text, unit_text),
        MetricDisplayContent::Coordinates(_) => panic!("fuel level is not a coordinate"),
    }
}

#[test]
fn csv_fuel_battery_and_range_columns_are_imported_in_canonical_units() {
    let csv = "Elapsed Time (s),Fuel Level (L),Battery Voltage (V),Estimated Range (km)\n\
0.0,19.5,14.1,380\n\
1.0,19.4,14.0,378.5\n";

    let activity = parse(csv);

    assert_eq!(activity.fuel_level, vec![Some(19.5), Some(19.4)]);
    assert_eq!(activity.battery_voltage, vec![Some(14.1), Some(14.0)]);
    assert_eq!(activity.estimated_range, vec![Some(380_000.0), Some(378_500.0)]);
}

#[test]
fn csv_range_in_miles_is_converted_to_metres() {
    let activity = parse("Elapsed Time (s),Estimated Range (mi)\n0.0,100\n1.0,99\n");

    assert_eq!(activity.estimated_range, vec![Some(160_934.4), Some(159_325.056)]);
}

#[test]
fn csv_garagem_style_headers_are_recognized() {
    let activity = parse("Elapsed Time (s),Fuel_L,Battery_V,Range_Km\n0.0,10,12.6,200\n1.0,9.9,12.5,199\n");

    assert_eq!(activity.fuel_level, vec![Some(10.0), Some(9.9)]);
    assert_eq!(activity.battery_voltage, vec![Some(12.6), Some(12.5)]);
    assert_eq!(activity.estimated_range, vec![Some(200_000.0), Some(199_000.0)]);
}

#[test]
fn csv_implausible_readings_are_discarded() {
    let csv = "Elapsed Time (s),Fuel Level (L),Battery Voltage (V),Estimated Range (km)\n\
0.0,-1,0,-5\n\
1.0,10,12.5,150\n\
2.0,900,99,9000\n";

    let activity = parse(csv);

    assert_eq!(activity.fuel_level, vec![None, Some(10.0), None]);
    // Zero volts is a sensor that is not answering, not a flat battery.
    assert_eq!(activity.battery_voltage, vec![None, Some(12.5), None]);
    assert_eq!(activity.estimated_range, vec![None, Some(150_000.0), None]);
}

#[test]
fn csv_fuel_level_declared_in_percent_is_rejected() {
    let result = parse_csv_activity_reader(Cursor::new("Elapsed Time (s),Fuel Level (%)\n0.0,80\n"), "percent.csv");

    assert!(result.is_err(), "a percentage cannot be converted without the tank capacity");
}

#[test]
fn fuel_level_shows_litres_or_a_percentage_of_the_tank() {
    assert_eq!(shown(fuel_config("l", Some(21.0)), 10.5), ("10.5".to_string(), Some("L".to_string())));
    assert_eq!(shown(fuel_config("percent", Some(21.0)), 10.5), ("50.0".to_string(), Some("%".to_string())));
    assert_eq!(shown(fuel_config("percent", Some(14.0)), 10.5).0, "75.0");
}

#[test]
fn estimated_range_shows_kilometres_or_miles() {
    let range = |unit: &str| {
        let validated = validate_value_widget(
            value_config(serde_json::json!({ "value": "estimated_range", "display_unit": unit, "decimals": 0 })),
            0,
        )
        .unwrap();
        let dense = common::builders::dense_report_with(|series| series.estimated_range = vec![Some(160_934.4)]);
        match format_validated_metric_parts(&validated, &dense, 0).unwrap().content {
            MetricDisplayContent::Standard { value_text, unit_text } => (value_text, unit_text),
            MetricDisplayContent::Coordinates(_) => unreachable!(),
        }
    };

    assert_eq!(range("km"), ("161".to_string(), Some("KM".to_string())));
    assert_eq!(range("mi"), ("100".to_string(), Some("MI".to_string())));
}

#[test]
fn battery_voltage_shows_volts() {
    let validated = validate_value_widget(
        value_config(serde_json::json!({ "value": "battery_voltage", "display_unit": "volt", "decimals": 1 })),
        0,
    )
    .unwrap();
    let dense = common::builders::dense_report_with(|series| series.battery_voltage = vec![Some(13.84)]);

    match format_validated_metric_parts(&validated, &dense, 0).unwrap().content {
        MetricDisplayContent::Standard { value_text, unit_text } => {
            assert_eq!(value_text, "13.8");
            assert_eq!(unit_text, Some("V".to_string()));
        }
        MetricDisplayContent::Coordinates(_) => unreachable!(),
    }
}

#[test]
fn fuel_level_requires_a_positive_tank_capacity_and_a_known_unit() {
    let error = |config: serde_json::Value| validate_value_widget(value_config(config), 0).unwrap_err().to_string();

    assert!(error(fuel_config("percent", None)).contains("fuel_tank_capacity: required"));
    assert!(error(fuel_config("percent", Some(0.0))).contains("fuel_tank_capacity"));
    assert!(error(fuel_config("percent", Some(-5.0))).contains("fuel_tank_capacity"));
    assert!(error(fuel_config("percent", Some(5000.0))).contains("fuel_tank_capacity"));
    assert!(error(fuel_config("gal", Some(21.0))).contains("expected 'l' or 'percent'"));
}

#[test]
fn tank_capacity_is_rejected_on_other_metrics() {
    let error = validate_value_widget(value_config(serde_json::json!({ "fuel_tank_capacity": 21 })), 0)
        .unwrap_err()
        .to_string();

    assert!(error.contains("fuel_tank_capacity: is only valid for fuel_level widgets"));
}
