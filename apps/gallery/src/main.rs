mod models;
mod thumbnails;
mod metadata;
mod scanner;

use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

use models::{format_bytes, Album, PhotoItem};
use scanner::scan_directory;

#[derive(Debug, Deserialize)]
#[serde(tag = "c", rename_all = "snake_case")]
enum Request {
    Ping,
    GetStatus,
    GetTheme,
    ScanLibrary { path: Option<String> },
    GetPhotos { section: Option<String> },
    ToggleFavorite { id: String },
    CreateAlbum { name: String },
    RenameAlbum { album_id: String, name: String },
    AddPhotosToAlbum { album_id: String, photo_ids: Vec<String> },
    RemovePhotoFromAlbum { album_id: String, photo_id: String },
    DeleteAlbum { album_id: String },
    GetAlbums,
    RevealInStrata { path: String },
    Quit,
}

#[derive(Debug, Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum Response {
    Pong { version: &'static str },
    Status {
        library_path: String,
        theme_name: String,
        total_items: usize,
        total_bytes: u64,
        total_bytes_formatted: String,
    },
    LibraryScanned {
        library_path: String,
        total_items: usize,
        total_bytes: u64,
        total_bytes_formatted: String,
        photos: Vec<PhotoItem>,
        albums: Vec<Album>,
    },
    Photos {
        section: String,
        photos: Vec<PhotoItem>,
    },
    Albums {
        albums: Vec<Album>,
    },
    AlbumCreated {
        album: Album,
    },
    FavoriteToggled {
        id: String,
        is_favorite: bool,
    },
    Theme {
        name: String,
        colors: serde_json::Value,
    },
    RevealResult {
        ok: bool,
        message: String,
    },
    Error {
        message: String,
    },
    QuitReady,
}

fn get_omarchy_theme_name() -> String {
    if let Some(home) = dirs::home_dir() {
        let theme_name_path = home.join(".local/state/omarchy/current/theme.name");
        if let Ok(name) = fs::read_to_string(theme_name_path) {
            return name.trim().to_string();
        }
    }
    "Catppuccin".to_string()
}

fn parse_colors_toml(content: &str) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_string();
            let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(key, serde_json::Value::String(val));
        }
    }
    serde_json::Value::Object(map)
}

fn get_omarchy_colors() -> (String, serde_json::Value) {
    let name = get_omarchy_theme_name();
    if let Some(home) = dirs::home_dir() {
        let colors_path = home.join(".local/state/omarchy/current/theme/colors.toml");
        if let Ok(content) = fs::read_to_string(colors_path) {
            let parsed = parse_colors_toml(&content);
            return (name, parsed);
        }
    }
    let mut fallback = serde_json::Map::new();
    fallback.insert("background".into(), "#1e1e2e".into());
    fallback.insert("surface".into(), "#242438".into());
    fallback.insert("accent".into(), "#89b4fa".into());
    (name, serde_json::Value::Object(fallback))
}

fn get_albums_file() -> PathBuf {
    let dir = dirs::data_local_dir()
        .map(|d| d.join("nexus-photos"))
        .unwrap_or_else(|| PathBuf::from("."));
    let _ = fs::create_dir_all(&dir);
    dir.join("albums.json")
}

fn load_albums() -> Vec<Album> {
    let path = get_albums_file();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(albums) = serde_json::from_str(&content) {
            return albums;
        }
    }
    // Default initial album if none exist
    vec![Album {
        id: "highlight_album".to_string(),
        name: "Favorites & Highlights".to_string(),
        photo_ids: Vec::new(),
        created_at: "2026-09-22".to_string(),
    }]
}

fn get_favorites_file() -> PathBuf {
    let dir = dirs::data_local_dir()
        .map(|d| d.join("nexus-photos"))
        .unwrap_or_else(|| PathBuf::from("."));
    let _ = fs::create_dir_all(&dir);
    dir.join("favorites.json")
}

fn load_favorites() -> Vec<String> {
    let path = get_favorites_file();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(favs) = serde_json::from_str(&content) {
            return favs;
        }
    }
    Vec::new()
}

fn save_favorites(favs: &[String]) {
    let path = get_favorites_file();
    if let Ok(json) = serde_json::to_string_pretty(favs) {
        let _ = fs::write(path, json);
    }
}

fn save_albums(albums: &[Album]) {
    let path = get_albums_file();
    if let Ok(json) = serde_json::to_string_pretty(albums) {
        let _ = fs::write(path, json);
    }
}

fn reveal_in_strata(target: &str) -> (bool, String) {
    let path = Path::new(target);
    let dir = if path.is_dir() {
        target
    } else {
        path.parent().and_then(|p| p.to_str()).unwrap_or(target)
    };

    if Command::new("strata").arg(dir).spawn().is_ok() {
        return (true, format!("Opened {} in Strata", dir));
    }

    match Command::new("xdg-open").arg(dir).spawn() {
        Ok(_) => (true, format!("Opened {} with default handler", dir)),
        Err(e) => (false, format!("Failed to open {}: {}", dir, e)),
    }
}

