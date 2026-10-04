//! The photo info drawer: a preview, file and camera details, the location and the
//! albums the photo is in. What to do with a photo is in the toolbar, not here.

use crate::models::PhotoItem;
use crate::widgets::{self, hbox, label, vbox};
use gtk::pango;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub const WIDTH: i32 = 280;

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
    /// The thumbnail, for the preview's histogram.
    pub cell: Option<crate::grid::CellData>,
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
        let title = label("Photo info", "menu-heading");
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
        preview.set_content_fit(gtk::ContentFit::Contain);
        preview.set_size_request(-1, 150);
        preview.set_can_shrink(true);
        preview.set_cursor_from_name(Some("pointer"));
        let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
        frame.add_css_class("inspector-preview-frame");
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
        if view.selected > 1 {
            let t = widgets::tag(&format!("{} selected", view.selected));
            t.set_halign(gtk::Align::Start);
            t.set_margin_top(6);
            self.body.append(&t);
        }

        if let Some(cell) = &view.cell
            && !photo.is_video()
        {
            self.body.append(&histogram(cell));
        }

        self.body.append(&label("Details", "group-title"));
        let dims = if photo.width > 0 { format!("{} × {}", photo.width, photo.height) } else { "Unknown".into() };
        let mut rows = vec![
            ("Dimensions", dims),
            ("Size", photo.file_size_formatted.clone()),
            ("Format", photo.format.clone()),
            ("Date", photo.date_display.clone()),
        ];
        if let Some(d) = photo.duration {
            rows.push(("Length", crate::models::format_duration(d)));
        }
        self.body.append(&details(&rows));

        // The folder, as a link that opens it.
        let folder = crate::paths::pretty(std::path::Path::new(photo.folder()));
        let link = gtk::Button::with_label(&folder);
        link.add_css_class("flat");
        link.add_css_class("folder-link");
        link.set_tooltip_text(Some("Show in the file manager (Ctrl+O)"));
        if let Some(l) = link.child().and_downcast::<gtk::Label>() {
            l.set_ellipsize(pango::EllipsizeMode::Start);
            l.set_xalign(0.0);
        }
        let me = self.clone();
        link.connect_clicked(move |_| me.emit(Action::Reveal));
        self.body.append(&link);

        if let (Some(lat), Some(lon)) = (photo.latitude, photo.longitude) {
            self.body.append(&label("Location", "group-title"));
            let text = format!(
                "{:.5}° {}, {:.5}° {}",
                lat.abs(),
                if lat < 0.0 { "S" } else { "N" },
                lon.abs(),
                if lon < 0.0 { "W" } else { "E" }
            );
            self.body.append(&details(&[("Position", text)]));
            let url = format!("https://www.openstreetmap.org/?mlat={lat:.6}&mlon={lon:.6}#map=15/{lat:.6}/{lon:.6}");
            let map = gtk::LinkButton::with_label(&url, "Open in OpenStreetMap");
            map.set_halign(gtk::Align::Start);
            map.add_css_class("map-link");
            self.body.append(&map);
        }

        let device = match (&photo.camera_make, &photo.camera_model) {
            (Some(m), Some(model)) if model.contains(m.as_str()) => Some(model.clone()),
            (Some(m), Some(model)) => Some(format!("{m} {model}")),
            (None, Some(model)) => Some(model.clone()),
            (Some(m), None) => Some(m.clone()),
            _ => None,
        };
        let camera: Vec<(&str, String)> = [
            ("Device", device),
            ("Lens", photo.lens_model.clone()),
            ("Focal length", photo.focal_length.clone()),
            ("Aperture", photo.aperture.clone()),
            ("Shutter", photo.shutter_speed.clone()),
            ("ISO", photo.iso.clone()),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .collect();
        if !camera.is_empty() {
            self.body.append(&label("Camera", "group-title"));
            self.body.append(&details(&camera));
        }

        if !view.albums.is_empty() {
            self.body.append(&label("In albums", "group-title"));
            let list = vbox(4);
            for (id, name) in &view.albums {
                let row = hbox(8);
                row.add_css_class("album-row");
                row.append(&gtk::Image::from_icon_name("folder-symbolic"));
                let l = label(name, "");
                l.set_hexpand(true);
                l.set_ellipsize(pango::EllipsizeMode::End);
                row.append(&l);
                let remove = widgets::icon_button("list-remove-symbolic", "Take it out of this album");
                let (me, id) = (self.clone(), id.clone());
                remove.connect_clicked(move |_| me.emit(Action::RemoveFromAlbum(id.clone())));
                row.append(&remove);
                list.append(&row);
            }
            self.body.append(&list);
        }
    }
}

/// How bright the photo is: a curve of how many pixels have each brightness, drawn
/// from its thumbnail.
fn histogram(cell: &crate::grid::CellData) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.add_css_class("histogram");
    area.set_content_height(56);
    area.set_tooltip_text(Some("Brightness, dark to light"));
    let bins: Rc<RefCell<Option<[f64; 64]>>> = Rc::default();
    let b = bins.clone();
    area.set_draw_func(move |area, cr, w, h| {
        let Some(bins) = *b.borrow() else { return };
        let peak = bins.iter().cloned().fold(0.0, f64::max).max(1e-9);
        let fg = area.color();
        let (w, h) = (w as f64, h as f64);
        cr.move_to(0.0, h);
        for (i, v) in bins.iter().enumerate() {
            cr.line_to(i as f64 / 63.0 * w, h - (v / peak).sqrt() * (h - 2.0));
        }
        cr.line_to(w, h);
        cr.close_path();
        cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, 0.3);
        let _ = cr.fill();
    });
    let a = area.clone();
    crate::grid::thumbnail_texture(cell, move |tex| {
        let mut downloader = gtk::gdk::TextureDownloader::new(&tex);
        downloader.set_format(gtk::gdk::MemoryFormat::R8g8b8a8);
        let (bytes, stride) = downloader.download_bytes();
        let mut counts = [0f64; 64];
        let (w, h) = (tex.width() as usize, tex.height() as usize);
        for row in bytes.chunks(stride).take(h) {
            for px in row[..w * 4].as_chunks::<4>().0 {
                let y = 0.2126 * px[0] as f64 + 0.7152 * px[1] as f64 + 0.0722 * px[2] as f64;
                counts[((y / 256.0) * 64.0) as usize] += 1.0;
            }
        }
        *bins.borrow_mut() = Some(counts);
        a.queue_draw();
    });
    area
}

/// Names and values in two aligned columns, in one rounded box.
fn details(rows: &[(&str, String)]) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.add_css_class("detail-grid");
    grid.set_column_spacing(16);
    grid.set_row_spacing(8);
    for (n, (k, v)) in rows.iter().enumerate() {
        let key = label(k, "detail-key");
        key.set_valign(gtk::Align::Start);
        key.set_width_chars(10);
        grid.attach(&key, 0, n as i32, 1, 1);
        let value = label(v, "detail-value mono");
        value.set_hexpand(true);
        value.set_wrap(true);
        value.set_wrap_mode(pango::WrapMode::WordChar);
        value.set_max_width_chars(20);
        value.set_selectable(true);
        value.set_can_focus(false);
        grid.attach(&value, 1, n as i32, 1, 1);
    }
    grid
}
