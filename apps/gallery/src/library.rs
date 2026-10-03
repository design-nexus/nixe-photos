//! The photo library: what was found on disk, plus the user's favourites and
//! albums. Everything here is plain data and file I/O; the UI calls it directly.

use crate::models::{Album, PhotoItem, format_bytes};
use crate::{cmd, paths, scanner};
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Library {
    pub root: PathBuf,
    pub photos: Vec<PhotoItem>,
    pub albums: Vec<Album>,
    pub favorites: HashSet<String>,
    pub total_bytes: u64,
}

/// Where photos are looked for unless a file or folder is named on the command line.
pub fn default_root() -> PathBuf {
    dirs::picture_dir().unwrap_or_else(|| paths::home().join("Pictures"))
}

/// Scan `root` for photos and videos. Slow (reads metadata, makes thumbnails): run it
/// off the UI thread.
pub fn scan(root: &Path, depth: usize) -> Vec<PhotoItem> {
    scanner::scan_directory_depth(root, depth)
}

impl Library {
    pub fn load(root: PathBuf) -> Self {
        Library { root, albums: load_albums(), favorites: load_favorites(), ..Default::default() }
    }

    pub fn set_photos(&mut self, mut photos: Vec<PhotoItem>) {
        for p in &mut photos {
            p.is_favorite = self.favorites.contains(&p.id);
        }
        self.total_bytes = photos.iter().map(|p| p.file_size).sum();
        self.photos = photos;
    }

    /// Read `path` again after an edit wrote it, adding it to the library if it is new.
    /// Returns its id. Favourites and albums go by the path, so a replaced file keeps them.
    pub fn add_or_refresh(&mut self, path: &Path) -> Option<String> {
        let mut item = crate::metadata::extract_photo_item(path)?;
        item.is_favorite = self.favorites.contains(&item.id);
        let id = item.id.clone();
        match self.photos.iter().position(|p| p.id == id) {
            Some(i) => self.photos[i] = item,
            None => {
                self.photos.push(item);
                self.photos.sort_by(|a, b| b.date_taken.cmp(&a.date_taken));
            }
        }
        self.total_bytes = self.photos.iter().map(|p| p.file_size).sum();
        Some(id)
    }

    /// Forget photos whose files are gone: out of the library, favourites and albums.
    pub fn remove_photos(&mut self, ids: &[String]) -> Result<()> {
        self.photos.retain(|p| !ids.contains(&p.id));
        self.total_bytes = self.photos.iter().map(|p| p.file_size).sum();
        self.favorites.retain(|f| !ids.contains(f));
        for a in &mut self.albums {
            a.photo_ids.retain(|p| !ids.contains(p));
        }
        self.save_favorites()?;
        self.save_albums()
    }

    pub fn total_bytes_formatted(&self) -> String {
        format_bytes(self.total_bytes)
    }

    pub fn is_favorite(&self, id: &str) -> bool {
        self.favorites.contains(id)
    }

    /// Flip a favourite and save. Returns the new state.
    pub fn toggle_favorite(&mut self, id: &str) -> Result<bool> {
        let now = if self.favorites.remove(id) {
            false
        } else {
            self.favorites.insert(id.to_string());
            true
        };
        if let Some(p) = self.photos.iter_mut().find(|p| p.id == id) {
            p.is_favorite = now;
        }
        self.save_favorites()?;
        Ok(now)
    }

    fn save_favorites(&self) -> Result<()> {
        let mut ids: Vec<&String> = self.favorites.iter().collect();
        ids.sort();
        cmd::atomic_write(&paths::favorites_file(), &serde_json::to_string_pretty(&ids)?)
    }

    fn save_albums(&self) -> Result<()> {
        cmd::atomic_write(&paths::albums_file(), &serde_json::to_string_pretty(&self.albums)?)
    }

    pub fn album(&self, id: &str) -> Option<&Album> {
        self.albums.iter().find(|a| a.id == id)
    }

    pub fn create_album(&mut self, name: &str) -> Result<String> {
        let id = next_album_id(&self.albums);
        self.albums.push(Album { id: id.clone(), name: name.to_string(), photo_ids: Vec::new(), created_at: today() });
        self.save_albums()?;
        Ok(id)
    }