fn run_backend() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let pictures_dir = dirs::picture_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join("Pictures"))
            .unwrap_or_else(|| PathBuf::from("."))
    });

    let photos_store: Arc<Mutex<Vec<PhotoItem>>> = Arc::new(Mutex::new(Vec::new()));
    let albums_store: Arc<Mutex<Vec<Album>>> = Arc::new(Mutex::new(load_albums()));

    // Initial scan on startup
    let favs = load_favorites();
    let mut initial_photos = scan_directory(&pictures_dir);
    for p in initial_photos.iter_mut() {
        if favs.contains(&p.id) {
            p.is_favorite = true;
        }
    }
    let total_bytes: u64 = initial_photos.iter().map(|p| p.file_size).sum();
    let total_items = initial_photos.len();

    {
        let mut store = photos_store.lock().unwrap();
        *store = initial_photos.clone();
    }

    let initial_albums = {
        let a = albums_store.lock().unwrap();
        a.clone()
    };

    // Send initial status, photos, and albums
    let initial_msg = Response::LibraryScanned {
        library_path: pictures_dir.to_string_lossy().to_string(),
        total_items,
        total_bytes,
        total_bytes_formatted: format_bytes(total_bytes),
        photos: initial_photos,
        albums: initial_albums,
    };
    if let Ok(line) = serde_json::to_string(&initial_msg) {
        let _ = writeln!(out, "{}", line);
        let _ = out.flush();
    }

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: Result<Request, _> = serde_json::from_str(line);
        let response = match request {
            Ok(Request::Ping) => Response::Pong {
                version: env!("CARGO_PKG_VERSION"),
            },
            Ok(Request::GetStatus) => {
                let store = photos_store.lock().unwrap();
                let bytes: u64 = store.iter().map(|p| p.file_size).sum();
                Response::Status {
                    library_path: pictures_dir.to_string_lossy().to_string(),
                    theme_name: get_omarchy_theme_name(),
                    total_items: store.len(),
                    total_bytes: bytes,
                    total_bytes_formatted: format_bytes(bytes),
                }
            }
            Ok(Request::ScanLibrary { path }) => {
                let target_dir = path
                    .map(PathBuf::from)
                    .unwrap_or_else(|| pictures_dir.clone());
                let mut scanned = scan_directory(&target_dir);
                let favs = load_favorites();
                for p in scanned.iter_mut() {
                    if favs.contains(&p.id) {
                        p.is_favorite = true;
                    }
                }
                let bytes: u64 = scanned.iter().map(|p| p.file_size).sum();
                let count = scanned.len();
                {
                    let mut store = photos_store.lock().unwrap();
                    *store = scanned.clone();
                }
                let albums = {
                    let a = albums_store.lock().unwrap();
                    a.clone()
                };
                Response::LibraryScanned {
                    library_path: target_dir.to_string_lossy().to_string(),
                    total_items: count,
                    total_bytes: bytes,
                    total_bytes_formatted: format_bytes(bytes),
                    photos: scanned,
                    albums,
                }
            }
            Ok(Request::GetPhotos { section }) => {
                let sec = section.unwrap_or_else(|| "all_photos".to_string());
                let store = photos_store.lock().unwrap();
                let filtered: Vec<PhotoItem> = match sec.as_str() {
                    "favorites" => store.iter().filter(|p| p.is_favorite).cloned().collect(),
                    "screenshots" => store
                        .iter()
                        .filter(|p| p.media_type == "screenshots")
                        .cloned()
                        .collect(),
                    "photos" => store
                        .iter()
                        .filter(|p| p.media_type == "photos")
                        .cloned()
                        .collect(),
                    _ => store.clone(),
                };
                Response::Photos {
                    section: sec,
                    photos: filtered,
                }
            }
            Ok(Request::ToggleFavorite { id }) => {
                let mut store = photos_store.lock().unwrap();
                let mut found_state = false;
                for p in store.iter_mut() {
                    if p.id == id {
                        p.is_favorite = !p.is_favorite;
                        found_state = p.is_favorite;
                        break;
                    }
                }
                let mut favs = load_favorites();
                if found_state {
                    if !favs.contains(&id) {
                        favs.push(id.clone());
                    }
                } else {
                    favs.retain(|f| f != &id);
                }
                save_favorites(&favs);

                Response::FavoriteToggled {
                    id,
                    is_favorite: found_state,
                }
            }
            Ok(Request::CreateAlbum { name }) => {
                let mut albums = albums_store.lock().unwrap();
                let new_id = format!("album_{}", albums.len() + 1);
                let album = Album {
                    id: new_id,
                    name,
                    photo_ids: Vec::new(),
                    created_at: "2026-09-22".to_string(),
                };
                albums.push(album.clone());
                save_albums(&albums);
                Response::AlbumCreated { album }
            }
            Ok(Request::RenameAlbum { album_id, name }) => {
                let mut albums = albums_store.lock().unwrap();
                for a in albums.iter_mut() {
                    if a.id == album_id {
                        a.name = name;
                        break;
                    }
                }
                save_albums(&albums);
                Response::Albums { albums: albums.clone() }
            }
            Ok(Request::AddPhotosToAlbum { album_id, photo_ids }) => {
                let mut albums = albums_store.lock().unwrap();
                for a in albums.iter_mut() {
                    if a.id == album_id {
                        for pid in &photo_ids {
                            if !a.photo_ids.contains(pid) {
                                a.photo_ids.push(pid.clone());
                            }
                        }
                        break;
                    }
                }
                save_albums(&albums);
                Response::Albums { albums: albums.clone() }
            }
            Ok(Request::RemovePhotoFromAlbum { album_id, photo_id }) => {
                let mut albums = albums_store.lock().unwrap();
                for a in albums.iter_mut() {
                    if a.id == album_id {
                        a.photo_ids.retain(|id| id != &photo_id);
                        break;
                    }
                }
                save_albums(&albums);
                Response::Albums { albums: albums.clone() }
            }
            Ok(Request::DeleteAlbum { album_id }) => {
                let mut albums = albums_store.lock().unwrap();
                albums.retain(|a| a.id != album_id);
                save_albums(&albums);
                Response::Albums { albums: albums.clone() }
            }
            Ok(Request::GetAlbums) => {
                let albums = albums_store.lock().unwrap();
                Response::Albums { albums: albums.clone() }
            }
            Ok(Request::GetTheme) => {
                let (name, colors) = get_omarchy_colors();
                Response::Theme { name, colors }
            }
            Ok(Request::RevealInStrata { path }) => {
                let (ok, message) = reveal_in_strata(&path);
                Response::RevealResult { ok, message }
            }
            Ok(Request::Quit) => {
                let _ = writeln!(out, "{}", serde_json::to_string(&Response::QuitReady).unwrap());
                let _ = out.flush();
                break;
            }
            Err(e) => Response::Error {
                message: format!("Malformed request: {}", e),
            },
        };

        if let Ok(json) = serde_json::to_string(&response) {
            let _ = writeln!(out, "{}", json);
            let _ = out.flush();
        }
    }

    Ok(())
}

