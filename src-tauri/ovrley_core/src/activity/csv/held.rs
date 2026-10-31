//! Reconstruction of held sensor readings in oversampled CSV logs.
//!
//! Some loggers write a row every 100 ms but refresh each sensor at its own,
//! slower pace: the row simply repeats the last reading until a new one
//! arrives (GPS once per second, panel or OBD values a few times per second).
//! Interpolating such a file row by row draws a staircase: a flat run and then
//! a 100 ms ramp. Here every run of an identical value is read as one
//! reading taken at the run's first row, and the rows in between are placed on
//! the straight line to the next reading, which is what linear interpolation
//! between the real readings gives.
//!
//! Only dense files qualify (see [`MIN_ROWS_PER_SECOND`]), and a run is only
//! bridged when the next reading follows within [`MAX_READING_INTERVAL_SECONDS`];
//! a longer run is a real plateau (stopped vehicle, sensor stall) and stays flat.

/// Files written slower than this are sparse logs whose consecutive equal values are real readings.
const MIN_ROWS_PER_SECOND: f64 = 4.0;

/// Longest time between two readings that is still read as one refresh interval.
const MAX_READING_INTERVAL_SECONDS: f64 = 1.5;

/// Returns whether the timeline is dense enough to be a held-value log.
///
/// `elapsed_seconds` is the canonical, non-decreasing timeline of the rows.
pub(super) fn is_oversampled(elapsed_seconds: &[f64]) -> bool {
    let mut intervals = elapsed_seconds
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .filter(|interval| *interval > 0.0)
        .collect::<Vec<_>>();
    if intervals.is_empty() {
        return false;
    }
    intervals.sort_by(f64::total_cmp);
    let median = intervals[intervals.len() / 2];
    1.0 / median >= MIN_ROWS_PER_SECOND
}

/// Replaces the repeated rows of each short run with the line to the next reading.
///
/// A run ends at a changed value; a missing value or the end of the series
/// leaves the run as it is, because there is no next reading to head for.
/// `values` and `elapsed_seconds` are aligned row by row.
pub(super) fn interpolate_held_readings(values: &mut [Option<f64>], elapsed_seconds: &[f64]) {
    let readings = values.to_vec();
    let mut run_start = 0;
    while run_start < readings.len() {
        let Some(held) = readings[run_start] else {
            run_start += 1;
            continue;
        };
        let mut run_end = run_start + 1;
        while run_end < readings.len() && readings[run_end] == Some(held) {
            run_end += 1;
        }
        if let Some(next) = readings.get(run_end).copied().flatten() {
            let interval = elapsed_seconds[run_end] - elapsed_seconds[run_start];
            if interval > 0.0 && interval <= MAX_READING_INTERVAL_SECONDS {
                for row in run_start + 1..run_end {
                    let progress = (elapsed_seconds[row] - elapsed_seconds[run_start]) / interval;
                    values[row] = Some(held + (next - held) * progress);
                }
            }
        }
        run_start = run_end;
    }
}

#[cfg(test)]
mod tests {
    use super::{interpolate_held_readings, is_oversampled};

    fn tenth_second_timeline(rows: usize) -> Vec<f64> {
        (0..rows).map(|row| row as f64 * 0.1).collect()
    }

    #[test]
    fn rows_between_two_readings_lie_on_the_line_between_them() {
        let mut speed = vec![Some(10.0), Some(10.0), Some(10.0), Some(10.0), Some(14.0), Some(14.0)];

        interpolate_held_readings(&mut speed, &tenth_second_timeline(6));

        let expected = [10.0, 11.0, 12.0, 13.0, 14.0, 14.0];
        for (value, expected) in speed.iter().zip(expected) {
            assert!((value.unwrap() - expected).abs() < 1e-9, "{speed:?}");
        }
    }

    #[test]
    fn a_run_longer_than_the_refresh_interval_stays_flat() {
        let mut speed = vec![Some(0.0); 20];
        speed.push(Some(5.0));

        interpolate_held_readings(&mut speed, &tenth_second_timeline(21));

        assert!(speed[..20].iter().all(|value| *value == Some(0.0)));
    }

    #[test]
    fn a_missing_value_ends_a_run_without_bridging_it() {
        let mut rpm = vec![Some(3000.0), Some(3000.0), None, Some(3500.0)];

        interpolate_held_readings(&mut rpm, &tenth_second_timeline(4));

        assert_eq!(rpm, vec![Some(3000.0), Some(3000.0), None, Some(3500.0)]);
    }

    #[test]
    fn the_last_run_has_no_next_reading_and_stays_flat() {
        let mut speed = vec![Some(1.0), Some(2.0), Some(2.0), Some(2.0)];

        interpolate_held_readings(&mut speed, &tenth_second_timeline(4));

        assert_eq!(speed, vec![Some(1.0), Some(2.0), Some(2.0), Some(2.0)]);
    }

    #[test]
    fn consecutive_short_runs_each_head_for_their_own_next_reading() {
        let mut throttle = vec![Some(0.0), Some(0.0), Some(10.0), Some(10.0), Some(30.0)];

        interpolate_held_readings(&mut throttle, &tenth_second_timeline(5));

        let expected = [0.0, 5.0, 10.0, 20.0, 30.0];
        for (value, expected) in throttle.iter().zip(expected) {
            assert!((value.unwrap() - expected).abs() < 1e-9, "{throttle:?}");
        }
    }

    #[test]
    fn only_dense_timelines_are_treated_as_held_value_logs() {
        assert!(is_oversampled(&tenth_second_timeline(50)));
        let one_hertz = (0..50).map(f64::from).collect::<Vec<_>>();
        assert!(!is_oversampled(&one_hertz));
        assert!(!is_oversampled(&[0.0]));
    }
}
