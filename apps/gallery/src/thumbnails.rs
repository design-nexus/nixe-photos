use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use image::ImageReader;

pub fn get_cache_dir() -> PathBuf {
    let cache_dir = dirs::cache_dir()
        .map(|c| c.join("nexus-photos").join("thumbnails"))
        .unwrap_or_else(|| PathBuf::from("/tmp/nexus-photos-thumbnails"));
    let _ = fs::create_dir_all(&cache_dir);
    cache_dir
}

pub fn generate_thumbnail(path: &Path, mtime: u64) -> String {
    let cache_dir = get_cache_dir();

    // Deterministic hash based on canonical path + modified time
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    mtime.hash(&mut hasher);
    let hash_val = hasher.finish();

    let thumb_file_name = format!("{:016x}.jpg", hash_val);
    let thumb_path = cache_dir.join(&thumb_file_name);

    if thumb_path.exists() {
        return thumb_path.to_string_lossy().to_string();
    }

    // Try generating thumbnail
    if let Ok(reader) = ImageReader::open(path) {
        if let Ok(reader) = reader.with_guessed_format() {
            if let Ok(img) = reader.decode() {
                let thumb = img.thumbnail(360, 360);
                if thumb.save(&thumb_path).is_ok() {
                    return thumb_path.to_string_lossy().to_string();
                }
            }
        }
    }

    // Fallback to original image path
    path.to_string_lossy().to_string()
}
