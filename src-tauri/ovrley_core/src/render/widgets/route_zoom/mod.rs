//! Close-up route map that follows the current position.
//!
//! Like the zoomed map of a navigation app: raster tiles centered on the current position at a
//! fixed zoom, the route drawn over them (finished and remaining parts in their own style) and
//! an arrow marker at the center. The map can stay north-up or turn so the direction of travel
//! points up.
//!
//! Everything moves with the frame, so there is no static layer: preparation computes the route
//! in tile pixels, the position and heading of every frame, and (for a video render) downloads
//! every tile the frames will show; each frame then draws only the visible tiles and route part.
//!
//! Module ownership:
//! - `geometry` — projection to tile pixels, view scale, position/heading along the route.
//! - `tiles` — tile store and the tile set a frame (or a whole render) needs.
//! - `prepare` — cache construction and per-frame states.
//! - `draw` — per-frame drawing.

mod draw;
mod geometry;
mod prepare;
mod tiles;

pub(crate) use draw::{draw_route_zoom_widget, render_route_zoom_frame};
pub(crate) use prepare::{frame_state_at_progress, prepare_route_zoom_cache, prepare_route_zoom_preview, RouteZoomCache};
