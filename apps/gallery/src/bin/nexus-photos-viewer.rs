//! Nexus Photos Viewer — one photo, minimal chrome, with a way into the gallery.

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use nexus_photos::scanner::IMAGE_EXTENSIONS;
use nexus_photos::stage::Stage;
use nexus_photos::widgets::{self, hbox, label, vbox};
use nexus_photos::{cmd, library, theme};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

const APP_ID: &str = "io.github.nexus.photos.viewer";
const GALLERY: &str = "nexus-photos";

const USAGE: &str = "Usage: nexus-photos-viewer FILE\n\
\n\
  FILE          the photo to show; ← and → move through its folder\n\
  -h, --help    show this help\n\
  -V, --version print the version\n";

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return glib::ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("nexus-photos-viewer {}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }
    // Every photo gets its own window and process.
    let flags = gio::ApplicationFlags::HANDLES_COMMAND_LINE | gio::ApplicationFlags::NON_UNIQUE;
    let app = gtk::Application::builder().application_id(APP_ID).flags(flags).build();
    app.connect_command_line(|app, cl| {
        let argv: Vec<String> = cl.arguments().iter().skip(1).map(|a| a.to_string_lossy().to_string()).collect();
        match library::file_argument(&argv) {
            Some(file) => {
                build(app, file);
                glib::ExitCode::SUCCESS
            }
            None => {
                eprintln!("{USAGE}");
                glib::ExitCode::FAILURE
            }
        }
    });
    app.run()
}

/// The images beside `file` (and `file` itself), in name order.
fn siblings(file: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = file
        .parent()
        .and_then(|dir| std::fs::read_dir(dir).ok())
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && is_image(p))
        .collect();
    if !found.iter().any(|p| p == file) {
        found.push(file.to_path_buf());
    }
    found.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    found
}

fn is_image(path: &Path) -> bool {
    path.extension().is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()))
}

struct Viewer {
    app: gtk::Application,
    window: gtk::ApplicationWindow,
    stage: Stage,
    overlay: gtk::Overlay,
    name: gtk::Label,
    position: gtk::Label,
    files: Vec<PathBuf>,
    index: Cell<usize>,
    rotation: Cell<u32>,
    toast: RefCell<Option<gtk::Label>>,
}

