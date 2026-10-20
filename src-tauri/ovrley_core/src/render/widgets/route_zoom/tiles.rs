//! Tiles of the close-up map: the store of decoded images and the tile sets frames need.

use super::geometry::{world_px, TILE_SIZE};
use super::super::route::map::load_tile_images;
use crate::error::{CoreError, CoreResult};
use skia_safe::Image;
use std::collections::{BTreeSet, HashMap};

/// Largest number of tiles one close-up map may download (public tile servers ask for moderate use).
pub(crate) const MAX_TILES: usize = 500;
/// Slack (in tile pixels) added when a tile set is planned from a coarse grid of centers.
const PLANNING_MARGIN_PX: f64 = TILE_SIZE / 2.0;

/// Decoded tiles of one zoom level.
#[derive(Clone, Debug)]
pub(crate) struct TileStore {
    template: String,
    zoom: u32,
    images: HashMap<(u32, u32), Image>,
}

impl TileStore {
    pub(crate) fn new(template: &str, zoom: u32) -> Self {
        Self { template: template.to_string(), zoom, images: HashMap::new() }
    }

    /// Downloads (or reads from the cache) every wanted tile that is not loaded yet.
    pub(crate) fn ensure(&mut self, wanted: &[(u32, u32)]) -> CoreResult<()> {
        let missing = wanted
            .iter()
            .copied()
            .filter(|tile| !self.images.contains_key(tile))
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return Ok(());
        }
        self.images.extend(load_tile_images(&self.template, self.zoom, &missing)?);
        Ok(())
    }

    pub(crate) fn get(&self, x: u32, y: u32) -> Option<&Image> {
        self.images.get(&(x, y))
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.images.len()
    }
}

/// Tiles that come within `radius` tile pixels of a center, clamped to the map.
pub(crate) fn tiles_around(center: (f64, f64), radius: f64, tile_zoom: u32) -> Vec<(u32, u32)> {
    let last = (world_px(tile_zoom) / TILE_SIZE).floor() - 1.0;
    let index = |value: f64| (value / TILE_SIZE).floor().clamp(0.0, last) as u32;
    let (x0, x1) = (index(center.0 - radius), index(center.0 + radius));
    let (y0, y1) = (index(center.1 - radius), index(center.1 + radius));
    (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).collect()
}

/// Plans every tile a series of view centers will show.
///
/// Centers are visited on a half-tile grid (the first center of each cell plans the cell, with
/// slack), so a long route costs one lookup per cell, not per frame.
pub(crate) fn plan_tiles(
    centers: impl Iterator<Item = (f64, f64)>,
    radius: f64,
    tile_zoom: u32,
) -> CoreResult<Vec<(u32, u32)>> {
    let cell = TILE_SIZE / 2.0;
    let mut planned_cells = BTreeSet::new();
    let mut tiles = BTreeSet::new();
    for center in centers {
        let key = ((center.0 / cell).floor() as i64, (center.1 / cell).floor() as i64);
        if !planned_cells.insert(key) {
            continue;
        }
        tiles.extend(tiles_around(center, radius + PLANNING_MARGIN_PX, tile_zoom));
        if tiles.len() > MAX_TILES {
            return Err(CoreError::Render(format!(
                "close-up map needs more than {MAX_TILES} tiles at zoom {tile_zoom}: lower the map zoom or detail, or export a shorter range"
            )));
        }
    }
    Ok(tiles.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_around_covers_the_view_and_stays_inside_the_map() {
        // Center of tile (5, 5) at zoom 4 with a radius smaller than half a tile: that tile only.
        assert_eq!(tiles_around((5.5 * 256.0, 5.5 * 256.0), 100.0, 4), vec![(5, 5)]);
        // Near a tile corner the four neighbours are needed.
        assert_eq!(tiles_around((6.0 * 256.0, 6.0 * 256.0), 10.0, 4).len(), 4);
        // At the edge of the world the range is clamped (zoom 2 has 4 x 4 tiles).
        let edge = tiles_around((5.0, 5.0), 400.0, 2);
        assert!(edge.iter().all(|&(x, y)| x <= 3 && y <= 3));
        assert!(edge.contains(&(0, 0)));
    }

    #[test]
    fn planning_visits_each_cell_once_and_includes_slack() {
        let centers = (0..1000).map(|step| (1000.0 + f64::from(step) * 0.01, 1000.0));
        let tiles = plan_tiles(centers, 100.0, 12).unwrap();
        assert!(tiles.contains(&(3, 3)) && tiles.contains(&(3, 4)), "{tiles:?}");
        assert!(tiles.len() <= 9);
    }

    #[test]
    fn a_route_that_needs_too_many_tiles_is_refused_with_advice() {
        let centers = (0..2000).map(|step| (f64::from(step) * 256.0, 4096.0));
        let error = plan_tiles(centers, 200.0, 14).unwrap_err().to_string();
        assert!(error.contains("lower the map zoom"), "{error}");
        assert!(error.contains(&MAX_TILES.to_string()), "{error}");
    }

    #[test]
    fn an_empty_store_has_no_tiles() {
        let mut store = TileStore::new("file:///nowhere/{z}/{x}/{y}.png", 3);
        assert_eq!(store.len(), 0);
        assert!(store.get(0, 0).is_none());
        store.ensure(&[]).unwrap();
    }
}
