//! This app's own preferences (`~/.config/nexus-photos/settings.toml`).

use crate::{cmd, paths};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    /// Follow the active Omarchy theme live.
    Omarchy,
    /// Use a bundled or custom theme.
    Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub mode: ThemeMode,
    pub theme: String,
    pub glow: bool,
    pub reduce_motion: bool,
    /// The sidebar shows only icons, whatever the window width.
    pub sidebar_collapsed: bool,
    pub last_section: String,
    /// How the timeline is grouped: `years`, `months`, `days` or `all`.
    pub view_mode: String,
    /// Thumbnail size, 0.0 (small) to 1.0 (large).
    pub zoom: f64,
    pub inspector_open: bool,
    /// Where a double-clicked photo opens: `builtin` (the gallery's viewer) or `viewer`
    /// (Nexus Photo Viewer, `npv`).
    pub open_with: String,
    /// What the editor's Save button does: `copy` or `replace`.
    pub save_mode: String,
    /// JPEG quality when saving an edit (1-100).
    pub jpeg_quality: u8,
    /// Keep a copy of the original before replacing it.
    pub backup_originals: bool,
    /// The library folders (`~/…` allowed). Empty means the Pictures folder.
    pub folders: Vec<String>,
    /// Thumbnail layout: `square` or `justified` (each photo keeps its shape).
    pub layout: String,
    /// Seconds each photo shows for in a slideshow.
    pub slideshow_seconds: u32,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Omarchy,
            theme: "tokyo-night".into(),
            glow: true,
            reduce_motion: false,
            sidebar_collapsed: false,
            last_section: "all_photos".into(),
            view_mode: "days".into(),
            zoom: 0.4,
            inspector_open: false,
            open_with: "builtin".into(),
            save_mode: "copy".into(),
            jpeg_quality: 92,
            backup_originals: true,
            folders: Vec::new(),
            layout: "square".into(),
            slideshow_seconds: 4,
        }
    }
}

thread_local! {
    static PREFS: RefCell<Prefs> = RefCell::new(load());
}

fn load() -> Prefs {
    std::fs::read_to_string(paths::prefs_file()).ok().and_then(|text| toml::from_str(&text).ok()).unwrap_or_default()
}

pub fn get() -> Prefs {
    PREFS.with(|p| p.borrow().clone())
}

/// Change a preference now and write the file shortly after the last change, so
/// dragging the zoom slider doesn't rewrite it on every tick.
pub fn update(change: impl FnOnce(&mut Prefs)) {
    PREFS.with(|p| change(&mut p.borrow_mut()));
    schedule_save();
}

thread_local! {
    static PENDING: RefCell<Option<gtk::glib::SourceId>> = const { RefCell::new(None) };
}

fn schedule_save() {
    PENDING.with(|pending| {
        if let Some(id) = pending.borrow_mut().take() {
            id.remove();
        }
        let id = gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(450), || {
            PENDING.with(|p| p.borrow_mut().take());
            flush();
        });
        *pending.borrow_mut() = Some(id);
    });
}

/// Write the preferences now (on quit, or when the debounce fires).
pub fn flush() {
    let prefs = get();
    if let Ok(text) = toml::to_string_pretty(&prefs) {
        let _ = cmd::atomic_write(&paths::prefs_file(), &text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_the_built_in_viewer_unless_told_otherwise() {
        assert_eq!(Prefs::default().open_with, "builtin");
        let old: Prefs = toml::from_str("zoom = 0.5").unwrap();
        assert_eq!(old.open_with, "builtin");
        let set: Prefs = toml::from_str("open_with = \"viewer\"").unwrap();
        assert_eq!(set.open_with, "viewer");
    }
}
