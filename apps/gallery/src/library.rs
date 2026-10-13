//! The photo library: what was found on disk, plus the user's favourites, albums and
//! view rotations. Everything here is plain data and file I/O; the UI calls it directly.

use crate::models::{Album, PhotoItem, fnv1a, format_bytes, legacy_photo_id, photo_id};
use crate::{cmd, paths, scanner};
use anyhow::{Context, Result, bail};
use gtk::gio;
use gtk::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Library {
    /// The folders photos are looked for in.
    pub roots: Vec<PathBuf>,
    pub photos: Vec<PhotoItem>,
    pub albums: Vec<Album>,
    pub favorites: HashSet<String>,
    /// Quarter turns the viewer shows a photo with, without changing the file.
    pub rotations: HashMap<String, u32>,
    pub total_bytes: u64,
    /// Pixel sizes of the connected screens: a PNG exactly that size is a screenshot.
    pub screens: Vec<(u32, u32)>,
}

/// When the favourites, albums and rotations files last changed, to notice another
/// window's changes.
pub fn saved_stamp() -> Vec<Option<std::time::SystemTime>> {
    [paths::favorites_file(), paths::albums_file(), paths::rotations_file()]
        .iter()
        .map(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok())
        .collect()
}

/// Where photos are looked for when no folder has been chosen.
pub fn default_root() -> PathBuf {
    dirs::picture_dir().unwrap_or_else(|| paths::home().join("Pictures"))
}

/// The library folders from the settings, or the Pictures folder.
pub fn configured_roots(folders: &[String]) -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = folders.iter().map(|f| paths::expand(f)).collect();
    if roots.is_empty() { vec![default_root()] } else { roots }
}

/// Scan `roots` for photos and videos, reusing what the last scan read for files that
/// haven't changed. Slow the first time: run it off the UI thread.
pub fn scan(roots: &[PathBuf], depth: usize, known: &HashMap<String, PhotoItem>) -> scanner::Scan {
    scanner::scan(roots, depth, known)
}

/// What the last scan found, by path. Read at start-up so the grid fills at once.
pub fn load_cache() -> HashMap<String, PhotoItem> {
    std::fs::read(paths::library_cache())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<PhotoItem>>(&bytes).ok())
        .map(|items| items.into_iter().map(|p| (p.path.clone(), p)).collect())
        .unwrap_or_default()
}

pub fn save_cache(items: &[PhotoItem]) {
    if let Ok(text) = serde_json::to_vec(items) {
        let _ = cmd::atomic_write(&paths::library_cache(), text);
    }
}

/// What a photo belonged to, so a deletion can be undone.
#[derive(Default, Clone)]
pub struct Membership {
    pub favorites: Vec<String>,
    pub albums: Vec<(String, String)>,
    pub rotations: Vec<(String, u32)>,
}

impl Library {
    pub fn load(roots: Vec<PathBuf>) -> Self {
        Library {
            roots,
            albums: load_albums(),
            favorites: load_favorites(),
            rotations: load_json(&paths::rotations_file()).unwrap_or_default(),
            ..Default::default()
        }
    }

    /// Read favourites, albums and rotations from disk again (another window, such as the
    /// stand-alone viewer, may have changed them).
    pub fn reload_saved(&mut self) {
        self.albums = load_albums();
        self.favorites = load_favorites();
        self.rotations = load_json(&paths::rotations_file()).unwrap_or_default();
        for p in &mut self.photos {
            p.is_favorite = self.favorites.contains(&p.id);
        }
    }

    pub fn set_photos(&mut self, mut photos: Vec<PhotoItem>) {
        if self.migrate_ids(&photos) {
            let _ = self.save_favorites();
            let _ = self.save_albums();
            let _ = self.save_rotations();
        }
        for p in &mut photos {
            p.is_favorite = self.favorites.contains(&p.id);
            self.classify(p);
        }
        self.total_bytes = photos.iter().map(|p| p.file_size).sum();
        self.photos = photos;
    }

    /// A PNG with no camera, exactly the size of a screen, is a screenshot.
    fn classify(&self, p: &mut PhotoItem) {
        if p.media_type == "photos"
            && p.format == "PNG"
            && p.camera_model.is_none()
            && self.screens.iter().any(|&(w, h)| (p.width, p.height) == (w, h) || (p.width, p.height) == (h, w))
        {
            p.media_type = "screenshots".into();
        }
    }

