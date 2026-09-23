use std::path::{Path, PathBuf};
use rayon::prelude::*;
use walkdir::WalkDir;
use crate::metadata::extract_photo_item;
use crate::models::PhotoItem;

pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "tiff", "tif", "bmp", "gif", "avif"
];

pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mkv", "avi", "webm", "m4v", "flv", "wmv", "3gp"
];

pub fn is_supported_media(ext: &str) -> bool {
    let lower = ext.to_lowercase();
    IMAGE_EXTENSIONS.contains(&lower.as_str()) || VIDEO_EXTENSIONS.contains(&lower.as_str())
}

pub fn scan_directory(dir: &Path) -> Vec<PhotoItem> {
    if !dir.exists() || !dir.is_dir() {
        return Vec::new();
    }

    // Collect all supported image and video paths
    let paths: Vec<PathBuf> = WalkDir::new(dir)
        .max_depth(8)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                is_supported_media(&ext.to_string_lossy())
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
