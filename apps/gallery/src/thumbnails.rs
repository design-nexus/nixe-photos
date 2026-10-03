use image::ImageReader;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub fn get_cache_dir() -> PathBuf {
    let cache_dir = crate::paths::thumbnail_dir();
    let _ = fs::create_dir_all(&cache_dir);
    cache_dir
}

/// Where the thumbnail for `path` lives (or will live) in the cache. The name depends
/// on the path and the modification time, so an edited photo gets a fresh one.
pub fn cache_path(path: &Path, mtime: u64) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    mtime.hash(&mut hasher);
    crate::paths::thumbnail_dir().join(format!("{:016x}.jpg", hasher.finish()))
}

/// Make the thumbnail if it isn't cached yet, and return the file to show: the
/// thumbnail, or the original when one can't be made. Reads the whole image, so run
/// it off the UI thread.
pub fn generate_thumbnail(path: &Path, mtime: u64) -> String {
    let _ = get_cache_dir();
    let thumb_path = cache_path(path, mtime);

    if thumb_path.exists() {
        return thumb_path.to_string_lossy().to_string();
    }

    let ext_lower = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();

    // If it is a video, use ffmpegthumbnailer or ffmpeg
    if crate::scanner::VIDEO_EXTENSIONS.contains(&ext_lower.as_str()) && generate_video_thumbnail(path, &thumb_path) {
        return thumb_path.to_string_lossy().to_string();
    }

    // Try generating image thumbnail
    if let Ok(reader) = ImageReader::open(path)
        && let Ok(reader) = reader.with_guessed_format()
        && let Ok(img) = reader.decode()
    {
        let thumb = img.thumbnail(360, 360);
        if thumb.save(&thumb_path).is_ok() {
            return thumb_path.to_string_lossy().to_string();
        }
    }

    // Fallback to original image path
    path.to_string_lossy().to_string()
}

fn generate_video_thumbnail(video_path: &Path, thumb_path: &Path) -> bool {
    use std::process::Command;

    // Try ffmpegthumbnailer first (fast)
    if let Ok(mut child) = Command::new("ffmpegthumbnailer")
        .arg("-i")
        .arg(video_path)
        .arg("-o")
        .arg(thumb_path)
        .arg("-s")
        .arg("360")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        && let Ok(status) = child.wait()
        && status.success()
        && thumb_path.exists()
    {
        return true;
    }

    // Fallback to ffmpeg
    if let Ok(mut child) = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg("00:00:01")
        .arg("-i")
        .arg(video_path)
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg("scale=360:360:force_original_aspect_ratio=decrease")
        .arg(thumb_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        && let Ok(status) = child.wait()
        && status.success()
        && thumb_path.exists()
    {
        return true;
    }

    false
}
