//! The picture area shared by the gallery's viewer and the stand-alone viewer: the photo
//! fitted to the window, or zoomed and panned, with wheel, pinch and double-click zoom.

use gtk::gdk_pixbuf::{Pixbuf, PixbufRotation};
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::Cell;
use std::rc::Rc;

use crate::widgets::label;

/// Full-size pictures decoded ahead (the photos either side of the one shown).
const PRELOAD_KEEP: usize = 4;

thread_local! {
    /// (path, rotation) and the decoded picture, most recent last.
    static FULL: std::cell::RefCell<Vec<((String, u32), gdk::Texture)>> = const { std::cell::RefCell::new(Vec::new()) };
    static LOADING: std::cell::RefCell<std::collections::HashSet<(String, u32)>> = std::cell::RefCell::default();
}

fn cached_full(path: &str, rotation: u32) -> Option<gdk::Texture> {
    FULL.with(|f| f.borrow().iter().find(|(k, _)| k.0 == path && k.1 == rotation).map(|(_, t)| t.clone()))
}

fn remember_full(path: &str, rotation: u32, tex: &gdk::Texture) {
    FULL.with(|f| {
        let mut f = f.borrow_mut();
        f.retain(|(k, _)| !(k.0 == path && k.1 == rotation));
        f.push(((path.to_string(), rotation), tex.clone()));
        while f.len() > PRELOAD_KEEP {
            f.remove(0);
        }
    });
}

/// Decode a photo in the background so showing it later is instant.
pub fn preload(path: &str, rotation: u32) {
    let key = (path.to_string(), rotation);
    if cached_full(path, rotation).is_some() || LOADING.with(|l| !l.borrow_mut().insert(key.clone())) {
        return;
    }
    glib::spawn_future_local(async move {
        let p = key.0.clone();
        let tex = gio::spawn_blocking(move || load_full(&p, rotation)).await.ok().flatten();
        LOADING.with(|l| l.borrow_mut().remove(&key));
        if let Some(tex) = tex {
            remember_full(&key.0, rotation, &tex);
        }
    });
}

struct Inner {
    root: gtk::Overlay,
    stack: gtk::Stack,
    scroller: gtk::ScrolledWindow,
    /// Shown while the photo fits the window; fills it and shrinks to suit.
    fit_pic: gtk::Picture,
    /// Shown when zoomed: sized in pixels, inside the scroller.
    zoom_pic: gtk::Picture,
    loading: gtk::Label,
    zoom_label: gtk::Label,
    /// Pixel size of the picture on screen at 100%; zero until it has loaded.
    size: Cell<(i32, i32)>,
    /// `None` fits the picture to the window.
    scale: Cell<Option<f64>>,
    /// Bumped for every photo shown, so a late load for an earlier one is dropped.
    token: Cell<u64>,
}

#[derive(Clone)]
pub struct Stage(Rc<Inner>);

impl Stage {
    /// `zoom_label` is kept up to date with the zoom percentage (empty while fitted).
    pub fn new(zoom_label: gtk::Label) -> Self {
        // A scroller hands its child the child's natural size, so a picture that
        // should shrink to fit lives outside it; the zoomed one is inside.
        let fit_pic = gtk::Picture::new();
        fit_pic.set_content_fit(gtk::ContentFit::Contain);
        fit_pic.set_can_shrink(true);
        fit_pic.add_css_class("lightbox-picture");
        let zoom_pic = gtk::Picture::new();
        // Contain, not Fill: the scroller may hand this picture a box wider or taller than the
        // zoomed size, and Fill would stretch the photo to it.
        zoom_pic.set_content_fit(gtk::ContentFit::Contain);
        zoom_pic.set_can_shrink(true);
        zoom_pic.set_halign(gtk::Align::Center);
        zoom_pic.set_valign(gtk::Align::Center);
        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::External)
            .vscrollbar_policy(gtk::PolicyType::External)
            .hexpand(true)
            .vexpand(true)
            .child(&zoom_pic)
            .build();
        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.add_named(&fit_pic, Some("fit"));
        stack.add_named(&scroller, Some("zoom"));
        let root = gtk::Overlay::new();
        root.set_child(Some(&stack));
        let loading = label("Loading full-size preview…", "dim lightbox-loading");
        loading.set_halign(gtk::Align::Center);
        loading.set_valign(gtk::Align::End);
        loading.set_can_target(false);
        loading.set_visible(false);
        root.add_overlay(&loading);

