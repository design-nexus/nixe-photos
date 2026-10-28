//! The photo info drawer: a preview, file and camera details, the location and the
//! albums the photo is in. What to do with a photo is in the toolbar, not here.

use crate::models::PhotoItem;
use crate::widgets::{self, hbox, label, vbox};
use gtk::pango;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub const WIDTH: i32 = 288;

type Handler = Rc<dyn Fn(Action)>;

#[derive(Clone)]
pub enum Action {
    Close,
    Open,
    Reveal,
    /// Take the photo out of this album.
    RemoveFromAlbum(String),
}

/// Everything the drawer needs to draw itself.
pub struct View {
    pub photo: Option<PhotoItem>,
    pub selected: usize,
    /// The albums the photo is in: (id, name).
    pub albums: Vec<(String, String)>,
}

#[derive(Clone)]
pub struct Inspector {
    pub root: gtk::Box,
    body: gtk::Box,
    on_action: Rc<RefCell<Option<Handler>>>,
}

impl Inspector {
    pub fn new() -> Self {
        let root = vbox(0);
        root.add_css_class("inspector");
        root.set_size_request(WIDTH, -1);
        let head = hbox(8);
        head.add_css_class("inspector-head");
        let title = label("Info", "inspector-title");
        title.set_hexpand(true);
        head.append(&title);
        let close = widgets::icon_button("window-close-symbolic", "Close (I)");
        head.append(&close);
        root.append(&head);
        let body = vbox(0);
        body.add_css_class("inspector-body");
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .vexpand(true)
            .child(&body)
            .build();
        root.append(&scroll);
        let inspector = Inspector { root, body, on_action: Rc::default() };
        let i = inspector.clone();
        close.connect_clicked(move |_| i.emit(Action::Close));
        inspector
    }

    pub fn on_action(&self, f: impl Fn(Action) + 'static) {
        *self.on_action.borrow_mut() = Some(Rc::new(f));
    }

    fn emit(&self, action: Action) {
        let handler = self.on_action.borrow().clone();
        if let Some(h) = handler {
            h(action);
        }
    }

