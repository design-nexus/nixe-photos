//! Photos — a fast, keyboard-first photo gallery for Omarchy.

use gtk::prelude::*;
use gtk::{gio, glib};
use nixe_photos::{control, library, window};

const APP_ID: &str = "io.github.design_nexus.nixe.Photos";

const USAGE: &str = "Usage: nixe-photos [--section ID] [--edit] [FILE]

  FILE          open this photo or video (its folder is shown if it isn't in the library)
  --edit        with FILE, open it straight in the editor
  --section ID  open a page: all_photos, favorites, recent, folders, duplicates,
                photos, screenshots, videos, settings
  -h, --help    show this help
  -V, --version print the version
";

fn main() -> glib::ExitCode {
    nixe_core::app::init(nixe_core::app::App { slug: "nixe-photos", name: "Photos" });
    nixe_core::migrate::from(&["nexus-photos"]);
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return glib::ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("nixe-photos {}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }

    // One window: launching again hands the arguments to the open one.
    let mut flags = gio::ApplicationFlags::HANDLES_COMMAND_LINE;
    // For trying a build beside a running copy without taking it over.
    if std::env::var_os("NIXE_PHOTOS_STANDALONE").is_some() {
        flags |= gio::ApplicationFlags::NON_UNIQUE;
    }
    let app = gtk::Application::builder().application_id(APP_ID).flags(flags).build();
    // Nixe Assistant finds and shows photos through this (primary instance only).
    app.connect_startup(|app| nixe_core::control::serve(app, control::tools(), control::handle));
    app.connect_command_line(|app, cl| {
        let argv: Vec<String> = cl.arguments().iter().skip(1).map(|a| a.to_string_lossy().to_string()).collect();
        let section = argv.iter().position(|a| a == "--section").and_then(|i| argv.get(i + 1)).cloned();
        // Leave out the section's value so it isn't mistaken for a file.
        let edit = argv.iter().any(|a| a == "--edit");
        let rest: Vec<String> = argv
            .iter()
            .enumerate()
            .filter(|(i, a)| *a != "--section" && *a != "--edit" && !(*i > 0 && argv[*i - 1] == "--section"))
            .map(|(_, a)| a.clone())
            .collect();
        let file = library::file_argument(&rest);
        window::present(app, section.as_deref(), file, edit);
        glib::ExitCode::SUCCESS
    });
    app.run()
}
