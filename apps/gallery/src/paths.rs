//! Well-known locations. Every path honours the XDG overrides so the whole app
//! can be pointed at a scratch copy of `~/.config` for testing.

use std::path::PathBuf;

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(fallback))
}

pub fn config_home() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config")
}

pub fn state_home() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state")
}

pub fn data_home() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share")
}

pub fn cache_home() -> PathBuf {
    xdg("XDG_CACHE_HOME", ".cache")
}

/// `~/.config/nexus-photos` — the preferences and user themes.
pub fn app_dir() -> PathBuf {
    config_home().join("nexus-photos")
}

pub fn prefs_file() -> PathBuf {
    app_dir().join("settings.toml")
}

pub fn custom_themes_dir() -> PathBuf {
    app_dir().join("themes")
}

/// `~/.local/share/nexus-photos` — albums and favourites.
pub fn data_dir() -> PathBuf {
    data_home().join("nexus-photos")
}

pub fn albums_file() -> PathBuf {
    data_dir().join("albums.json")
}

pub fn favorites_file() -> PathBuf {
    data_dir().join("favorites.json")
}

/// Quarter turns the viewer shows photos with.
pub fn rotations_file() -> PathBuf {
    data_dir().join("rotations.json")
}

/// What the last scan found, so the next start shows it at once.
pub fn library_cache() -> PathBuf {
    cache_home().join("nexus-photos/library.json")
}

/// Originals kept before an edit replaced them.
pub fn backup_dir() -> PathBuf {
    data_dir().join("backups")
}

pub fn thumbnail_dir() -> PathBuf {
    cache_home().join("nexus-photos/thumbnails")
}

pub fn omarchy_colors() -> PathBuf {
    state_home().join("omarchy/current/theme/colors.toml")
}

/// A saved folder: `~/…` is under the home folder.
pub fn expand(text: &str) -> PathBuf {
    match text.strip_prefix("~/") {
        Some(rest) => home().join(rest),
        None if text == "~" => home(),
        None => PathBuf::from(text),
    }
}

/// Replace `$HOME` with `~` for display.
pub fn pretty(path: &std::path::Path) -> String {
    match path.strip_prefix(home()) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}