    pub fn update(&self, view: &View) {
        while let Some(c) = self.body.first_child() {
            self.body.remove(&c);
        }
        let Some(photo) = &view.photo else {
            let l = label("Select a photo to see its details.", "dim");
            l.set_wrap(true);
            self.body.append(&l);
            return;
        };

        let preview = gtk::Picture::for_filename(photo.preview_path());
        preview.set_content_fit(gtk::ContentFit::Cover);
        preview.set_size_request(-1, 150);
        preview.set_can_shrink(true);
        preview.set_cursor_from_name(Some("pointer"));
        preview.set_tooltip_text(Some("Open (Enter)"));
        let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
        frame.add_css_class("inspector-preview-frame");
        frame.set_overflow(gtk::Overflow::Hidden);
        frame.append(&preview);
        let click = gtk::GestureClick::new();
        let me = self.clone();
        click.connect_pressed(move |_, n, _, _| {
            if n >= 1 {
                me.emit(Action::Open);
            }
        });
        frame.add_controller(click);
        self.body.append(&frame);

        let name = label(&photo.file_name, "inspector-name");
        name.set_ellipsize(pango::EllipsizeMode::Middle);
        name.set_tooltip_text(Some(&photo.file_name));
        self.body.append(&name);
        self.body.append(&label(&taken_line(photo.taken), "inspector-sub"));
        if view.selected > 1 {
            let t = widgets::tag(&format!("{} selected", view.selected));
            t.set_halign(gtk::Align::Start);
            t.set_margin_top(6);
            self.body.append(&t);
        }

        // One card: the camera, the picture, where it was taken, and its folder.
        let card = vbox(0);
        card.add_css_class("info-card");
        let device = match (&photo.camera_make, &photo.camera_model) {
            (Some(m), Some(model)) if model.contains(m.as_str()) => Some(model.clone()),
            (Some(m), Some(model)) => Some(format!("{m} {model}")),
            (None, Some(model)) => Some(model.clone()),
            (Some(m), None) => Some(m.clone()),
            _ => None,
        };
        let exposure: Vec<String> = [
            photo.focal_length.clone(),
            photo.aperture.clone(),
            photo.shutter_speed.clone(),
            photo.iso.as_ref().map(|i| format!("ISO {i}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        if device.is_some() || !exposure.is_empty() {
            let lens = photo.lens_model.clone().into_iter().chain(Some(exposure.join(" · "))).filter(|s| !s.is_empty());
            card.append(&info_row(
                "camera-photo-symbolic",
                device.as_deref().unwrap_or("Camera"),
                &lens.collect::<Vec<_>>().join("\n"),
            ));
        }
        let mut picture = Vec::new();
        if photo.width > 0 {
            let mp = photo.width as f64 * photo.height as f64 / 1_000_000.0;
            picture.push(format!("{mp:.1} MP"));
        }
        picture.push(photo.file_size_formatted.clone());
        picture.push(photo.format.clone());
        if let Some(d) = photo.duration {
            picture.push(crate::models::format_duration(d));
        }
        let dims = if photo.width > 0 { format!("{} × {}", photo.width, photo.height) } else { "Unknown size".into() };
        card.append(&info_row("image-x-generic-symbolic", &dims, &picture.join(" · ")));
        if let (Some(lat), Some(lon)) = (photo.latitude, photo.longitude) {
            let text = format!(
                "{:.2}° {}, {:.2}° {}",
                lat.abs(),
                if lat < 0.0 { "S" } else { "N" },
                lon.abs(),
                if lon < 0.0 { "W" } else { "E" }
            );
            let row = info_row("mark-location-symbolic", &text, "Open in OpenStreetMap");
            let url = format!("https://www.openstreetmap.org/?mlat={lat:.6}&mlon={lon:.6}#map=15/{lat:.6}/{lon:.6}");
            let link = gtk::LinkButton::new(&url);
            link.set_child(Some(&row));
            link.add_css_class("info-row-button");
            card.append(&link);
        }
        let folder_path = std::path::Path::new(photo.folder());
        let folder = folder_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into());
        let row = info_row("folder-symbolic", &folder, "Show in Files");
        row.set_tooltip_text(Some(&crate::paths::pretty(folder_path)));
        let reveal = gtk::Button::new();
        reveal.set_child(Some(&row));
        reveal.add_css_class("info-row-button");
        reveal.set_tooltip_text(Some("Show in the file manager (Ctrl+O)"));
        let me = self.clone();
        reveal.connect_clicked(move |_| me.emit(Action::Reveal));
        card.append(&reveal);
        self.body.append(&card);

        // The albums it's in, as chips; the cross takes it out of one.
        if !view.albums.is_empty() {
            let chips = gtk::FlowBox::new();
            chips.add_css_class("album-chips");
            chips.set_selection_mode(gtk::SelectionMode::None);
            chips.set_max_children_per_line(6);
            chips.set_column_spacing(6);
            chips.set_row_spacing(6);
            for (id, name) in &view.albums {
                let chip = hbox(4);
                chip.add_css_class("info-chip");
                let l = label(name, "");
                l.set_ellipsize(pango::EllipsizeMode::End);
                l.set_max_width_chars(18);
                chip.append(&l);
                let remove = gtk::Button::from_icon_name("window-close-symbolic");
                remove.add_css_class("chip-remove");
                remove.set_tooltip_text(Some("Take it out of this album"));
                let (me, id) = (self.clone(), id.clone());
                remove.connect_clicked(move |_| me.emit(Action::RemoveFromAlbum(id.clone())));
                chip.append(&remove);
                chips.insert(&chip, -1);
            }
            self.body.append(&chips);
        }
    }
}

/// When it was taken: "Saturday, Oct 18, 2026 · 17:42" (wall-clock seconds).
fn taken_line(taken: i64) -> String {
    const WEEKDAYS: [&str; 7] = ["Thursday", "Friday", "Saturday", "Sunday", "Monday", "Tuesday", "Wednesday"];
    let (y, m, d) = crate::metadata::ymd(taken);
    let seconds = taken.rem_euclid(86400);
    format!(
        "{}, {} {d}, {y} · {:02}:{:02}",
        WEEKDAYS[taken.div_euclid(86400).rem_euclid(7) as usize],
        crate::metadata::month_abbr(m),
        seconds / 3600,
        seconds % 3600 / 60
    )
}

/// A card row: an icon, then a line and the quieter lines under it.
fn info_row(icon: &str, title: &str, sub: &str) -> gtk::Box {
    let row = hbox(12);
    row.add_css_class("info-row");
    let image = gtk::Image::from_icon_name(icon);
    image.set_valign(gtk::Align::Start);
    image.add_css_class("info-icon");
    row.append(&image);
    let text = vbox(2);
    let t = label(title, "info-title");
    t.set_xalign(0.0);
    t.set_wrap(true);
    t.set_wrap_mode(pango::WrapMode::WordChar);
    text.append(&t);
    if !sub.is_empty() {
        let s = label(sub, "info-sub");
        s.set_xalign(0.0);
        s.set_wrap(true);
        s.set_wrap_mode(pango::WrapMode::WordChar);
        text.append(&s);
    }
    row.append(&text);
    row
}
