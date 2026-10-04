//! Exporting a selection: copies of the photos, optionally made smaller and saved in
//! another format, written to a folder. Reads whole pictures: run it off the UI thread.

use crate::edit::pipeline::{self, Kind};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Options {
    /// Longest side in pixels; `None` keeps the size.
    pub max_edge: Option<u32>,
    /// `None` keeps each file's format.
    pub format: Option<Kind>,
    pub quality: u8,
}

pub struct Report {
    pub written: usize,
    pub failed: Vec<String>,
}

pub fn export(files: &[PathBuf], dest: &Path, opts: &Options) -> Report {
    let mut report = Report { written: 0, failed: Vec::new() };
    if let Err(e) = std::fs::create_dir_all(dest) {
        report.failed = files.iter().map(|f| format!("{}: {e}", f.display())).collect();
        return report;
    }
    for f in files {
        match export_one(f, dest, opts) {
            Ok(_) => report.written += 1,
            Err(e) => report.failed.push(format!("{}: {e:#}", f.file_name().unwrap_or_default().to_string_lossy())),
        }
    }
    report
}

fn export_one(src: &Path, dest: &Path, opts: &Options) -> Result<PathBuf> {
    let stem = src.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let source_kind = pipeline::kind_of(src);
    let is_video =
        src.extension().is_some_and(|e| crate::scanner::VIDEO_EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()));
    // Unchanged (or a video, which isn't converted): copy the file as it is.
    let keep = is_video || (opts.max_edge.is_none() && (opts.format.is_none() || opts.format == source_kind));
    if keep {
        let ext = src.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        let target = unique(dest, &stem, &ext);
        std::fs::copy(src, &target).with_context(|| format!("could not copy to {}", target.display()))?;
        return Ok(target);
    }
    let kind = opts.format.or(source_kind).unwrap_or(Kind::Jpeg);
    let mut img = load_any(src)?;
    if let Some(edge) = opts.max_edge {
        img = pipeline::downscale(&img, edge);
    }
    let bytes = pipeline::encode(&img, kind, opts.quality)?;
    let ext = match kind {
        Kind::Jpeg => "jpg",
        Kind::Png => "png",
        Kind::WebP => "webp",
    };
    let target = unique(dest, &stem, ext);
    std::fs::write(&target, bytes).with_context(|| format!("could not write {}", target.display()))?;
    Ok(target)
}

/// Decode with the image crate, or gdk-pixbuf for the formats it can't read.
fn load_any(path: &Path) -> Result<image::RgbaImage> {
    if let Ok(img) = pipeline::load(path) {
        return Ok(img);
    }
    let pb = gtk::gdk_pixbuf::Pixbuf::from_file(path).context("could not read the picture")?;
    let pb = pb.apply_embedded_orientation().unwrap_or(pb);
    let pb = if pb.has_alpha() { pb } else { pb.add_alpha(false, 0, 0, 0).context("could not convert the picture")? };
    let (w, h, stride) = (pb.width() as u32, pb.height() as u32, pb.rowstride() as usize);
    let bytes = pb.read_pixel_bytes();
    let mut out = image::RgbaImage::new(w, h);
    for (y, row) in bytes.chunks(stride).take(h as usize).enumerate() {
        for x in 0..w as usize {
            let p = &row[x * 4..x * 4 + 4];
            out.put_pixel(x as u32, y as u32, image::Rgba([p[0], p[1], p[2], p[3]]));
        }
    }
    Ok(out)
}

/// `dest/stem.ext`, or `stem (2).ext` and so on if that is taken.
fn unique(dest: &Path, stem: &str, ext: &str) -> PathBuf {
    let name = |n: usize| {
        let base = if n == 1 { stem.to_string() } else { format!("{stem} ({n})") };
        if ext.is_empty() { base } else { format!("{base}.{ext}") }
    };
    (1..).map(|n| dest.join(name(n))).find(|p| !p.exists()).unwrap()
}

/// Where exports go unless another folder is picked.
pub fn default_folder() -> PathBuf {
    crate::library::default_root().join("Exports")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_smaller_copies_without_overwriting() {
        let dir = std::env::temp_dir().join(format!("nexus-photos-export-{}", std::process::id()));
        let out = dir.join("out");
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("a.png");
        let img = image::RgbaImage::from_pixel(400, 200, image::Rgba([10, 20, 30, 255]));
        std::fs::write(&src, pipeline::encode(&img, Kind::Png, 90).unwrap()).unwrap();
        let opts = Options { max_edge: Some(100), format: Some(Kind::Jpeg), quality: 80 };
        let r = export(&[src.clone(), src.clone()], &out, &opts);
        assert_eq!(r.written, 2);
        assert_eq!(image::image_dimensions(out.join("a.jpg")).unwrap(), (100, 50));
        assert!(out.join("a (2).jpg").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
