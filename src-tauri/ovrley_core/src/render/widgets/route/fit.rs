/// Uniform fit of projected route points into the widget box.
///
/// One transform is shared by the route geometry and the raster map behind it so the
/// route line sits exactly on the map. Projected points have y pointing up; fitted
/// widget points have y pointing down.

/// Scale and offsets that center projected points inside the widget with an inset.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RouteFit {
    pub(crate) min_x: f64,
    pub(crate) min_y: f64,
    pub(crate) scale: f64,
    pub(crate) offset_x: f64,
    pub(crate) offset_y: f64,
    pub(crate) height: f64,
}

impl RouteFit {
    /// Fits the bounding box of `points` into `width` x `height`, leaving `inset_px` free.
    /// Returns `None` for an empty point list.
    pub(crate) fn new(points: &[(f64, f64)], width: f64, height: f64, inset_px: f64) -> Option<Self> {
        if points.is_empty() {
            return None;
        }
        let min_x = points.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
        let max_x = points
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = points.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
        let max_y = points
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::NEG_INFINITY, f64::max);
        let safe_inset = inset_px.max(0.0).min(width.min(height) * 0.45);
        let inner_width = (width - safe_inset * 2.0).max(1.0);
        let inner_height = (height - safe_inset * 2.0).max(1.0);
        let span_x = (max_x - min_x).max(1e-6);
        let span_y = (max_y - min_y).max(1e-6);
        let scale = (inner_width / span_x).min(inner_height / span_y);
        Some(Self {
            min_x,
            min_y,
            scale,
            offset_x: (width - span_x * scale) / 2.0,
            offset_y: (height - span_y * scale) / 2.0,
            height,
        })
    }

    /// Maps a projected point to widget space (y down).
    pub(crate) fn apply(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (
            (x - self.min_x) * self.scale + self.offset_x,
            self.height - ((y - self.min_y) * self.scale + self.offset_y),
        )
    }

    /// Maps a widget-space point back to the projected plane.
    pub(crate) fn unapply(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (
            self.min_x + (x - self.offset_x) / self.scale,
            self.min_y + ((self.height - y) - self.offset_y) / self.scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_and_unapply_round_trip() {
        let points = [(10.0, 20.0), (14.0, 22.0)];
        let fit = RouteFit::new(&points, 200.0, 100.0, 5.0).unwrap();
        for point in points {
            let back = fit.unapply(fit.apply(point));
            assert!((back.0 - point.0).abs() < 1e-9 && (back.1 - point.1).abs() < 1e-9);
        }
    }

    #[test]
    fn higher_projected_y_is_higher_on_screen() {
        let fit = RouteFit::new(&[(0.0, 0.0), (1.0, 1.0)], 100.0, 100.0, 0.0).unwrap();
        assert!(fit.apply((0.5, 1.0)).1 < fit.apply((0.5, 0.0)).1);
    }

    #[test]
    fn empty_points_have_no_fit() {
        assert!(RouteFit::new(&[], 10.0, 10.0, 0.0).is_none());
    }
}
