//! Pure rectangle and cap geometry for linear gauges.
//!
//! Orientation is handled here once so the layer renderer can operate on
//! concrete rectangles and a shared tangent/normal frame. Horizontal gauges
//! fill left-to-right; vertical gauges fill bottom-to-top.

use super::super::track_path::TrackPathFrame;
use crate::normalize::ValidatedLinearGaugeOrientation;
use skia_safe::{Point, Rect};

/// Computes the filled-bar rect without border insetting.
pub fn bar_fill_rect(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    fill01: f32,
    orientation: ValidatedLinearGaugeOrientation,
) -> (f32, f32, f32, f32) {
    let fill01 = fill01.clamp(0.0, 1.0);
    match orientation {
        ValidatedLinearGaugeOrientation::Horizontal => (x, y, width * fill01, height),
        ValidatedLinearGaugeOrientation::Vertical => {
            let filled_height = height * fill01;
            (x, y + height - filled_height, width, filled_height)
        }
    }
}

/// Computes the filled-bar rect, inset by the border thickness.
pub fn bordered_bar_fill_rect(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    fill01: f32,
    orientation: ValidatedLinearGaugeOrientation,
    border_thickness: f32,
) -> (f32, f32, f32, f32) {
    let inner_width = (width - border_thickness * 2.0).max(0.0);
    let inner_height = (height - border_thickness * 2.0).max(0.0);
    bar_fill_rect(
        x + border_thickness,
        y + border_thickness,
        inner_width,
        inner_height,
        fill01,
        orientation,
    )
}

/// Splits a bordered track at the redline into `(normal, redline)` clip rects.
///
/// The boundary sits where a fill of `fraction` ends, so a fill drawn up to the
/// redline meets the redline zone exactly. Both rects extend far past the track
/// so they only constrain the axis the gauge fills along.
pub fn redline_zone_rects(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    fraction: f32,
    orientation: ValidatedLinearGaugeOrientation,
    border_thickness: f32,
) -> (Rect, Rect) {
    const REACH: f32 = 100_000.0;
    let (fill_x, fill_y, fill_width, _) = bordered_bar_fill_rect(
        x,
        y,
        width,
        height,
        fraction,
        orientation,
        border_thickness,
    );
    match orientation {
        ValidatedLinearGaugeOrientation::Horizontal => {
            let boundary = fill_x + fill_width;
            (
                Rect::from_ltrb(-REACH, -REACH, boundary, REACH),
                Rect::from_ltrb(boundary, -REACH, REACH, REACH),
            )
        }
        ValidatedLinearGaugeOrientation::Vertical => (
            Rect::from_ltrb(-REACH, fill_y, REACH, REACH),
            Rect::from_ltrb(-REACH, -REACH, REACH, fill_y),
        ),
    }
}

pub(super) fn segment_rect(
    index: u32,
    extent: f32,
    gap: f32,
    width: f32,
    height: f32,
    orientation: ValidatedLinearGaugeOrientation,
) -> Rect {
    match orientation {
        ValidatedLinearGaugeOrientation::Horizontal => {
            Rect::from_xywh(index as f32 * (extent + gap), 0.0, extent, height)
        }
        ValidatedLinearGaugeOrientation::Vertical => Rect::from_xywh(
            0.0,
            height - extent - index as f32 * (extent + gap),
            width,
            extent,
        ),
    }
}

/// Returns the local frame and thickness used by the translated low-fill cap.
pub(super) fn track_cap_frame(
    track: Rect,
    corner_radius: f32,
    orientation: ValidatedLinearGaugeOrientation,
) -> (TrackPathFrame, f32) {
    match orientation {
        ValidatedLinearGaugeOrientation::Horizontal => (
            TrackPathFrame {
                origin: Point::new(track.left + corner_radius, track.top + track.height() * 0.5),
                tangent: Point::new(1.0, 0.0),
                normal: Point::new(0.0, 1.0),
            },
            track.height(),
        ),
        ValidatedLinearGaugeOrientation::Vertical => (
            TrackPathFrame {
                origin: Point::new(
                    track.left + track.width() * 0.5,
                    track.bottom - corner_radius,
                ),
                tangent: Point::new(0.0, -1.0),
                normal: Point::new(1.0, 0.0),
            },
            track.width(),
        ),
    }
}