fn build(app: &gtk::Application, file: PathBuf) {
    theme::install();
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Nexus Photos Viewer")
        .default_width(1000)
        .default_height(700)
        .build();
    window.add_css_class("photos-window");
    // No client-side titlebar: Hyprland manages the window.
    window.set_titlebar(Some(&gtk::Box::new(gtk::Orientation::Horizontal, 0)));
    window.set_icon_name(Some("io.github.nexus.photos"));

    let root = vbox(0);
    root.add_css_class("lightbox");

    // The same bar as the gallery's viewer: Open in gallery stands where Favorite does there.
    let top = hbox(8);
    top.add_css_class("lightbox-bar");
    let name = label("", "lightbox-title");
    name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    name.set_margin_start(8);
    let position = label("", "dim mono");
    let titles = hbox(12);
    titles.set_hexpand(true);
    titles.set_halign(gtk::Align::Start);
    titles.append(&name);
    titles.append(&position);
    top.append(&titles);
    let zoom_label = label("", "dim mono");
    zoom_label.set_valign(gtk::Align::Center);
    top.append(&zoom_label);
    let gallery = widgets::icon_button("view-grid-symbolic", "Open in gallery (G)");
    let edit = widgets::icon_button("document-edit-symbolic", "Edit in gallery (E)");
    let rotate = widgets::icon_button("object-rotate-right-symbolic", "Rotate 90° (R)");
    let reveal = widgets::icon_button("folder-open-symbolic", "Reveal in file manager (Ctrl+O)");
    for b in [&gallery, &edit, &rotate, &reveal] {
        top.append(b);
    }
    root.append(&top);

    let stage = Stage::new(zoom_label);
    root.append(stage.widget());
    window.set_child(Some(&root));
    let overlay = stage.widget().clone();

    let files = siblings(&file);
    let index = files.iter().position(|p| *p == file).unwrap_or(0);
    let v = Rc::new(Viewer {
        app: app.clone(),
        window: window.clone(),
        stage,
        overlay,
        name,
        position,
        files,
        index: Cell::new(index),
        rotation: Cell::new(0),
        toast: RefCell::new(None),
    });

    let me = v.clone();
    gallery.connect_clicked(move |_| me.open_in_gallery(false));
    let me = v.clone();
    edit.connect_clicked(move |_| me.open_in_gallery(true));
    let me = v.clone();
    rotate.connect_clicked(move |_| me.rotate());
    let me = v.clone();
    reveal.connect_clicked(move |_| me.reveal());

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let me = v.clone();
    keys.connect_key_pressed(move |_, key, _, state| {
        let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
        match key {
            gdk::Key::Escape => me.window.close(),
            gdk::Key::q | gdk::Key::w if ctrl => me.window.close(),
            gdk::Key::o if ctrl => me.reveal(),
            gdk::Key::Left | gdk::Key::Page_Up => me.step(-1),
            gdk::Key::Right | gdk::Key::Page_Down | gdk::Key::space => me.step(1),
            gdk::Key::Home => me.go(0),
            gdk::Key::End => me.go(me.files.len() - 1),
            gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => me.stage.zoom_by(1.25),
            gdk::Key::minus | gdk::Key::KP_Subtract => me.stage.zoom_by(0.8),
            gdk::Key::_0 | gdk::Key::KP_0 => me.stage.zoom_fit(),
            gdk::Key::_1 | gdk::Key::KP_1 => me.stage.zoom_actual(),
            gdk::Key::r | gdk::Key::R if !ctrl => me.rotate(),
            gdk::Key::e | gdk::Key::E if !ctrl => me.open_in_gallery(true),
            gdk::Key::g | gdk::Key::G if !ctrl => me.open_in_gallery(false),
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    window.add_controller(keys);

    v.show();
    window.present();
}

impl Viewer {
    fn current(&self) -> &Path {
        &self.files[self.index.get()]
    }

    fn show(&self) {
        let path = self.current().to_string_lossy().to_string();
        self.name.set_text(&self.current().file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
        self.position.set_text(&if self.files.len() > 1 {
            format!("{} / {}", self.index.get() + 1, self.files.len())
        } else {
            String::new()
        });
        self.window.set_title(Some(&format!("{} — Nexus Photos Viewer", self.name.text())));
        self.stage.show("", Some(&path), self.rotation.get());
    }

    fn go(&self, to: usize) {
        if to != self.index.get() {
            self.index.set(to);
            self.rotation.set(0);
            self.show();
        }
    }

    fn step(&self, by: isize) {
        let n = self.files.len() as isize;
        self.go((self.index.get() as isize + by).rem_euclid(n) as usize);
    }

    fn rotate(&self) {
        self.rotation.set((self.rotation.get() + 90) % 360);
        self.show();
    }

    fn reveal(&self) {
        library::reveal(&self.current().to_string_lossy());
    }

    /// Hand the photo to the gallery (optionally straight into its editor) and close.
    fn open_in_gallery(self: &Rc<Self>, edit: bool) {
        if !cmd::present(GALLERY) {
            self.toast("Nexus Photos isn't installed");
            return;
        }
        let mut command = std::process::Command::new(GALLERY);
        if edit {
            command.arg("--edit");
        }
        command.arg(self.current()).stdin(std::process::Stdio::null());
        match command.spawn() {
            Ok(_) => self.app.quit(),
            Err(e) => self.toast(&format!("Couldn't start Nexus Photos: {e}")),
        }
    }

    fn toast(&self, message: &str) {
        if let Some(old) = self.toast.borrow_mut().take() {
            self.overlay.remove_overlay(&old);
        }
        let l = label(message, "toast");
        l.set_halign(gtk::Align::Center);
        l.set_valign(gtk::Align::End);
        l.set_can_target(false);
        self.overlay.add_overlay(&l);
        *self.toast.borrow_mut() = Some(l.clone());
        let overlay = self.overlay.clone();
        glib::timeout_add_local_once(Duration::from_millis(3500), move || {
            if l.parent().is_some() {
                overlay.remove_overlay(&l);
            }
        });
    }
}
