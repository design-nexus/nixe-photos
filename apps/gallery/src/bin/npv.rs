//! Nexus Photo Viewer (`npv`) — one folder of photos in a light window, with a way into
//! the gallery. It shares the gallery's viewer, favourites and rotations.

use gtk::prelude::*;
use gtk::{gdk, glib};
use nexus_photos::grid::CellData;
use nexus_photos::library::{self, Library};
use nexus_photos::lightbox::{self, Action, Lightbox};
use nexus_photos::models::{PhotoItem, photo_id};
use nexus_photos::scanner::IMAGE_EXTENSIONS;
use nexus_photos::widgets::hbox;
use nexus_photos::{actions, cmd, dialog, metadata, theme, thumbnails};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

const APP_ID: &str = "io.github.nexus.photos.viewer";
const NAME: &str = "Nexus Photo Viewer";
const GALLERY: &str = "nexus-photos";

const USAGE: &str = "Usage: npv [FILE]

  FILE          the photo to show; ← and → move through its folder.
                Without one, the window offers to open a photo.
  -h, --help    show this help
  -V, --version print the version
";

const SHORTCUTS: &dialog::Shortcuts = &[(
    "Viewer",
    &[
        ("Previous and next photo", "←, →"),
        ("First and last photo", "Home, End"),
        ("Zoom in, out, fit, 100%", "+, −, 0, 1"),
        ("Rotate the view", "R"),
        ("Save the rotation to the file", "Ctrl+S"),
        ("Favorite", "F"),
        ("Slideshow", "S"),
        ("Set as the background", "W"),
        ("Copy the image", "Ctrl+C"),
        ("Rename", "F2"),
        ("Move to the trash", "Delete"),
        ("Reveal in the file manager", "Ctrl+O"),
        ("Open another photo", "Ctrl+Shift+O"),
        ("Open in the gallery", "G"),
        ("Edit in the gallery", "E"),
        ("Show these shortcuts", "?"),
        ("Close", "Esc"),
    ],
)];

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return glib::ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("npv {}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }
    // Every photo gets its own window and process.
    let flags = gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE | gtk::gio::ApplicationFlags::NON_UNIQUE;
    let app = gtk::Application::builder().application_id(APP_ID).flags(flags).build();
    app.connect_command_line(|app, cl| {
        let argv: Vec<String> = cl.arguments().iter().skip(1).map(|a| a.to_string_lossy().to_string()).collect();
        let file = library::file_argument(&argv);
        // A name that isn't a file is a mistake; no name at all is fine.
        if file.is_none() && argv.iter().any(|a| !a.starts_with('-')) {
            eprintln!("npv: no such file: {}\n\n{USAGE}", argv.iter().find(|a| !a.starts_with('-')).unwrap());
            return glib::ExitCode::FAILURE;
        }
        build(app, file);
        glib::ExitCode::SUCCESS
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
        .filter(|p| p.is_file() && is_image(p) && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')))
        .collect();
    if !found.iter().any(|p| p == file) {
        found.push(file.to_path_buf());
    }
    found.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    found
}

/// Pictures the viewer steps through beyond the gallery's photo formats.
const VIEWER_EXTENSIONS: &[&str] = &["svg", "ico"];

fn is_image(path: &Path) -> bool {
    path.extension().is_some_and(|e| {
        let e = e.to_string_lossy().to_lowercase();
        IMAGE_EXTENSIONS.contains(&e.as_str()) || VIEWER_EXTENSIONS.contains(&e.as_str())
    })
}

struct Viewer {
    app: gtk::Application,
    window: gtk::ApplicationWindow,
    lightbox: Lightbox,
    /// The photo, or the "Open a photo" page when there is none.
    pages: gtk::Stack,
    /// Holds the pages and the toasts.
    overlay: gtk::Overlay,
    files: RefCell<Vec<PathBuf>>,
    index: Cell<usize>,
    /// Details read so far, by path.
    items: RefCell<HashMap<PathBuf, PhotoItem>>,
    toast: RefCell<Option<gtk::Revealer>>,
}

fn build(app: &gtk::Application, file: Option<PathBuf>) {
    theme::install();
    let window = gtk::ApplicationWindow::builder().application(app).title(NAME).default_width(1000).default_height(700).build();
    window.add_css_class("photos-window");
    // No client-side titlebar: Hyprland manages the window.
    window.set_titlebar(Some(&gtk::Box::new(gtk::Orientation::Horizontal, 0)));
    window.set_icon_name(Some("io.github.nexus.photos"));

    let lightbox = Lightbox::new(lightbox::Mode::Standalone);
    let pages = gtk::Stack::new();
    pages.add_named(&empty_page(), Some("empty"));
    pages.add_named(lightbox.widget(), Some("photo"));
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&pages));
    window.set_child(Some(&overlay));

    let v = Rc::new(Viewer {
        app: app.clone(),
        window: window.clone(),
        lightbox,
        pages,
        overlay,
        files: RefCell::default(),
        index: Cell::new(0),
        items: RefCell::default(),
        toast: RefCell::new(None),
    });

    let me = v.clone();
    v.lightbox.on_action(move |a| me.on_action(a));

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let me = v.clone();
    keys.connect_key_pressed(move |_, key, _, state| me.on_key(key, state));
    window.add_controller(keys);

    // Drop a picture on the window to show it.
    let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    let me = v.clone();
    drop.connect_drop(move |_, value, _, _| {
        let Ok(list) = value.get::<gdk::FileList>() else { return false };
        match list.files().iter().filter_map(|f| f.path()).find(|p| p.is_file()) {
            Some(path) => {
                me.load(&path);
                true
            }
            None => false,
        }
    });
    window.add_controller(drop);

    if let Some(button) = find_open_button(&v.pages) {
        let me = v.clone();
        button.connect_clicked(move |_| me.choose());
    }

    match file {
        Some(f) => v.load(&f),
        None => v.show(),
    }
    window.present();
}

/// What the window shows with no photo: a way to open one.
fn empty_page() -> gtk::Box {
    let page = nexus_photos::widgets::vbox(10);
    page.add_css_class("lightbox");
    page.add_css_class("viewer-empty");
    page.set_valign(gtk::Align::Fill);
    let inner = nexus_photos::widgets::vbox(10);
    inner.set_valign(gtk::Align::Center);
    inner.set_vexpand(true);
    let icon = gtk::Image::from_icon_name("image-x-generic-symbolic");
    icon.set_pixel_size(64);
    icon.add_css_class("dim");
    inner.append(&icon);
    let title = nexus_photos::widgets::label(NAME, "section-title");
    title.set_halign(gtk::Align::Center);
    inner.append(&title);
    let text = nexus_photos::widgets::label("Open a photo, or drop one here. ← and → then move through its folder.", "dim");
    text.set_halign(gtk::Align::Center);
    text.set_wrap(true);
    text.set_justify(gtk::Justification::Center);
    inner.append(&text);
    let open = gtk::Button::new();
    open.set_child(Some(&nexus_photos::widgets::button_content("document-open-symbolic", "Open a photo…")));
    open.add_css_class("suggested-action");
    open.add_css_class("open-photo");
    open.set_halign(gtk::Align::Center);
    open.set_margin_top(8);
    open.set_tooltip_text(Some("Ctrl+O"));
    inner.append(&open);
    page.append(&inner);
    page
}

fn find_open_button(w: &impl IsA<gtk::Widget>) -> Option<gtk::Button> {
    let w = w.as_ref();
    if w.has_css_class("open-photo") {
        return w.clone().downcast().ok();
    }
    let mut child = w.first_child();
    while let Some(c) = child {
        if let Some(b) = find_open_button(&c) {
            return Some(b);
        }
        child = c.next_sibling();
    }
    None
}

/// The saved favourites, albums and rotations, read fresh: the gallery may have changed them.
fn saved() -> Library {
    Library::load(Vec::new())
}

impl Viewer {
    fn current(&self) -> PathBuf {
        self.files.borrow()[self.index.get()].clone()
    }

    fn id(&self) -> String {
        photo_id(&self.current().to_string_lossy())
    }

    /// The photo's details, read once.
    fn item(&self, path: &Path) -> PhotoItem {
        if let Some(i) = self.items.borrow().get(path) {
            return i.clone();
        }
        let item = metadata::extract_photo_item(path).unwrap_or_else(|| fallback_item(path));
        self.items.borrow_mut().insert(path.to_path_buf(), item.clone());
        item
    }

    /// Show `file`, with the other pictures in its folder to move through.
    fn load(&self, file: &Path) {
        let files = siblings(file);
        let index = files.iter().position(|p| p == file).unwrap_or(0);
        *self.files.borrow_mut() = files;
        self.items.borrow_mut().clear();
        self.index.set(index);
        self.show();
    }

    /// Pick a photo with the file chooser.
    fn choose(self: &Rc<Self>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Pictures"));
        for ext in IMAGE_EXTENSIONS.iter().chain(VIEWER_EXTENSIONS) {
            filter.add_suffix(ext);
        }
        let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let chooser =
            gtk::FileDialog::builder().title("Open a photo").modal(true).filters(&filters).default_filter(&filter).build();
        let start = self
            .files
            .borrow()
            .get(self.index.get())
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(library::default_root);
        if start.is_dir() {
            chooser.set_initial_folder(Some(&gtk::gio::File::for_path(&start)));
        }
        let me = self.clone();
        chooser.open(Some(&self.window), gtk::gio::Cancellable::NONE, move |result| {
            if let Some(path) = result.ok().and_then(|f| f.path()) {
                me.load(&path);
            }
        });
    }

    fn show(&self) {
        let files = self.files.borrow().clone();
        if files.is_empty() {
            self.lightbox.hide();
            self.pages.set_visible_child_name("empty");
            self.window.set_title(Some(NAME));
            return;
        }
        self.pages.set_visible_child_name("photo");
        let at = self.index.get().min(files.len() - 1);
        self.index.set(at);
        let lib = saved();
        let mut photo = self.item(&files[at]);
        photo.is_favorite = lib.is_favorite(&photo.id);
        let from = at.saturating_sub(lightbox::STRIP_SIDE);
        let to = (at + lightbox::STRIP_SIDE + 1).min(files.len());
        let strip = (from..to).map(|i| (i, cell_for(&files[i]))).collect();
        self.lightbox.show(&photo, lib.rotation(&photo.id), at + 1, files.len(), strip);
        self.window.set_title(Some(&format!("{} — {NAME}", photo.file_name)));
        let n = files.len();
        if n > 1 {
            for i in [(at + 1) % n, (at + n - 1) % n] {
                let p = &files[i];
                nexus_photos::stage::preload(&p.to_string_lossy(), lib.rotation(&photo_id(&p.to_string_lossy())));
            }
        }
    }

    fn go(&self, to: usize) {
        self.index.set(to);
        self.show();
    }

    fn step(&self, by: isize) {
        let n = self.files.borrow().len() as isize;
        if n > 0 {
            self.go((self.index.get() as isize + by).rem_euclid(n) as usize);
        }
    }

    fn on_key(self: &Rc<Self>, key: gdk::Key, state: gdk::ModifierType) -> glib::Propagation {
        let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
        if key != gdk::Key::s && self.lightbox.is_playing() {
            self.lightbox.stop_slideshow();
        }
        if self.files.borrow().is_empty() {
            match key {
                gdk::Key::q | gdk::Key::w if ctrl => self.window.close(),
                gdk::Key::o if ctrl => self.choose(),
                gdk::Key::Escape => self.window.close(),
                gdk::Key::question => dialog::show_shortcuts(SHORTCUTS),
                _ => return glib::Propagation::Proceed,
            }
            return glib::Propagation::Stop;
        }
        match key {
            gdk::Key::q | gdk::Key::w if ctrl => self.window.close(),
            // With Shift held the key arrives as a capital O.
            gdk::Key::o | gdk::Key::O if ctrl && state.contains(gdk::ModifierType::SHIFT_MASK) => self.choose(),
            gdk::Key::o if ctrl => self.reveal(),
            gdk::Key::c if ctrl => self.copy(),
            gdk::Key::s if ctrl => self.save_rotation(),
            _ if ctrl => return glib::Propagation::Proceed,
            gdk::Key::Escape => self.window.close(),
            gdk::Key::Left | gdk::Key::Page_Up => self.step(-1),
            gdk::Key::Right | gdk::Key::Page_Down | gdk::Key::space => self.step(1),
            gdk::Key::Home => self.go(0),
            gdk::Key::End => self.go(self.files.borrow().len().saturating_sub(1)),
            gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => self.lightbox.zoom_by(1.25),
            gdk::Key::minus | gdk::Key::KP_Subtract => self.lightbox.zoom_by(0.8),
            gdk::Key::_0 | gdk::Key::KP_0 => self.lightbox.zoom_fit(),
            gdk::Key::_1 | gdk::Key::KP_1 => self.lightbox.zoom_actual(),
            gdk::Key::r | gdk::Key::R => self.rotate(),
            gdk::Key::f | gdk::Key::period => self.toggle_favorite(),
            gdk::Key::s => self.toggle_slideshow(),
            gdk::Key::w => self.set_background(),
            gdk::Key::Delete | gdk::Key::KP_Delete => self.delete(),
            gdk::Key::F2 => self.rename(),
            gdk::Key::question => dialog::show_shortcuts(SHORTCUTS),
            gdk::Key::e | gdk::Key::E => self.open_in_gallery(true),
            gdk::Key::g | gdk::Key::G => self.open_in_gallery(false),
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    }

    fn on_action(self: &Rc<Self>, a: Action) {
        match a {
            Action::Close => self.window.close(),
            Action::Prev => self.step(-1),
            Action::Next => self.step(1),
            Action::Rotate => self.rotate(),
            Action::Reveal => self.reveal(),
            Action::ToggleFavorite => self.toggle_favorite(),
            Action::Edit => self.open_in_gallery(true),
            Action::OpenGallery => self.open_in_gallery(false),
            Action::Play => {}
            Action::Slideshow => self.toggle_slideshow(),
            Action::SaveRotation => self.save_rotation(),
            Action::ResetRotation => {
                saved().set_rotation(&self.id(), 0);
                self.show();
            }
            Action::Wallpaper => self.set_background(),
            Action::CopyImage => self.copy(),
            Action::Jump(i) => {
                self.lightbox.stop_slideshow();
                self.go(i);
            }
        }
    }

    fn rotate(&self) {
        saved().rotate(&self.id());
        self.show();
    }

    fn save_rotation(self: &Rc<Self>) {
        let (path, id) = (self.current(), self.id());
        let rotation = saved().rotation(&id);
        if rotation == 0 {
            return;
        }
        self.toast("Saving the rotation…", None);
        let me = self.clone();
        let file = path.clone();
        actions::save_rotation(&file, rotation, move |result| match result {
            Ok(saved_file) => {
                saved().set_rotation(&id, 0);
                me.items.borrow_mut().remove(&path);
                me.show();
                me.toast(actions::rotation_saved_message(&saved_file), None);
            }
            Err(e) => me.toast(&format!("Couldn't save the rotation: {e:#}"), None),
        });
    }

    fn toggle_favorite(&self) {
        let mut lib = saved();
        match lib.toggle_favorite(&self.id()) {
            Ok(on) => {
                self.lightbox.set_favorite(on);
                self.toast(if on { "Added to favorites" } else { "Removed from favorites" }, None);
            }
            Err(e) => self.toast(&format!("Couldn't save favorites: {e}"), None),
        }
    }

    fn toggle_slideshow(&self) {
        if self.lightbox.is_playing() {
            self.lightbox.stop_slideshow();
        } else {
            self.lightbox.start_slideshow(nexus_photos::prefs::get().slideshow_seconds);
        }
    }

    fn set_background(&self) {
        self.toast(actions::set_background(&self.current().to_string_lossy()), None);
    }

    fn copy(self: &Rc<Self>) {
        let me = self.clone();
        let path = self.current().to_string_lossy().to_string();
        actions::copy(&self.window.clipboard(), vec![path], true, move |m| me.toast(&m, None));
    }

    fn reveal(&self) {
        library::reveal(&self.current().to_string_lossy());
    }

    /// Move the photo to the trash and show the next one. The message can undo it.
    fn delete(self: &Rc<Self>) {
        let path = self.current();
        let text = path.to_string_lossy().to_string();
        let (gone, failed) = actions::trash(std::slice::from_ref(&text));
        if gone.is_empty() {
            self.toast(&actions::trash_message(0, failed), None);
            return;
        }
        let id = self.id();
        let mut lib = saved();
        let membership = lib.membership(std::slice::from_ref(&id));
        let _ = lib.remove_photos(std::slice::from_ref(&id));
        let at = self.index.get();
        self.files.borrow_mut().remove(at);
        self.show();
        let me = self.clone();
        let undo: Rc<dyn Fn()> = Rc::new(move || me.undo_delete(&path, at, &membership));
        self.toast(&actions::trash_message(1, 0), Some(("Undo", undo)));
    }

    fn undo_delete(self: &Rc<Self>, path: &Path, at: usize, membership: &library::Membership) {
        let text = path.to_string_lossy().to_string();
        let (me, path, membership) = (self.clone(), path.to_path_buf(), membership.clone());
        cmd::background(
            move || library::untrash(&[text]),
            move |restored| {
                if restored.is_empty() {
                    me.toast("Couldn't find the file in the trash", None);
                    return;
                }
                let _ = saved().restore(&membership);
                let at = at.min(me.files.borrow().len());
                me.files.borrow_mut().insert(at, path);
                me.go(at);
                me.toast("Put back the photo", None);
            },
        );
    }

    fn rename(self: &Rc<Self>) {
        let path = self.current();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let me = self.clone();
        dialog::ask_text("Rename", "The file is renamed in its folder.", &name, "Rename", move |new| {
            let mut lib = saved();
            let Some(id) = lib.add_or_refresh(&path) else { return };
            match lib.rename(&id, &new) {
                Ok(_) => {
                    let renamed = path.with_file_name(new.trim());
                    me.items.borrow_mut().remove(&path);
                    me.files.borrow_mut()[me.index.get()] = renamed;
                    me.show();
                }
                Err(e) => me.toast(&format!("Couldn't rename: {e:#}"), None),
            }
        });
    }

    /// Hand the photo to the gallery (optionally straight into its editor) and close.
    fn open_in_gallery(self: &Rc<Self>, edit: bool) {
        if !cmd::present(GALLERY) {
            self.toast("Nexus Photos isn't installed", None);
            return;
        }
        let mut command = std::process::Command::new(GALLERY);
        if edit {
            command.arg("--edit");
        }
        command.arg(self.current()).stdin(std::process::Stdio::null());
        match command.spawn() {
            Ok(_) => self.app.quit(),
            Err(e) => self.toast(&format!("Couldn't start Nexus Photos: {e}"), None),
        }
    }

    /// A message at the bottom, with an optional button (Undo).
    fn toast(&self, message: &str, action: Option<nexus_photos::widgets::ButtonAction>) {
        let overlay = self.overlay.clone();
        if let Some(old) = self.toast.borrow_mut().take() {
            overlay.remove_overlay(&old);
        }
        let bx = hbox(12);
        bx.add_css_class("toast");
        let l = gtk::Label::new(Some(message));
        l.set_wrap(true);
        bx.append(&l);
        let revealer = gtk::Revealer::builder().child(&bx).halign(gtk::Align::Center).valign(gtk::Align::End).build();
        revealer.set_margin_bottom(84);
        revealer.set_can_target(action.is_some());
        let stay = if action.is_some() { 8000 } else { 3500 };
        if let Some((text, run)) = action {
            let b = gtk::Button::with_label(text);
            b.add_css_class("toast-action");
            let (r, o) = (revealer.clone(), overlay.clone());
            b.connect_clicked(move |_| {
                run();
                if r.parent().is_some() {
                    o.remove_overlay(&r);
                }
            });
            bx.append(&b);
        }
        overlay.add_overlay(&revealer);
        revealer.set_reveal_child(true);
        *self.toast.borrow_mut() = Some(revealer.clone());
        glib::timeout_add_local_once(Duration::from_millis(stay), move || {
            if revealer.parent().is_some() {
                overlay.remove_overlay(&revealer);
            }
        });
    }
}

/// What the filmstrip needs for a file.
fn cell_for(path: &Path) -> CellData {
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    CellData {
        thumb: thumbnails::cache_path(path, mtime).to_string_lossy().to_string(),
        source: path.to_string_lossy().to_string(),
        mtime,
        favorite: false,
        badge: None,
        video: false,
        aspect: 1.0,
    }
}

/// Details for a file that couldn't be read: just its name.
fn fallback_item(path: &Path) -> PhotoItem {
    let text = path.to_string_lossy().to_string();
    PhotoItem {
        id: photo_id(&text),
        file_name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        path: text,
        file_size: 0,
        file_size_formatted: String::new(),
        format: String::new(),
        taken: 0,
        date_display: String::new(),
        year: 0,
        month: 0,
        day: 0,
        width: 0,
        height: 0,
        thumbnail_path: String::new(),
        mtime: 0,
        added: 0,
        camera_make: None,
        camera_model: None,
        lens_model: None,
        focal_length: None,
        aperture: None,
        shutter_speed: None,
        iso: None,
        latitude: None,
        longitude: None,
        duration: None,
        is_favorite: false,
        media_type: "photos".into(),
    }
}
