//! Raster map tile sources.
//!
//! Owns: the `RasterTileSource` boundary trait, the process-wide source registry, `file://` tile
//! packs, and tile URL template expansion.
//! Does not own: HTTP transport or disk caching (the application shell installs a source that does
//! both), projection, or drawing (see `render::widgets::route::map`).
//!
//! A tile URL template is `https://host/path/{z}/{x}/{y}.png` or `file:///folder/{z}/{x}/{y}.png`
//! for an offline tile pack. `file://` templates are read directly; every other URL goes through
//! the installed source and fails loudly when none is installed.

use crate::error::{CoreError, CoreResult};
use std::sync::{Arc, RwLock};

/// External-system boundary that downloads (and typically caches) one tile.
pub trait RasterTileSource: Send + Sync {
    /// Returns the encoded image bytes (PNG or JPEG) for a fully expanded tile URL.
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String>;
}

static SOURCE: RwLock<Option<Arc<dyn RasterTileSource>>> = RwLock::new(None);

/// Installs the source used for every non-`file://` tile URL, replacing any previous one.
pub fn install_raster_tile_source(source: Arc<dyn RasterTileSource>) {
    *SOURCE.write().expect("raster tile source lock poisoned") = Some(source);
}

/// Reports whether a tile URL template carries the three coordinate placeholders.
pub fn is_tile_template(template: &str) -> bool {
    ["{z}", "{x}", "{y}"]
        .iter()
        .all(|placeholder| template.contains(placeholder))
}

/// Expands a tile URL template for one tile.
pub fn expand_tile_url(template: &str, zoom: u32, x: u32, y: u32) -> String {
    template
        .replace("{z}", &zoom.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string())
}

/// Fetches the encoded bytes of one expanded tile URL.
pub fn fetch_tile(url: &str) -> CoreResult<Vec<u8>> {
    if let Some(path) = url.strip_prefix("file://") {
        let path = file_url_path(path);
        return std::fs::read(&path).map_err(|source| CoreError::Io { path, source });
    }
    let source = SOURCE
        .read()
        .expect("raster tile source lock poisoned")
        .clone()
        .ok_or_else(|| {
            CoreError::Render(format!(
                "map tile {url}: no raster tile source is installed (only file:// tile packs work)"
            ))
        })?;
    source
        .fetch(url)
        .map_err(|message| CoreError::Render(format!("map tile {url}: {message}")))
}

/// Converts the part of a `file://` URL after the scheme into a filesystem path
/// (`/C:/tiles/1.png` becomes `C:/tiles/1.png` on Windows-style drive paths).
fn file_url_path(path: &str) -> std::path::PathBuf {
    let bytes = path.as_bytes();
    let drive_path = bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':';
    std::path::PathBuf::from(if drive_path { &path[1..] } else { path })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_need_all_three_placeholders() {
        assert!(is_tile_template("https://a.example/{z}/{x}/{y}.png"));
        assert!(!is_tile_template("https://a.example/{z}/{x}.png"));
    }

    #[test]
    fn expands_every_placeholder() {
        assert_eq!(expand_tile_url("u/{z}/{x}/{y}.png?z={z}", 12, 3, 4), "u/12/3/4.png?z=12");
    }

    #[test]
    fn file_urls_drop_the_slash_before_a_drive_letter() {
        assert_eq!(file_url_path("/C:/tiles/1.png"), std::path::PathBuf::from("C:/tiles/1.png"));
        assert_eq!(file_url_path("/home/tiles/1.png"), std::path::PathBuf::from("/home/tiles/1.png"));
    }
}