    /// Older versions made ids with a hasher that may change between Rust releases. Move
    /// favourites, albums and rotations saved under those ids to the stable ones. Returns
    /// whether anything moved (and so needs saving).
    fn migrate_ids(&mut self, photos: &[PhotoItem]) -> bool {
        let known: HashSet<&str> = photos.iter().map(|p| p.id.as_str()).collect();
        let stale = |id: &String| !known.contains(id.as_str());
        let any_stale = self.favorites.iter().any(stale)
            || self.albums.iter().any(|a| a.photo_ids.iter().any(stale))
            || self.rotations.keys().any(stale);
        if !any_stale {
            return false;
        }
        let old_to_new: HashMap<String, String> = photos.iter().map(|p| (legacy_photo_id(&p.path), p.id.clone())).collect();
        let mut changed = false;
        let mut swap = |id: &mut String| {
            if let Some(new) = old_to_new.get(id.as_str()) {
                *id = new.clone();
                changed = true;
            }
        };
        let mut favs: Vec<String> = self.favorites.drain().collect();
        favs.iter_mut().for_each(&mut swap);
        for a in &mut self.albums {
            a.photo_ids.iter_mut().for_each(&mut swap);
        }
        let rotations: Vec<(String, u32)> = self.rotations.drain().collect();
        let mut moved = HashMap::new();
        for (mut id, deg) in rotations {
            swap(&mut id);
            moved.insert(id, deg);
        }
        self.favorites = favs.into_iter().collect();
        self.rotations = moved;
        changed
    }

    /// Read `path` again after an edit wrote it, adding it to the library if it is new.
    /// Returns its id. Favourites and albums go by the path, so a replaced file keeps them.
    pub fn add_or_refresh(&mut self, path: &Path) -> Option<String> {
        let mut item = crate::metadata::extract_photo_item(path)?;
        item.is_favorite = self.favorites.contains(&item.id);
        self.classify(&mut item);
        let id = item.id.clone();
        match self.photos.iter().position(|p| p.id == id) {
            Some(i) => self.photos[i] = item,
            None => {
                self.photos.push(item);
                scanner::sort_newest_first(&mut self.photos);
            }
        }
        self.total_bytes = self.photos.iter().map(|p| p.file_size).sum();
        Some(id)
    }

    /// The favourites, albums and rotations `ids` are part of.
    pub fn membership(&self, ids: &[String]) -> Membership {
        Membership {
            favorites: ids.iter().filter(|id| self.favorites.contains(*id)).cloned().collect(),
            albums: self
                .albums
                .iter()
                .flat_map(|a| a.photo_ids.iter().filter(|p| ids.contains(p)).map(|p| (a.id.clone(), p.clone())))
                .collect(),
            rotations: ids.iter().filter_map(|id| self.rotations.get(id).map(|r| (id.clone(), *r))).collect(),
        }
    }

    /// Put back what `membership` recorded (after an undone deletion).
    pub fn restore(&mut self, m: &Membership) -> Result<()> {
        self.favorites.extend(m.favorites.iter().cloned());
        for p in &mut self.photos {
            p.is_favorite = self.favorites.contains(&p.id);
        }
        for (album, id) in &m.albums {
            if let Some(a) = self.albums.iter_mut().find(|a| &a.id == album)
                && !a.photo_ids.contains(id)
            {
                a.photo_ids.push(id.clone());
            }
        }
        self.rotations.extend(m.rotations.iter().cloned());
        self.save_favorites()?;
        self.save_rotations()?;
        self.save_albums()
    }

    /// Forget photos whose files are gone: out of the library, favourites and albums.
    pub fn remove_photos(&mut self, ids: &[String]) -> Result<()> {
        self.photos.retain(|p| !ids.contains(&p.id));
        self.total_bytes = self.photos.iter().map(|p| p.file_size).sum();
        self.favorites.retain(|f| !ids.contains(f));
        self.rotations.retain(|f, _| !ids.contains(f));
        for a in &mut self.albums {
            a.photo_ids.retain(|p| !ids.contains(p));
        }
        self.save_favorites()?;
        self.save_rotations()?;
        self.save_albums()
    }

