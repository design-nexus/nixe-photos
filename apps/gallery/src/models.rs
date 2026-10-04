use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoItem {
    pub id: String,
    pub file_name: String,
    pub path: String,
    pub file_size: u64,
    pub file_size_formatted: String,
    pub format: String,
    /// When it was taken, as seconds on the local wall clock (EXIF date, else the file's
    /// modification time). Compare these, never the display text.
    pub taken: i64,
    pub date_display: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// Shown upright: EXIF orientation is already applied.
    pub width: u32,
    pub height: u32,
    /// Where the cached thumbnail is, or will be once it has been made.
    pub thumbnail_path: String,
    pub mtime: u64,
    /// When the file arrived on this machine (creation time, else `mtime`), Unix seconds.
    pub added: u64,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length: Option<String>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub iso: Option<String>,
    /// Decimal degrees, from the EXIF GPS fields.
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    /// Length of a video in seconds.
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(skip)]
    pub is_favorite: bool,
    pub media_type: String, // "photos", "screenshots", "videos"
}

impl PhotoItem {
    /// A file that can be shown now: the cached thumbnail if it exists, else the original.
    pub fn preview_path(&self) -> &str {
        if std::path::Path::new(&self.thumbnail_path).exists() { &self.thumbnail_path } else { &self.path }
    }

    pub fn is_video(&self) -> bool {
        self.media_type == "videos"
    }

    /// The folder the file is in.
    pub fn folder(&self) -> &str {
        std::path::Path::new(&self.path).parent().and_then(|p| p.to_str()).unwrap_or("")
    }

    /// Width over height, or 1 when unknown.
    pub fn aspect(&self) -> f64 {
        if self.width > 0 && self.height > 0 { self.width as f64 / self.height as f64 } else { 1.0 }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// `1:05`, or `1:02:03` for an hour or more.
pub fn format_duration(seconds: f64) -> String {
    let s = seconds.round().max(0.0) as u64;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub photo_ids: Vec<String>,
    pub created_at: String,
}

/// A stable 64-bit FNV-1a hash. Ids and thumbnail names are saved to disk, so they must
/// not change between builds (std's `DefaultHasher` may).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The id of the photo at `path`.
pub fn photo_id(path: &str) -> String {
    format!("{:016x}", fnv1a(path.as_bytes()))
}

/// The id older versions gave a path. Only used to carry favourites and albums over.
pub fn legacy_photo_id(path: &str) -> String {
    use std::hash::{Hash, Hasher};
    #[allow(deprecated)]
    let mut s = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut s);
    format!("{:x}", s.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable() {
        // FNV-1a test vector: a change here would orphan every saved favourite.
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(photo_id("a"), "af63dc4c8601ec8c");
    }

    #[test]
    fn durations() {
        assert_eq!(format_duration(5.4), "0:05");
        assert_eq!(format_duration(65.0), "1:05");
        assert_eq!(format_duration(3723.0), "1:02:03");
    }
}
