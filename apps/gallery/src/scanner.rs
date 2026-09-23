use std::path::{Path, PathBuf};
use rayon::prelude::*;
use walkdir::WalkDir;
use crate::metadata::extract_photo_item;
use crate::models::PhotoItem;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "tiff", "tif", "bmp", "gif"
];

pub fn scan_directory(dir: &Path) -> Vec<PhotoItem> {
    if !dir.exists() || !dir.is_dir() {
        return Vec::new();
    }

    // Collect all supported image paths
    let paths: Vec<PathBuf> = WalkDir::new(dir)
        .max_depth(8)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                SUPPORTED_EXTENSIONS.contains(&ext_str.as_str())
            } else {
                false
            }
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    // Process metadata and thumbnails in parallel across CPU cores
    let mut items: Vec<PhotoItem> = paths
        .par_iter()
        .filter_map(|p| extract_photo_item(p))
        .collect();

    // Sort newest first by date_taken / date
    items.sort_by(|a, b| b.date_taken.cmp(&a.date_taken));

    items
}