    /// Rename a photo's file in its folder, keeping its favourite, albums and rotation.
    /// Returns the new id.
    pub fn rename(&mut self, id: &str, new_name: &str) -> Result<String> {
        let name = new_name.trim();
        if name.is_empty() || name.contains('/') || name == "." || name == ".." {
            bail!("“{name}” isn't a file name");
        }
        let i = self.photos.iter().position(|p| p.id == id).context("the photo isn't in the library")?;
        let old = PathBuf::from(&self.photos[i].path);
        let new = old.with_file_name(name);
        if new == old {
            return Ok(id.to_string());
        }
        if new.exists() {
            bail!("there is already a file called “{name}”");
        }
        std::fs::rename(&old, &new).with_context(|| format!("could not rename {}", old.display()))?;
        let new_text = new.to_string_lossy().to_string();
        let new_id = photo_id(&new_text);
        let p = &mut self.photos[i];
        let new_thumb = crate::thumbnails::cache_path(&new, p.mtime);
        let _ = std::fs::rename(&p.thumbnail_path, &new_thumb);
        p.thumbnail_path = new_thumb.to_string_lossy().to_string();
        p.path = new_text;
        p.file_name = name.to_string();
        p.id = new_id.clone();
        if self.favorites.remove(id) {
            self.favorites.insert(new_id.clone());
        }
        if let Some(r) = self.rotations.remove(id) {
            self.rotations.insert(new_id.clone(), r);
        }
        for a in &mut self.albums {
            for p in a.photo_ids.iter_mut().filter(|p| *p == id) {
                *p = new_id.clone();
            }
        }
        self.save_favorites()?;
        self.save_rotations()?;
        self.save_albums()?;
        Ok(new_id)
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

    pub fn rotation(&self, id: &str) -> u32 {
        self.rotations.get(id).copied().unwrap_or(0)
    }

    /// Turn the view of a photo a quarter clockwise and remember it. Returns the new angle.
    pub fn rotate(&mut self, id: &str) -> u32 {
        let angle = (self.rotation(id) + 90) % 360;
        self.set_rotation(id, angle);
        angle
    }

    pub fn set_rotation(&mut self, id: &str, angle: u32) {
        if angle == 0 {
            self.rotations.remove(id);
        } else {
            self.rotations.insert(id.to_string(), angle);
        }
        let _ = self.save_rotations();
    }

    fn save_favorites(&self) -> Result<()> {
        let mut ids: Vec<&String> = self.favorites.iter().collect();
        ids.sort();
        cmd::atomic_write(&paths::favorites_file(), &serde_json::to_string_pretty(&ids)?)
    }

    fn save_rotations(&self) -> Result<()> {
        cmd::atomic_write(&paths::rotations_file(), &serde_json::to_string_pretty(&self.rotations)?)
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

    /// Delete an album (not its photos). Returns it and where it was, so it can be put back.
    pub fn delete_album(&mut self, id: &str) -> Result<Option<(Album, usize)>> {
        let Some(at) = self.albums.iter().position(|a| a.id == id) else { return Ok(None) };
        let album = self.albums.remove(at);
        self.save_albums()?;
        Ok(Some((album, at)))
    }

    /// Put a deleted album back where it was.
    pub fn restore_album(&mut self, album: Album, at: usize) -> Result<()> {
        if self.album(&album.id).is_none() {
            self.albums.insert(at.min(self.albums.len()), album);
        }
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

/// Sets of files with the same contents, biggest files first. Reads every file that
/// shares its size with another: run it off the UI thread.
pub fn find_duplicates(files: Vec<(String, String, u64)>) -> Vec<Vec<String>> {
    use rayon::prelude::*;
    let mut by_size: HashMap<u64, Vec<(String, String)>> = HashMap::new();
    for (id, path, size) in files {
        if size > 0 {
            by_size.entry(size).or_default().push((id, path));
        }
    }
    let mut sizes: Vec<(u64, Vec<(String, String)>)> = by_size.into_iter().filter(|(_, v)| v.len() > 1).collect();
    sizes.sort_by_key(|s| std::cmp::Reverse(s.0));
    sizes
        .into_par_iter()
        .flat_map(|(_, same_size)| {
            let mut by_hash: HashMap<u64, Vec<String>> = HashMap::new();
            for (id, path) in same_size {
                if let Some(h) = hash_file(Path::new(&path)) {
                    by_hash.entry(h).or_default().push(id);
                }
            }
            by_hash.into_values().filter(|v| v.len() > 1).collect::<Vec<_>>()
        })
        .collect()
}

fn hash_file(path: &Path) -> Option<u64> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; 1 << 20];
    let mut h: u64 = 0;
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            return Some(h);
        }
        h = fnv1a(&[&h.to_le_bytes()[..], &buf[..n]].concat());
    }
}