    pub fn rename_album(&mut self, id: &str, name: &str) -> Result<()> {
        if let Some(a) = self.albums.iter_mut().find(|a| a.id == id) {
            a.name = name.to_string();
        }
        self.save_albums()
    }

    pub fn delete_album(&mut self, id: &str) -> Result<()> {
        self.albums.retain(|a| a.id != id);
        self.save_albums()
    }

    /// Returns how many photos were new to the album.
    pub fn add_to_album(&mut self, id: &str, photo_ids: &[String]) -> Result<usize> {
        let mut added = 0;
        if let Some(a) = self.albums.iter_mut().find(|a| a.id == id) {
            for pid in photo_ids {
                if !a.photo_ids.contains(pid) {
                    a.photo_ids.push(pid.clone());
                    added += 1;
                }
            }
        }
        self.save_albums()?;
        Ok(added)
    }

    pub fn remove_from_album(&mut self, id: &str, photo_id: &str) -> Result<()> {
        if let Some(a) = self.albums.iter_mut().find(|a| a.id == id) {
            a.photo_ids.retain(|p| p != photo_id);
        }
        self.save_albums()
    }
}

/// `album_N`, one past the highest number in use, so a deleted album's id isn't reused.
fn next_album_id(albums: &[Album]) -> String {
    let highest =
        albums.iter().filter_map(|a| a.id.strip_prefix("album_").and_then(|n| n.parse::<u64>().ok())).max().unwrap_or(0);
    format!("album_{}", highest + 1)
}

fn load_albums() -> Vec<Album> {
    std::fs::read_to_string(paths::albums_file()).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_else(|| {
        vec![Album {
            id: "highlight_album".into(),
            name: "Favorites & highlights".into(),
            photo_ids: Vec::new(),
            created_at: today(),
        }]
    })
}

fn load_favorites() -> HashSet<String> {
    std::fs::read_to_string(paths::favorites_file())
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .map(|v| v.into_iter().collect())
        .unwrap_or_default()
}

fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, m, d) = crate::metadata::days_to_ymd(secs / 86400);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Show the folder holding `target` in the default file manager.
pub fn reveal(target: &str) {
    let path = Path::new(target);
    let dir = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
    cmd::spawn(&["xdg-open", &dir.to_string_lossy()]);
}

/// The first file named on the command line (a path or a `file://` URI), made absolute.
pub fn file_argument(args: &[String]) -> Option<PathBuf> {
    let arg = args.iter().find(|a| !a.starts_with('-'))?;
    let path = PathBuf::from(arg.strip_prefix("file://").map(percent_decode).unwrap_or_else(|| arg.clone()));
    path.is_file().then(|| std::fs::canonicalize(&path).unwrap_or(path))
}

/// Decode `%XX` escapes in a `file://` URI path.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let decoded = (bytes[i] == b'%')
            .then(|| bytes.get(i + 1..i + 3))
            .flatten()
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        if let Some(b) = decoded {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_edit_is_added_once_and_refreshed_after() {
        let dir = std::env::temp_dir().join(format!("nexus-photos-lib-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.png");
        let png = |w: u32| {
            let img = image::RgbaImage::from_pixel(w, 10, image::Rgba([1, 2, 3, 255]));
            crate::edit::pipeline::encode(&img, crate::edit::pipeline::Kind::Png, 90).unwrap()
        };
        std::fs::write(&path, png(20)).unwrap();
        let mut lib = Library::default();
        let id = lib.add_or_refresh(&path).unwrap();
        assert_eq!(lib.photos.len(), 1);
        std::fs::write(&path, png(30)).unwrap();
        assert_eq!(lib.add_or_refresh(&path).as_deref(), Some(id.as_str()));
        assert_eq!(lib.photos.len(), 1);
        assert_eq!(lib.photos[0].width, 30);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn decodes_uris() {
        assert_eq!(percent_decode("/a%20b/c%2Fd"), "/a b/c/d");
        assert_eq!(percent_decode("/100%"), "/100%");
    }

    #[test]
    fn album_ids_do_not_collide_after_delete() {
        let album = |id: &str| Album { id: id.into(), name: id.into(), photo_ids: vec![], created_at: String::new() };
        assert_eq!(next_album_id(&[]), "album_1");
        assert_eq!(next_album_id(&[album("highlight_album"), album("album_1"), album("album_3")]), "album_4");
    }
}
