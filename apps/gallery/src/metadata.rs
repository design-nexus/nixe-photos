use crate::models::{PhotoItem, format_bytes, photo_id};
use exif::{Exif, In, Reader, Tag, Value};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub fn extract_photo_item(path: &Path) -> Option<PhotoItem> {
    let metadata = fs::metadata(path).ok()?;
    let file_size = metadata.len();
    let unix =
        |t: std::io::Result<std::time::SystemTime>| t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs());
    let mtime = unix(metadata.modified()).unwrap_or(0);
    let added = unix(metadata.created()).unwrap_or(mtime);

    let file_name = path.file_name()?.to_string_lossy().to_string();
    let ext = path.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_else(|| "IMAGE".to_string());
    let ext_lower = ext.to_lowercase();
    let is_video = crate::scanner::VIDEO_EXTENSIONS.contains(&ext_lower.as_str());

    let (mut width, mut height) = if is_video { (0, 0) } else { image::image_dimensions(path).unwrap_or((0, 0)) };
    let mut duration = None;
    if is_video {
        (width, height, duration) = probe_video(path);
    } else if width == 0 {
        // Formats the image crate can't read (AVIF, …): ask gdk-pixbuf.
        if let Some((_, w, h)) = gtk::gdk_pixbuf::Pixbuf::file_info(path) {
            (width, height) = (w.max(0) as u32, h.max(0) as u32);
        }
    }

    let exif = if is_video {
        None
    } else {
        File::open(path).ok().and_then(|file| Reader::new().read_from_container(&mut BufReader::new(file)).ok())
    };
    let e = exif.as_ref();
    let tag = |t: Tag| e.and_then(|e| get_tag_string(e, t));

    // Orientations 5 to 8 turn the picture a quarter: it is shown the other way round.
    if e.and_then(|e| e.get_field(Tag::Orientation, In::PRIMARY))
        .and_then(|f| f.value.get_uint(0))
        .is_some_and(|o| (5..=8).contains(&o))
    {
        std::mem::swap(&mut width, &mut height);
    }

    let exif_date = tag(Tag::DateTimeOriginal).or_else(|| tag(Tag::DateTime)).and_then(|d| parse_exif_date(&d));
    let taken = exif_date.unwrap_or_else(|| local_naive(mtime as i64));
    let (year, month, day) = ymd(taken);

    let (latitude, longitude) = match e {
        Some(e) => (
            gps_degrees(e, Tag::GPSLatitude, Tag::GPSLatitudeRef, "S"),
            gps_degrees(e, Tag::GPSLongitude, Tag::GPSLongitudeRef, "W"),
        ),
        None => (None, None),
    };

    let media_type = if is_video {
        "videos"
    } else if looks_like_screenshot(path) {
        "screenshots"
    } else {
        "photos"
    }
    .to_string();

    let thumbnail_path = crate::thumbnails::cache_path(path, mtime).to_string_lossy().to_string();
    let path_text = path.to_string_lossy().to_string();

    Some(PhotoItem {
        id: photo_id(&path_text),
        file_name,
        path: path_text,
        file_size,
        file_size_formatted: format_bytes(file_size),
        format: ext,
        taken,
        date_display: format!("{} {}, {}", month_abbr(month), day, year),
        year,
        month,
        day,
        width,
        height,
        thumbnail_path,
        mtime,
        added,
        camera_make: tag(Tag::Make),
        camera_model: tag(Tag::Model),
        lens_model: tag(Tag::LensModel),
        aperture: tag(Tag::FNumber).map(|v| format!("f/{v}")),
        shutter_speed: tag(Tag::ExposureTime).map(|v| format!("{v}s")),
        iso: tag(Tag::PhotographicSensitivity).map(|v| format!("ISO {v}")),
        focal_length: tag(Tag::FocalLength).map(|v| format!("{v}mm")),
        latitude,
        longitude,
        duration,
        is_favorite: false,
        media_type,
    })
}

/// Width, height and length of a video, from ffprobe when it is installed.
fn probe_video(path: &Path) -> (u32, u32, Option<f64>) {
    let Ok(out) = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height:format=duration",
            "-of",
            "default=nw=1",
        ])
        .arg(path)
        .output()
    else {
        return (0, 0, None);
    };
    let (mut w, mut h, mut d) = (0, 0, None);
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        match line.split_once('=') {
            Some(("width", v)) => w = v.trim().parse().unwrap_or(0),
            Some(("height", v)) => h = v.trim().parse().unwrap_or(0),
            Some(("duration", v)) => d = v.trim().parse().ok().filter(|d: &f64| d.is_finite() && *d > 0.0),
            _ => {}
        }
    }
    (w, h, d)
}

