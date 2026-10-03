use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct PhotoItem {
    pub id: String,
    pub file_name: String,
    pub path: String,
    pub file_size: u64,
    pub file_size_formatted: String,
    pub format: String,
    pub date_taken: String,
    pub date_display: String,
    pub year: i32,
    pub month: u32,
    pub width: u32,
    pub height: u32,
    /// Where the cached thumbnail is, or will be once it has been made.
    pub thumbnail_path: String,
    pub mtime: u64,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length: Option<String>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub iso: Option<String>,
    pub is_favorite: bool,
    pub media_type: String, // "photos", "screenshots", "videos"
}

impl PhotoItem {
    /// A file that can be shown now: the cached thumbnail if it exists, else the original.
    pub fn preview_path(&self) -> &str {
        if std::path::Path::new(&self.thumbnail_path).exists() { &self.thumbnail_path } else { &self.path }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub photo_ids: Vec<String>,
    pub created_at: String,
}
