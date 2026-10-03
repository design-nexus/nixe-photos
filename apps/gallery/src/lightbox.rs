//! The full-window photo viewer: a scrim over the gallery with the picture,
//! zoom and pan, and the few actions that make sense while looking at one photo.

use crate::models::PhotoItem;
use crate::stage::Stage;
use crate::widgets::{self, hbox, label};
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

type Handler = Rc<dyn Fn(Action)>;

#[derive(Clone, Copy)]
pub enum Action {
    Close,
    Prev,
    Next,
    Rotate,
    Reveal,
    ToggleFavorite,
    Edit,
    Play,
}

struct Inner {
    root: gtk::Box,
    stage: Stage,
    name: gtk::Label,
    position: gtk::Label,
    play: gtk::Button,
    favorite: gtk::Button,
    on_action: RefCell<Option<Handler>>,
}

#[derive(Clone)]
pub struct Lightbox(Rc<Inner>);

impl Lightbox {
    pub fn new() -> Self {
        let root = widgets::vbox(0);
        root.add_css_class("lightbox");
        root.set_visible(false);

        // Top bar
        let top = hbox(8);
        top.add_css_class("lightbox-bar");
        let back = gtk::Button::new();
        back.set_child(Some(&widgets::button_content("go-previous-symbolic", "Back to gallery")));
        back.set_tooltip_text(Some("Esc"));
        top.append(&back);
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
        let favorite = widgets::icon_button("non-starred-symbolic", "Favorite (F)");
        let edit = widgets::icon_button("document-edit-symbolic", "Edit (E)");
        let rotate = widgets::icon_button("object-rotate-right-symbolic", "Rotate 90° (R)");
        let reveal = widgets::icon_button("folder-open-symbolic", "Reveal in file manager (Ctrl+O)");
        let close = widgets::icon_button("window-close-symbolic", "Close (Esc)");
        for b in [&favorite, &edit, &rotate, &reveal, &close] {
            top.append(b);
        }
        root.append(&top);

        let stage = Stage::new(zoom_label.clone());
        let prev = nav_button("go-previous-symbolic", "Previous (←)", gtk::Align::Start);
        let next = nav_button("go-next-symbolic", "Next (→)", gtk::Align::End);
        let overlay = stage.widget();
        overlay.add_overlay(&prev);
        overlay.add_overlay(&next);
        let play = gtk::Button::new();
        play.set_child(Some(&widgets::button_content("media-playback-start-symbolic", "Play video")));
        play.add_css_class("suggested-action");
        play.set_halign(gtk::Align::Center);
        play.set_valign(gtk::Align::Center);
        play.set_visible(false);
        overlay.add_overlay(&play);
        root.append(overlay);

        let inner = Rc::new(Inner { root, stage, name, position, play, favorite, on_action: RefCell::new(None) });
        let lb = Lightbox(inner);
        for (b, a) in [
            (&back, Action::Close),
            (&close, Action::Close),
            (&prev, Action::Prev),
            (&next, Action::Next),
            (&edit, Action::Edit),
            (&rotate, Action::Rotate),
            (&reveal, Action::Reveal),
            (&lb.0.favorite, Action::ToggleFavorite),
            (&lb.0.play, Action::Play),
        ] {
            let me = lb.clone();
            b.connect_clicked(move |_| me.emit(a));
        }
        lb
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.0.root
    }

    pub fn is_open(&self) -> bool {
        self.0.root.is_visible()
    }

    pub fn on_action(&self, f: impl Fn(Action) + 'static) {
        *self.0.on_action.borrow_mut() = Some(Rc::new(f));
    }

    fn emit(&self, a: Action) {
        let handler = self.0.on_action.borrow().clone();
        if let Some(h) = handler {
            h(a);
        }
    }

    pub fn hide(&self) {
        self.0.stage.clear();
        self.0.root.set_visible(false);
    }

    /// Show `photo` (number `n` of `total`), turned by `rotation` degrees.
    pub fn show(&self, photo: &PhotoItem, rotation: u32, n: usize, total: usize) {
        let i = &self.0;
        i.root.set_visible(true);
        i.name.set_text(&photo.file_name);
        i.position.set_text(&format!("{n} / {total}"));
        self.set_favorite(photo.is_favorite);
        let video = photo.media_type == "videos";
        i.play.set_visible(video);
        i.stage.show(photo.preview_path(), (!video).then_some(photo.path.as_str()), rotation);
    }

    pub fn set_favorite(&self, on: bool) {
        self.0.favorite.set_icon_name(if on { "starred-symbolic" } else { "non-starred-symbolic" });
    }

    pub fn zoom_by(&self, factor: f64) {
        self.0.stage.zoom_by(factor);
    }

    pub fn zoom_fit(&self) {
        self.0.stage.zoom_fit();
    }

    pub fn zoom_actual(&self) {
        self.0.stage.zoom_actual();
    }
}

fn nav_button(icon: &str, tip: &str, side: gtk::Align) -> gtk::Button {
    let b = gtk::Button::from_icon_name(icon);
    b.add_css_class("lightbox-nav");
    b.set_tooltip_text(Some(tip));
    b.set_halign(side);
    b.set_valign(gtk::Align::Center);
    b
}
