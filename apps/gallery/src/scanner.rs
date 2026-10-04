use crate::metadata::extract_photo_item;
use crate::models::PhotoItem;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "tiff", "tif", "bmp", "gif", "avif", "heic", "heif"];

pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "mkv", "avi", "webm", "m4v", "flv", "wmv", "3gp"];

/// Folders watched for changes at most; past this, use Rescan.
const MAX_WATCHED: usize = 4000;

pub fn is_supported_media(ext: &str) -> bool {
    let lower = ext.to_lowercase();
    IMAGE_EXTENSIONS.contains(&lower.as_str()) || VIDEO_EXTENSIONS.contains(&lower.as_str())
}

pub struct Scan {
    /// Newest first.
    pub items: Vec<PhotoItem>,
    /// The folders that were looked in, for watching.
    pub dirs: Vec<PathBuf>,
}

/// Find photos and videos in `roots`, `depth` levels down (1 = just the folder itself).
/// Hidden folders are skipped. A file whose size and modification time match its entry
/// in `known` is not read again.
pub fn scan(roots: &[PathBuf], depth: usize, known: &HashMap<String, PhotoItem>) -> Scan {
    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    let mut dirs = Vec::new();
    for root in roots.iter().filter(|r| r.is_dir()) {
        let walker = WalkDir::new(root)
            .max_depth(depth)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'));
        for e in walker.filter_map(|e| e.ok()) {
            if e.file_type().is_dir() {
                if dirs.len() < MAX_WATCHED {
                    dirs.push(e.path().to_path_buf());
                }
                continue;
            }
            let supported = e.path().extension().is_some_and(|x| is_supported_media(&x.to_string_lossy()));
            if supported && seen.insert(e.path().to_path_buf()) {
                paths.push(e.path().to_path_buf());
            }
        }
    }
    dirs.sort();
    dirs.dedup();

    let mut items: Vec<PhotoItem> = paths.par_iter().filter_map(|p| cached(p, known).or_else(|| extract_photo_item(p))).collect();
    sort_newest_first(&mut items);
    Scan { items, dirs }
}

fn cached(path: &Path, known: &HashMap<String, PhotoItem>) -> Option<PhotoItem> {
    let item = known.get(path.to_str()?)?;
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    (mtime == item.mtime && meta.len() == item.file_size).then(|| item.clone())
}

/// Newest first; the same time falls back to the name, so the order is stable.
pub fn sort_newest_first(items: &mut [PhotoItem]) {
    items.sort_by(|a, b| b.taken.cmp(&a.taken).then_with(|| a.file_name.cmp(&b.file_name)));
}
