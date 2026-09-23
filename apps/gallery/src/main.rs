use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "c", rename_all = "snake_case")]
enum Request {
    Ping,
    GetStatus,
    GetTheme,
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
    // Fallback Catppuccin Mocha colors
    let mut fallback = serde_json::Map::new();
    fallback.insert("background".into(), "#1e1e2e".into());
    fallback.insert("surface".into(), "#242438".into());
    fallback.insert("dark_background".into(), "#161622".into());
    fallback.insert("lighter_background".into(), "#313244".into());
    fallback.insert("foreground".into(), "#cdd6f4".into());
    fallback.insert("accent".into(), "#89b4fa".into());
    fallback.insert("selection".into(), "#45475a".into());
    fallback.insert("muted".into(), "#585b70".into());
    fallback.insert("dark_foreground".into(), "#6c7086".into());
    (name, serde_json::Value::Object(fallback))
}

fn reveal_in_strata(target: &str) -> (bool, String) {
    let path = Path::new(target);
    let dir = if path.is_dir() {
        target
    } else {
        path.parent().and_then(|p| p.to_str()).unwrap_or(target)
    };

    // First try strata binary directly
    if Command::new("strata").arg(dir).spawn().is_ok() {
        return (true, format!("Opened {} in Strata", dir));
    }

    // Fallback to xdg-open
    match Command::new("xdg-open").arg(dir).spawn() {
        Ok(_) => (true, format!("Opened {} with system handler", dir)),
        Err(e) => (false, format!("Failed to open {}: {}", dir, e)),
    }
}

fn run_backend() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Send initial status immediately upon starting
    let pictures_dir = dirs::picture_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "~/Pictures".to_string());
    
    let initial_status = Response::Status {
        library_path: pictures_dir.clone(),
        theme_name: get_omarchy_theme_name(),
        total_items: 0,
    };
    if let Ok(line) = serde_json::to_string(&initial_status) {
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
                let theme_name = get_omarchy_theme_name();
                Response::Status {
                    library_path: pictures_dir.clone(),
                    theme_name,
                    total_items: 0,
                }
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

    // Check relative to current working directory
    let cwd_ui = PathBuf::from("ui");
    if cwd_ui.exists() {
        return cwd_ui;
    }

    // Check relative to executable
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

fn launch_gui() -> io::Result<()> {
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
    cmd.arg("-p")
        .arg(&shell_path)
        .env("NEXUS_BIN", &current_exe)
        .env("NEXUS_UI", &ui_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
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
        println!("  --backend     Run in backend IPC daemon mode");
        println!("  -h, --help    Show this help message");
        println!("  -V, --version Print version");
        Ok(())
    } else if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("nexus-photos {}", env!("CARGO_PKG_VERSION"));
        Ok(())
    } else {
        launch_gui()
    }
}
