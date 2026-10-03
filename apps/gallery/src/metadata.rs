use crate::models::{PhotoItem, format_bytes};
use exif::{Exif, In, Reader, Tag};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub fn extract_photo_item(path: &Path) -> Option<PhotoItem> {
    let metadata = fs::metadata(path).ok()?;
    let file_size = metadata.len();
    let mtime = metadata.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);

    let file_name = path.file_name()?.to_string_lossy().to_string();
    let ext = path.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_else(|| "IMAGE".to_string());

    let ext_lower = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();

    // Try reading dimensions using image crate header parser
    let (mut width, mut height) = image::image_dimensions(path).unwrap_or((0, 0));

    // For videos, try ffprobe for width and height
    if width == 0
        && height == 0
        && crate::scanner::VIDEO_EXTENSIONS.contains(&ext_lower.as_str())
        && let Ok(output) = std::process::Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-select_streams")
            .arg("v:0")
            .arg("-show_entries")
            .arg("stream=width,height")
            .arg("-of")
            .arg("csv=s=x:p=0")
            .arg(path)
            .output()
    {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = s.split('x').collect();
        if parts.len() == 2 {
            width = parts[0].parse().unwrap_or(0);
            height = parts[1].parse().unwrap_or(0);
        }
    }

    // Try reading EXIF
    let exif = File::open(path).ok().and_then(|file| {
        let mut bufreader = BufReader::new(file);
        Reader::new().read_from_container(&mut bufreader).ok()
    });

    let (camera_make, camera_model, lens_model, aperture, shutter_speed, iso, focal_length, exif_date) = if let Some(ref e) = exif
    {
        (
            get_tag_string(e, Tag::Make),
            get_tag_string(e, Tag::Model),
            get_tag_string(e, Tag::LensModel),
            get_tag_string(e, Tag::FNumber).map(|v| format!("f/{}", v)),
            get_tag_string(e, Tag::ExposureTime).map(|v| format!("{}s", v)),
            get_tag_string(e, Tag::PhotographicSensitivity).map(|v| format!("ISO {}", v)),
            get_tag_string(e, Tag::FocalLength).map(|v| format!("{}mm", v)),
            get_tag_string(e, Tag::DateTimeOriginal).or_else(|| get_tag_string(e, Tag::DateTime)),
        )
    } else {
        (None, None, None, None, None, None, None, None)
    };

    // Parse date or derive from mtime
    let (date_taken, date_display, year, month, _day) = parse_or_format_date(exif_date, mtime);

    // Classify media type strictly
    let lower_name = file_name.to_lowercase();
    let media_type = if crate::scanner::VIDEO_EXTENSIONS.contains(&ext_lower.as_str()) {
        "videos".to_string()
    } else if lower_name.contains("screenshot") || lower_name.contains("screen_shot") || lower_name.starts_with("scr_") {
        "screenshots".to_string()
    } else {
        "photos".to_string()
    };

    let thumbnail_path = crate::thumbnails::cache_path(path, mtime).to_string_lossy().to_string();

    Some(PhotoItem {
        id: format!("{:x}", md5_hash(&path.to_string_lossy())),
        file_name,
        path: path.to_string_lossy().to_string(),
        file_size,
        file_size_formatted: format_bytes(file_size),
        format: ext,
        date_taken,
        date_display,
        year,
        month,
        width,
        height,
        thumbnail_path,
        mtime,
        camera_make,
        camera_model,
        lens_model,
        focal_length,
        aperture,
        shutter_speed,
        iso,
        is_favorite: false,
        media_type,
    })
}

fn get_tag_string(exif: &Exif, tag: Tag) -> Option<String> {
    exif.get_field(tag, In::PRIMARY).map(|field| field.display_value().to_string())
}

fn md5_hash(text: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut s = DefaultHasher::new();
    text.hash(&mut s);
    s.finish()
}

fn parse_or_format_date(exif_date: Option<String>, mtime: u64) -> (String, String, i32, u32, u32) {
    if let Some(d) = exif_date {
        // EXIF format is usually "YYYY:MM:DD HH:MM:SS"
        let parts: Vec<&str> = d.split_whitespace().collect();
        if !parts.is_empty() {
            let date_parts: Vec<&str> = parts[0].split(':').collect();
            if date_parts.len() == 3 {
                let y = date_parts[0].parse::<i32>().unwrap_or(2026);
                let m = date_parts[1].parse::<u32>().unwrap_or(1);
                let day = date_parts[2].parse::<u32>().unwrap_or(1);
                let month_name = month_abbr(m);
                return (d.clone(), format!("{} {}, {}", month_name, day, y), y, m, day);
            }
        }
    }

    // Fallback using epoch seconds
    let days_since_epoch = mtime / 86400;
    // Simple calendar calculation
    let (y, m, day) = days_to_ymd(days_since_epoch);
    let month_name = month_abbr(m);
    (format!("{:04}-{:02}-{:02}", y, m, day), format!("{} {}, {}", month_name, day, y), y, m, day)
}

fn month_abbr(m: u32) -> &'static str {
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "Jan",
    }
}

// Convert unix epoch days to (year, month, day)
pub fn days_to_ymd(days: u64) -> (i32, u32, u32) {
    let mut d = days as i64;
    // Shift epoch to March 1, year 0
    d += 719468;
    let era = if d >= 0 { d } else { d - 146096 } / 146097;
    let doe = (d - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month, day)
}
