use crate::models::fnv1a;
use image::{DynamicImage, ImageDecoder, ImageReader};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Longest side of a thumbnail, in pixels: sharp at the largest grid size on a 2× screen.
pub const SIZE: u32 = 512;
/// Part of every thumbnail's name; bump it to have them all made again.
const VERSION: &str = "v2";

pub fn get_cache_dir() -> PathBuf {
    let cache_dir = crate::paths::thumbnail_dir();
    let _ = fs::create_dir_all(&cache_dir);
    cache_dir
}

/// Where the thumbnail for `path` lives (or will live) in the cache. The name depends
/// on the path and the modification time, so an edited photo gets a fresh one.
pub fn cache_path(path: &Path, mtime: u64) -> PathBuf {
    let key = format!("{VERSION}\0{}\0{mtime}", path.to_string_lossy());
    crate::paths::thumbnail_dir().join(format!("{:016x}.jpg", fnv1a(key.as_bytes())))
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

    // Written under a hidden name and then renamed, so a thumbnail is never read half-made.
    let tmp = thumb_path.with_file_name(format!(
        ".{}-{}-{:?}.jpg",
        thumb_path.file_stem().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        std::thread::current().id()
    ));
    let ext_lower = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let made = if crate::scanner::VIDEO_EXTENSIONS.contains(&ext_lower.as_str()) {
        generate_video_thumbnail(path, &tmp)
    } else {
        upright(path)
            .is_some_and(|img| img.thumbnail(SIZE, SIZE).to_rgb8().save_with_format(&tmp, image::ImageFormat::Jpeg).is_ok())
            || pixbuf_thumbnail(path, &tmp)
    };
    if made && fs::rename(&tmp, &thumb_path).is_ok() {
        return thumb_path.to_string_lossy().to_string();
    }
    let _ = fs::remove_file(&tmp);
    path.to_string_lossy().to_string()
}

/// Decode with the image crate and turn the picture upright by its EXIF orientation.
fn upright(path: &Path) -> Option<DynamicImage> {
    let mut decoder = ImageReader::open(path).ok()?.with_guessed_format().ok()?.into_decoder().ok()?;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).ok()?;
    img.apply_orientation(orientation);
    Some(img)
}

/// For formats the image crate doesn't read (AVIF, HEIC, …): gdk-pixbuf's loaders.
fn pixbuf_thumbnail(path: &Path, thumb_path: &Path) -> bool {
    let Ok(pb) = gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(path, SIZE as i32, SIZE as i32, true) else { return false };
    let pb = pb.apply_embedded_orientation().unwrap_or(pb);
    pb.savev(thumb_path, "jpeg", &[("quality", "88")]).is_ok()
}

fn generate_video_thumbnail(video_path: &Path, thumb_path: &Path) -> bool {
    use std::process::{Command, Stdio};
    let size = SIZE.to_string();

    // Try ffmpegthumbnailer first (fast)
    let ok = Command::new("ffmpegthumbnailer")
        .arg("-i")
        .arg(video_path)
        .arg("-o")
        .arg(thumb_path)
        .args(["-s", &size])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if ok && thumb_path.exists() {
        return true;
    }

    // Fallback to ffmpeg
    let ok = Command::new("ffmpeg")
        .args(["-y", "-ss", "00:00:01", "-i"])
        .arg(video_path)
        .args(["-vframes", "1", "-vf", &format!("scale={size}:{size}:force_original_aspect_ratio=decrease")])
        .arg(thumb_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    ok && thumb_path.exists()
}

/// Delete cached thumbnails that no photo in `keep` uses (deleted, edited or moved
/// photos, and thumbnails from older versions). Returns how many went.
pub fn prune(keep: &HashSet<PathBuf>) -> usize {
    let Ok(entries) = fs::read_dir(crate::paths::thumbnail_dir()) else { return 0 };
    let mut removed = 0;
    for e in entries.flatten() {
        let p = e.path();
        // Hidden names are thumbnails being written.
        let in_progress = e.file_name().to_string_lossy().starts_with('.');
        if !in_progress && p.extension().is_some_and(|x| x == "jpg") && !keep.contains(&p) && fs::remove_file(&p).is_ok() {
            removed += 1;
        }
    }
    removed
}