/// Move files back out of the trash to where they were. Returns the paths put back.
/// Blocking: run it off the UI thread.
pub fn untrash(paths: &[String]) -> Vec<String> {
    let trash = gio::File::for_uri("trash:///");
    let Ok(children) = trash.enumerate_children(
        "standard::name,trash::orig-path,trash::deletion-date",
        gio::FileQueryInfoFlags::NONE,
        gio::Cancellable::NONE,
    ) else {
        return Vec::new();
    };
    // The newest trashed copy of each path.
    let mut best: HashMap<String, (String, String)> = HashMap::new();
    for info in children.flatten() {
        let Some(orig) = info.attribute_byte_string("trash::orig-path").map(|s| s.to_string()) else { continue };
        if !paths.contains(&orig) {
            continue;
        }
        let date = info.attribute_string("trash::deletion-date").map(|s| s.to_string()).unwrap_or_default();
        let name = info.name().to_string_lossy().to_string();
        if best.get(&orig).is_none_or(|(d, _)| date > *d) {
            best.insert(orig, (date, name));
        }
    }
    let mut restored = Vec::new();
    for (orig, (_, name)) in best {
        let item = trash.child(&name);
        if Path::new(&orig).exists() {
            continue;
        }
        if item.move_(&gio::File::for_path(&orig), gio::FileCopyFlags::NONE, gio::Cancellable::NONE, None).is_ok() {
            restored.push(orig);
        }
    }
    restored
}

/// `album_N`, one past the highest number in use, so a deleted album's id isn't reused.
fn next_album_id(albums: &[Album]) -> String {
    let highest =
        albums.iter().filter_map(|a| a.id.strip_prefix("album_").and_then(|n| n.parse::<u64>().ok())).max().unwrap_or(0);
    format!("album_{}", highest + 1)
}

fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    std::fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok())
}

fn load_albums() -> Vec<Album> {
    load_json(&paths::albums_file()).unwrap_or_else(|| {
        vec![Album {
            id: "highlight_album".into(),
            name: "Favorites & highlights".into(),
            photo_ids: Vec::new(),
            created_at: today(),
        }]
    })
}

fn load_favorites() -> HashSet<String> {
    load_json::<Vec<String>>(&paths::favorites_file()).map(|v| v.into_iter().collect()).unwrap_or_default()
}

fn today() -> String {
    let (y, m, d) = crate::metadata::ymd(crate::metadata::now_naive());
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

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nixe-photos-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn png(w: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, 10, image::Rgba([1, 2, 3, 255]));
        crate::edit::pipeline::encode(&img, crate::edit::pipeline::Kind::Png, 90).unwrap()
    }

    #[test]
    fn a_saved_edit_is_added_once_and_refreshed_after() {
        let dir = scratch("lib");
        let path = dir.join("a.png");
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
    fn legacy_ids_move_to_stable_ones() {
        let mut lib = Library::default();
        let mut p = crate::state::tests::photo("x", "a.jpg", 2024, 1, 1);
        p.id = photo_id(&p.path);
        lib.favorites.insert(legacy_photo_id(&p.path));
        lib.albums.push(Album {
            id: "album_1".into(),
            name: "A".into(),
            photo_ids: vec![legacy_photo_id(&p.path)],
            created_at: String::new(),
        });
        lib.rotations.insert(legacy_photo_id(&p.path), 90);
        assert!(lib.migrate_ids(std::slice::from_ref(&p)));
        assert!(lib.favorites.contains(&p.id));
        assert_eq!(lib.albums[0].photo_ids, vec![p.id.clone()]);
        assert_eq!(lib.rotation(&p.id), 90);
        assert!(!lib.migrate_ids(&[p]));
    }

    #[test]
    fn duplicates_are_found_by_contents() {
        let dir = scratch("dups");
        let (a, b, c) = (dir.join("a.png"), dir.join("b.png"), dir.join("c.png"));
        std::fs::write(&a, png(20)).unwrap();
        std::fs::write(&b, png(20)).unwrap();
        let mut other = png(20);
        *other.last_mut().unwrap() ^= 1;
        std::fs::write(&c, &other).unwrap();
        let s = |p: &Path| p.to_string_lossy().to_string();
        let len = std::fs::metadata(&a).unwrap().len();
        let sets = find_duplicates(vec![("a".into(), s(&a), len), ("b".into(), s(&b), len), ("c".into(), s(&c), len)]);
        assert_eq!(sets.len(), 1);
        let mut set = sets[0].clone();
        set.sort();
        assert_eq!(set, vec!["a".to_string(), "b".to_string()]);
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
