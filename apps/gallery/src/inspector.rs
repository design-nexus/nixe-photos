//! The photo info drawer: a preview, file and camera details, and actions.

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
    Edit,
    Rotate,
    CopyPath,
    Reveal,
    ToggleFavorite,
    AddToAlbum(String),
    RemoveFromAlbum,
}

/// Everything the drawer needs to draw itself.
pub struct View {
    pub photo: Option<PhotoItem>,
    pub selected: usize,
    pub albums: Vec<(String, String)>,
    /// The id of the album being viewed, if any.
    pub in_album: bool,
    pub rotation: u32,
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

        self.body.append(&label("Details", "group-title"));
        let dims = if photo.width > 0 { format!("{} × {}", photo.width, photo.height) } else { "Unknown".into() };
        self.body.append(&details(&[
            ("Dimensions", dims),
            ("Size", photo.file_size_formatted.clone()),
            ("Format", photo.format.clone()),
            ("Date", photo.date_display.clone()),
        ]));

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

        self.body.append(&label("Actions", "group-title"));
        let actions = vbox(4);
        actions.append(&self.action_button("document-edit-symbolic", "Edit photo", "E", Action::Edit));
        let fav = if photo.is_favorite { "Remove from favorites" } else { "Add to favorites" };
        let icon = if photo.is_favorite { "starred-symbolic" } else { "non-starred-symbolic" };
        actions.append(&self.action_button(icon, fav, "F", Action::ToggleFavorite));
        let rot = if view.rotation == 0 { "Rotate".to_string() } else { format!("Rotate ({}°)", view.rotation) };
        actions.append(&self.action_button("object-rotate-right-symbolic", &rot, "R", Action::Rotate));
        actions.append(&self.action_button("edit-copy-symbolic", "Copy path", "", Action::CopyPath));
        let reveal = self.action_button("folder-open-symbolic", "Reveal in file manager", "", Action::Reveal);
        reveal.set_tooltip_text(Some("Ctrl+O"));
        actions.append(&reveal);
        self.body.append(&actions);

        if !view.albums.is_empty() {
            self.body.append(&label("Add to album", "group-title"));
            let flow = gtk::FlowBox::new();
            flow.set_selection_mode(gtk::SelectionMode::None);
            flow.set_min_children_per_line(1);
            flow.set_max_children_per_line(3);
            flow.set_row_spacing(4);
            flow.set_column_spacing(4);
            flow.set_homogeneous(false);
            flow.add_css_class("album-chips");
            for (id, name) in &view.albums {
                let b = gtk::Button::with_label(name);
                b.add_css_class("chip");
                if let Some(l) = b.child().and_downcast::<gtk::Label>() {
                    l.set_ellipsize(pango::EllipsizeMode::End);
                    l.set_max_width_chars(16);
                }
                let (me, id) = (self.clone(), id.clone());
                b.connect_clicked(move |_| me.emit(Action::AddToAlbum(id.clone())));
                flow.append(&b);
            }
            self.body.append(&flow);
        }
        if view.in_album {
            let b = gtk::Button::with_label("Remove from this album");
            b.set_margin_top(12);
            let me = self.clone();
            b.connect_clicked(move |_| me.emit(Action::RemoveFromAlbum));
            self.body.append(&b);
        }
    }

    fn action_button(&self, icon: &str, text: &str, keys: &str, action: Action) -> gtk::Button {
        let b = gtk::Button::new();
        b.add_css_class("action-button");
        let content = hbox(8);
        content.append(&gtk::Image::from_icon_name(icon));
        let l = gtk::Label::new(Some(text));
        l.set_xalign(0.0);
        l.set_hexpand(true);
        l.set_ellipsize(pango::EllipsizeMode::End);
        content.append(&l);
        if !keys.is_empty() {
            content.append(&widgets::key_combo(keys));
        }
        b.set_child(Some(&content));
        let me = self.clone();
        b.connect_clicked(move |_| me.emit(action.clone()));
        b
    }
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