        let stage = Stage(Rc::new(Inner {
            root,
            stack,
            scroller,
            fit_pic,
            zoom_pic,
            loading,
            zoom_label,
            size: Cell::new((0, 0)),
            scale: Cell::new(None),
            token: Cell::new(0),
        }));
        stage.wire_gestures();
        stage
    }

    /// The overlay holding the picture; callers add their own controls over it.
    pub fn widget(&self) -> &gtk::Overlay {
        &self.0.root
    }

    /// Drop the picture and ignore any load still running.
    pub fn clear(&self) {
        self.0.token.set(self.0.token.get() + 1);
        self.set_paintable(None);
    }

    /// Show the cached `preview` image now and, when `full` is a path, the whole picture
    /// (turned by `rotation` degrees clockwise) once it has decoded.
    pub fn show(&self, preview: &str, full: Option<&str>, rotation: u32) {
        let i = &self.0;
        i.token.set(i.token.get() + 1);
        let mine = i.token.get();
        i.scale.set(None);
        self.apply_scale();
        if let Ok(tex) = gdk::Texture::from_filename(preview) {
            self.set_paintable(Some(tex.upcast_ref()));
            i.size.set((0, 0));
        }
        let Some(path) = full else {
            i.loading.set_visible(false);
            return;
        };
        if let Some(tex) = cached_full(path, rotation) {
            i.loading.set_visible(false);
            i.size.set((tex.width(), tex.height()));
            self.set_paintable(Some(tex.upcast_ref()));
            self.apply_scale();
            return;
        }
        i.loading.set_visible(true);
        let (path, me) = (path.to_string(), self.clone());
        glib::spawn_future_local(async move {
            let p = path.clone();
            let tex = gio::spawn_blocking(move || load_full(&p, rotation)).await.ok().flatten();
            if let Some(tex) = &tex {
                remember_full(&path, rotation, tex);
            }
            if me.0.token.get() != mine {
                return;
            }
            me.0.loading.set_visible(false);
            if let Some(tex) = tex {
                me.0.size.set((tex.width(), tex.height()));
                me.set_paintable(Some(tex.upcast_ref()));
                me.apply_scale();
            }
        });
    }

    fn set_paintable(&self, p: Option<&gdk::Paintable>) {
        self.0.fit_pic.set_paintable(p);
        self.0.zoom_pic.set_paintable(p);
    }

    fn fit_scale(&self) -> Option<f64> {
        let (w, h) = self.0.size.get();
        let (vw, vh) = (self.0.stack.width(), self.0.stack.height());
        (w > 0 && h > 0 && vw > 0 && vh > 0).then(|| (vw as f64 / w as f64).min(vh as f64 / h as f64))
    }

    fn apply_scale(&self) {
        let i = &self.0;
        let (w, h) = i.size.get();
        match (i.scale.get(), w > 0) {
            (Some(s), true) => {
                let first = i.stack.visible_child_name().as_deref() != Some("zoom");
                i.zoom_pic.set_size_request((w as f64 * s) as i32, (h as f64 * s) as i32);
                i.stack.set_visible_child_name("zoom");
                i.zoom_label.set_text(&format!("{:.0}%", s * 100.0));
                if first {
                    self.centre_soon();
                }
            }
            _ => {
                i.stack.set_visible_child_name("fit");
                i.zoom_label.set_text("");
            }
        }
    }

    /// Start a freshly zoomed picture centred in the window.
    fn centre_soon(&self) {
        let sc = self.0.scroller.clone();
        glib::idle_add_local_once(move || {
            for adj in [sc.hadjustment(), sc.vadjustment()] {
                adj.set_value((adj.upper() - adj.page_size()) / 2.0);
            }
        });
    }

    pub fn zoom_by(&self, factor: f64) {
        let Some(fit) = self.fit_scale() else { return };
        let now = self.0.scale.get().unwrap_or(fit);
        let next = (now * factor).clamp(fit, 16.0);
        // Back at the fitted size means "fit", so it follows window resizes again.
        self.0.scale.set(if next <= fit * 1.001 { None } else { Some(next) });
        self.apply_scale();
    }

    pub fn zoom_fit(&self) {
        self.0.scale.set(None);
        self.apply_scale();
    }

    pub fn zoom_actual(&self) {
        if self.0.size.get().0 > 0 {
            self.0.scale.set(Some(1.0));
            self.apply_scale();
        }
    }

    fn wire_gestures(&self) {
        let i = &self.0;
        // Ctrl + wheel zooms.
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
        let me = self.clone();
        scroll.connect_scroll(move |c, _, dy| {
            if c.current_event_state().contains(gdk::ModifierType::CONTROL_MASK) {
                me.zoom_by(if dy < 0.0 { 1.15 } else { 1.0 / 1.15 });
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        i.stack.add_controller(scroll);

        // Pinch zooms.
        let pinch = gtk::GestureZoom::new();
        let base = Rc::new(Cell::new(1.0f64));
        let (me, b) = (self.clone(), base.clone());
        pinch.connect_begin(move |_, _| b.set(me.0.scale.get().or_else(|| me.fit_scale()).unwrap_or(1.0)));
        let me = self.clone();
        pinch.connect_scale_changed(move |_, s| {
            if let Some(fit) = me.fit_scale() {
                let next = (base.get() * s).clamp(fit, 16.0);
                me.0.scale.set(if next <= fit * 1.001 { None } else { Some(next) });
                me.apply_scale();
            }
        });
        i.stack.add_controller(pinch);

        // Dragging pans a zoomed picture; a double click toggles fit and 100%.
        let drag = gtk::GestureDrag::new();
        let start = Rc::new(Cell::new((0.0, 0.0)));
        let (sc, s) = (i.scroller.clone(), start.clone());
        drag.connect_drag_begin(move |_, _, _| s.set((sc.hadjustment().value(), sc.vadjustment().value())));
        let sc = i.scroller.clone();
        drag.connect_drag_update(move |_, dx, dy| {
            let (x, y) = start.get();
            sc.hadjustment().set_value(x - dx);
            sc.vadjustment().set_value(y - dy);
        });
        i.scroller.add_controller(drag);
        let click = gtk::GestureClick::new();
        let me = self.clone();
        click.connect_pressed(move |_, n, _, _| {
            if n == 2 {
                if me.0.scale.get().is_some() { me.zoom_fit() } else { me.zoom_actual() }
            }
        });
        i.stack.add_controller(click);
    }
}

/// Decode the whole picture, honour its EXIF orientation, and turn it by `rotation`
/// degrees clockwise. Runs on a worker thread.
fn load_full(path: &str, rotation: u32) -> Option<gdk::Texture> {
    let mut pixbuf = Pixbuf::from_file(path).ok()?;
    if let Some(upright) = pixbuf.apply_embedded_orientation() {
        pixbuf = upright;
    }
    let turn = match rotation % 360 {
        90 => Some(PixbufRotation::Clockwise),
        180 => Some(PixbufRotation::Upsidedown),
        270 => Some(PixbufRotation::Counterclockwise),
        _ => None,
    };
    if let Some(t) = turn {
        pixbuf = pixbuf.rotate_simple(t)?;
    }
    Some(gdk::Texture::for_pixbuf(&pixbuf))
}