fn locate_ui_dir() -> PathBuf {
    if let Ok(env_path) = env::var("NEXUS_UI") {
        let path = PathBuf::from(env_path);
        if path.exists() {
            return path;
        }
    }

    let cwd_ui = PathBuf::from("ui");
    if cwd_ui.exists() {
        return cwd_ui;
    }

    if let Some(data_dir) = dirs::data_local_dir() {
        let user_share = data_dir.join("nexus-photos").join("ui");
        if user_share.exists() {
            return user_share;
        }
    }

    if let Ok(mut exe) = env::current_exe() {
        exe.pop();
        let relative = exe.join("ui");
        if relative.exists() {
            return relative;
        }
        let share_dir = PathBuf::from("/usr/share/nexus-photos/ui");
        if share_dir.exists() {
            return share_dir;
        }
    }

    PathBuf::from("ui")
}

fn launch_gui(foreground: bool) -> io::Result<()> {
    let ui_dir = locate_ui_dir();
    let shell_path = ui_dir.join("shell.qml");

    if !shell_path.exists() {
        eprintln!(
            "nexus-photos: UI entrypoint not found at {}. Set NEXUS_UI or run from project root.",
            shell_path.display()
        );
        std::process::exit(1);
    }

    let current_exe = env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("nexus-photos"));

    let mut cmd = Command::new("quickshell");
    if !foreground {
        cmd.arg("-d");
    }
    cmd.arg("-p")
        .arg(&shell_path)
        .env("NEXUS_BIN", &current_exe)
        .env("NEXUS_UI", &ui_dir);

    if foreground {
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        match cmd.spawn() {
            Ok(_) => {
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("nexus-photos: failed to launch GUI: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--backend" {
        run_backend()
    } else if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        println!("Nexus Photos — Fast, keyboard-first photo gallery for Omarchy");
        println!();
        println!("Usage: nexus-photos [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -f, --foreground  Run in foreground attached to terminal");
        println!("  --backend         Run in backend IPC daemon mode");
        println!("  -h, --help        Show this help message");
        println!("  -V, --version     Print version");
        Ok(())
    } else if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("nexus-photos {}", env!("CARGO_PKG_VERSION"));
        Ok(())
    } else {
        let foreground = args.iter().any(|arg| arg == "--foreground" || arg == "-f");
        launch_gui(foreground)
    }
}
