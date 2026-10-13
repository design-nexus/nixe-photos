//! Things both the gallery and the stand-alone viewer do to a photo file: copy it to the
//! clipboard, move it to the trash, write a rotation into it, make it the background.

use crate::cmd;
use crate::edit::save;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::path::Path;

/// Omarchy's command for setting the desktop background.
pub const BACKGROUND_COMMAND: &str = "omarchy-theme-bg-set";

/// Put files on the clipboard. One picture also goes on as an image, upright, for
/// pasting into apps that take pictures. `done` gets a message to show.
pub fn copy(clipboard: &gdk::Clipboard, paths: Vec<String>, as_image: bool, done: impl FnOnce(String) + 'static) {
    if paths.is_empty() {
        return;
    }
    let files: Vec<gio::File> = paths.iter().map(gio::File::for_path).collect();
    let file_list = gdk::ContentProvider::for_value(&gdk::FileList::from_array(&files).to_value());
    if paths.len() > 1 || !as_image {
        clipboard.set_content(Some(&file_list)).ok();
        let n = paths.len();
        done(format!("Copied {n} {}", if n == 1 { "file" } else { "files" }));
        return;
    }
    let path = paths[0].clone();
    let clipboard = clipboard.clone();
    cmd::background(
        move || {
            let pb = gtk::gdk_pixbuf::Pixbuf::from_file(&path).ok()?;
            let pb = pb.apply_embedded_orientation().unwrap_or(pb);
            pb.save_to_bufferv("png", &[]).ok()
        },
        move |png: Option<Vec<u8>>| {
            let mut providers = vec![file_list];
            if let Some(png) = png {
                providers.insert(0, gdk::ContentProvider::for_bytes("image/png", &glib::Bytes::from_owned(png)));
            }
            clipboard.set_content(Some(&gdk::ContentProvider::new_union(&providers))).ok();
            done("Copied the image".into());
        },
    );
}

/// Move files to the trash. Returns the ones that went, and how many couldn't.
pub fn trash(paths: &[String]) -> (Vec<String>, usize) {
    let mut gone = Vec::new();
    let mut failed = 0;
    for p in paths {
        match gio::File::for_path(p).trash(gio::Cancellable::NONE) {
            Ok(()) => gone.push(p.clone()),
            Err(_) => failed += 1,
        }
    }
    (gone, failed)
}

/// The message after trashing.
pub fn trash_message(moved: usize, failed: usize) -> String {
    let plural = |n: usize| if n == 1 { "photo" } else { "photos" };
    match (moved, failed) {
        (n, 0) => format!("Moved {n} {} to the trash", plural(n)),
        (0, f) => format!("Couldn't move {f} {} to the trash", plural(f)),
        (n, f) => format!("Moved {n} {} to the trash. {f} couldn't be moved", plural(n)),
    }
}

/// Write a quarter-turn rotation into the file, off the UI thread, keeping a backup of
/// the original when the settings say so.
pub fn save_rotation(path: &Path, rotation: u32, done: impl FnOnce(anyhow::Result<save::Saved>) + 'static) {
    let p = crate::prefs::get();
    let opts = save::Options { mode: save::Mode::Replace, quality: p.jpeg_quality, backup: p.backup_originals };
    let st = crate::edit::EditState { rotation, ..Default::default() };
    let path = path.to_path_buf();
    cmd::background(move || save::save(&path, &st, &opts), done);
}

/// The message after a rotation was saved.
pub fn rotation_saved_message(saved: &save::Saved) -> &'static str {
    match saved.backup {
        Some(_) => "Saved the rotation. The original is backed up",
        None => "Saved the rotation",
    }
}

/// Whether the background can be set here.
pub fn can_set_background() -> bool {
    cmd::present(BACKGROUND_COMMAND)
}

/// Make a picture the desktop background. Returns the message to show.
pub fn set_background(path: &str) -> &'static str {
    if !can_set_background() {
        return "Setting the background needs Omarchy";
    }
    cmd::spawn(&[BACKGROUND_COMMAND, path]);
    "Set as the desktop background"
}
