//! Held sensor readings in oversampled CSV logs are reconstructed as lines between the real readings.

use ovrley_core::activity::csv::parse_csv_activity_reader;
use ovrley_core::activity::schema::ParsedActivity;
use std::io::Cursor;

fn parse(csv: &str) -> ParsedActivity {
    parse_csv_activity_reader(Cursor::new(csv), "test.csv")
        .unwrap()
        .parsed_activity
}

fn assert_series(actual: &[Option<f64>], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?}");
    for (value, expected) in actual.iter().zip(expected) {
        assert!((value.expect("value present") - expected).abs() < 1e-6, "{actual:?}");
    }
}

#[test]
fn repeated_speed_and_rpm_rows_become_lines_between_readings() {
    let activity = parse(
        "Elapsed Time (s),Speed (m/s),RPM (rpm)\n\
0.0,10,3000\n0.1,10,3000\n0.2,10,3200\n0.3,10,3200\n0.4,14,3600\n0.5,14,3600\n0.6,14,3600\n0.7,14,3600\n0.8,18,3600\n0.9,18,3600\n",
    );

    assert_series(&activity.speed, &[10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 18.0]);
    assert_series(&activity.rpm, &[3000.0, 3100.0, 3200.0, 3400.0, 3600.0, 3600.0, 3600.0, 3600.0, 3600.0, 3600.0]);
}

#[test]
fn a_long_stop_stays_flat_instead_of_creeping_towards_the_next_reading() {
    let mut csv = String::from("Elapsed Time (s),Speed (m/s)\n");
    for row in 0..30 {
        csv.push_str(&format!("{:.1},0\n", row as f64 * 0.1));
    }
    csv.push_str("3.0,2\n3.1,2\n");

    let activity = parse(&csv);

    assert!(activity.speed[..30].iter().all(|value| *value == Some(0.0)));
}

#[test]
fn sparse_logs_keep_equal_consecutive_values_as_real_readings() {
    let activity = parse("Elapsed Time (s),Speed (m/s)\n0,10\n1,10\n2,14\n3,14\n");

    assert_series(&activity.speed, &[10.0, 10.0, 14.0, 14.0]);
}

#[test]
fn the_gear_is_not_interpolated() {
    let activity = parse(
        "Elapsed Time (s),Speed (m/s),Gear\n0.0,1,2\n0.1,1,2\n0.2,1,2\n0.3,1,3\n0.4,1,3\n0.5,1,3\n",
    );

    let gears = activity
        .gear_position
        .iter()
        .map(|gear| gear.clone().expect("gear present").to_string())
        .collect::<Vec<_>>();
    assert_eq!(gears, ["2", "2", "2", "3", "3", "3"]);
}
