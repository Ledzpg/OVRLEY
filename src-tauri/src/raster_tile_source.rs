//! HTTP raster map tiles with a persistent on-disk cache.
//!
//! Owns: downloading one tile URL, identifying the app to the tile server, and the cache files.
//! Does not own: tile coordinates, projection, or drawing (`ovrley_core::render::widgets::route`).
//!
//! Installed once at startup through `ovrley_core::raster_tiles::install_raster_tile_source`.
//! Tiles are reused from disk for `CACHE_TTL`; an expired tile that cannot be refreshed
//! (offline) is still served, so a rendered route keeps its map without a connection.

use ovrley_core::raster_tiles::RasterTileSource;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

const CACHE_TTL: Duration = Duration::from_secs(14 * 24 * 60 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_TILE_BYTES: usize = 4 * 1024 * 1024;
const MAX_FILE_NAME_CHARS: usize = 160;

pub(crate) struct HttpRasterTileSource {
    cache_root: PathBuf,
    client: Client,
}

impl HttpRasterTileSource {
    pub(crate) fn new(cache_root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&cache_root)
            .map_err(|error| format!("raster tile cache {}: {error}", cache_root.display()))?;
        // Public tile servers require an identifying User-Agent.
        let user_agent = format!(
            "OVRLEY-luizleite/{} (video overlay renderer; +https://www.luizleite.eng.br/garagem/ovrley)",
            env!("CARGO_PKG_VERSION")
        );
        let client = Client::builder()
            .user_agent(user_agent)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { cache_root, client })
    }

    /// Cache file for a URL: its readable tail, which always carries `z/x/y`.
    fn cache_path(&self, url: &str) -> PathBuf {
        let readable = url
            .trim_start_matches("https://")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
            .collect::<Vec<_>>();
        let tail = &readable[readable.len().saturating_sub(MAX_FILE_NAME_CHARS)..];
        self.cache_root.join(tail.iter().collect::<String>())
    }

    fn download(&self, url: &str) -> Result<Vec<u8>, String> {
        let response = self
            .client
            .get(url)
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|error| error.to_string())?;
        let is_image = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("image/"));
        if !is_image {
            return Err("the server did not answer with an image".to_string());
        }
        let bytes = response.bytes().map_err(|error| error.to_string())?;
        if bytes.len() > MAX_TILE_BYTES {
            return Err(format!("tile is larger than {MAX_TILE_BYTES} bytes"));
        }
        Ok(bytes.to_vec())
    }

    fn store(&self, path: &PathBuf, bytes: &[u8]) {
        // Several render threads can fetch tiles at once: write aside, then rename into place.
        let temporary = self.cache_root.join(format!(".{}.part", Uuid::new_v4()));
        if fs::write(&temporary, bytes).is_ok() && fs::rename(&temporary, path).is_err() {
            let _ = fs::remove_file(&temporary);
        }
    }
}

impl RasterTileSource for HttpRasterTileSource {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
        let path = self.cache_path(url);
        let cached = fs::metadata(&path).ok().and_then(|metadata| {
            let age = SystemTime::now().duration_since(metadata.modified().ok()?).ok()?;
            Some((age, fs::read(&path).ok()?))
        });
        if let Some((age, bytes)) = &cached {
            if *age < CACHE_TTL {
                return Ok(bytes.clone());
            }
        }
        match self.download(url) {
            Ok(bytes) => {
                self.store(&path, &bytes);
                Ok(bytes)
            }
            Err(error) => cached.map(|(_, bytes)| bytes).ok_or(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn serve(content_type: &'static str, hits: Arc<AtomicUsize>) -> (String, std::thread::JoinHandle<()>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let address = server.server_addr().to_ip().unwrap();
        let handle = std::thread::spawn(move || {
            if let Ok(request) = server.recv() {
                hits.fetch_add(1, Ordering::SeqCst);
                let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap();
                let _ = request.respond(tiny_http::Response::from_data(vec![1u8, 2, 3]).with_header(header));
            }
        });
        (format!("http://{address}/tiles/4/5/6.png"), handle)
    }

    fn source() -> (HttpRasterTileSource, PathBuf) {
        let root = std::env::temp_dir().join(format!("ovrley-raster-tiles-{}", Uuid::new_v4()));
        (HttpRasterTileSource::new(root.clone()).unwrap(), root)
    }

    #[test]
    fn a_downloaded_tile_is_served_from_disk_afterwards() {
        let hits = Arc::new(AtomicUsize::new(0));
        let (url, handle) = serve("image/png", Arc::clone(&hits));
        let (source, root) = source();
        assert_eq!(source.fetch(&url).unwrap(), vec![1, 2, 3]);
        handle.join().unwrap();
        // The server is gone; the second read must come from the cache.
        assert_eq!(source.fetch(&url).unwrap(), vec![1, 2, 3]);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_expired_tile_is_still_used_when_the_refresh_fails() {
        let (source, root) = source();
        let url = "http://127.0.0.1:9/tiles/1/2/3.png";
        let path = source.cache_path(url);
        fs::write(&path, [9u8, 9]).unwrap();
        let old = SystemTime::now() - CACHE_TTL - Duration::from_secs(60);
        fs::File::options().write(true).open(&path).unwrap().set_modified(old).unwrap();
        assert_eq!(source.fetch(url).unwrap(), vec![9, 9]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_non_image_answer_is_rejected_and_not_cached() {
        let hits = Arc::new(AtomicUsize::new(0));
        let (url, handle) = serve("text/html", hits);
        let (source, root) = source();
        let error = source.fetch(&url).unwrap_err();
        handle.join().unwrap();
        assert!(error.contains("did not answer with an image"), "{error}");
        assert!(!source.cache_path(&url).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cache_names_are_flat_and_keep_the_tile_coordinates() {
        let (source, root) = source();
        let path = source.cache_path("https://tile.example.org/a/b/12/345/678.png");
        assert_eq!(path.parent().unwrap(), root);
        assert!(path.file_name().unwrap().to_string_lossy().ends_with("12_345_678.png"));
        let _ = fs::remove_dir_all(root);
    }
}