/// A screenshot by its name, or by sitting in a Screenshots folder.
pub fn looks_like_screenshot(path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let named = ["screenshot", "screen_shot", "screen shot", "bildschirmfoto", "capture d"].iter().any(|k| name.contains(k))
        || name.starts_with("scr_")
        || name.starts_with("grim");
    let in_folder = path
        .parent()
        .into_iter()
        .flat_map(|p| p.components())
        .any(|c| matches!(c.as_os_str().to_string_lossy().to_lowercase().as_str(), "screenshots" | "screenshot"));
    named || in_folder
}

fn get_tag_string(exif: &Exif, tag: Tag) -> Option<String> {
    exif.get_field(tag, In::PRIMARY)
        .map(|field| field.display_value().to_string().trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty() && s != "unknown")
}

/// Degrees from a GPS latitude or longitude (degrees, minutes, seconds), negative
/// in the southern or western half.
fn gps_degrees(exif: &Exif, tag: Tag, reference: Tag, negative: &str) -> Option<f64> {
    let Value::Rational(parts) = &exif.get_field(tag, In::PRIMARY)?.value else { return None };
    let v: Vec<f64> = parts.iter().map(|r| r.to_f64()).collect();
    let deg = v.first()? + v.get(1).unwrap_or(&0.0) / 60.0 + v.get(2).unwrap_or(&0.0) / 3600.0;
    if !deg.is_finite() || deg == 0.0 {
        return None;
    }
    let neg = get_tag_string(exif, reference).is_some_and(|r| r.to_uppercase().starts_with(negative));
    Some(if neg { -deg } else { deg })
}

/// Read `YYYY:MM:DD HH:MM:SS` (or with dashes) into local wall-clock seconds.
pub fn parse_exif_date(text: &str) -> Option<i64> {
    let nums: Vec<i64> =
        text.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();
    let (y, m, d) = (*nums.first()?, *nums.get(1)?, *nums.get(2)?);
    if !(1800..=3000).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let t = |i: usize| nums.get(i).copied().unwrap_or(0);
    Some(naive(y as i32, m as u32, d as u32, t(3), t(4), t(5)))
}

/// Seconds since 1970-01-01 00:00 on a wall clock with no time zone.
pub fn naive(y: i32, m: u32, d: u32, h: i64, mi: i64, s: i64) -> i64 {
    days_from_civil(y, m, d) * 86400 + h * 3600 + mi * 60 + s
}

/// Unix time as seconds on the local wall clock.
pub fn local_naive(unix: i64) -> i64 {
    let offset = gtk::glib::DateTime::from_unix_local(unix).map(|d| d.utc_offset().as_seconds()).unwrap_or(0);
    unix + offset
}

/// Now, on the local wall clock.
pub fn now_naive() -> i64 {
    let secs = std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    local_naive(secs)
}

/// The calendar day of a wall-clock time.
pub fn ymd(naive: i64) -> (i32, u32, u32) {
    days_to_ymd(naive.div_euclid(86400))
}

pub fn month_abbr(m: u32) -> &'static str {
    const NAMES: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    NAMES.get((m as usize).wrapping_sub(1)).copied().unwrap_or("Jan")
}

/// Days since 1970-01-01 of a calendar date.
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64 - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Convert days since 1970-01-01 to (year, month, day).
pub fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    let d = days + 719468;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_exif_date_spellings() {
        let a = parse_exif_date("2024:05:01 13:22:10").unwrap();
        let b = parse_exif_date("2024-05-01 13:22:10").unwrap();
        assert_eq!(a, b);
        assert_eq!(ymd(a), (2024, 5, 1));
        assert_eq!(a % 86400, 13 * 3600 + 22 * 60 + 10);
        assert!(parse_exif_date("0000:00:00 00:00:00").is_none());
    }

    #[test]
    fn days_round_trip() {
        for days in [-1000, 0, 19000, 20000, 25000] {
            let (y, m, d) = days_to_ymd(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
    }

    #[test]
    fn screenshots_by_name_or_folder() {
        assert!(looks_like_screenshot(Path::new("/p/screenshot-2024.png")));
        assert!(looks_like_screenshot(Path::new("/p/Screenshots/a.png")));
        assert!(!looks_like_screenshot(Path::new("/p/holiday/a.jpg")));
    }
}
