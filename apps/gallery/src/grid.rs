//! The thumbnail grid. A `ListView` whose rows are either a timeline heading or a
//! row of square thumbnails, so scrolling stays virtualised (only visible rows exist)
//! and headings can sit between groups, which a plain `GridView` can't do.

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Space between thumbnails, and the padding either side of the grid.
const GAP: i32 = 8;
/// Side padding of the grid: the page padding, or the narrow one.
const PAD_WIDE: i32 = 20;
const PAD_NARROW: i32 = 12;
const CACHE_LIMIT: usize = 900;

/// What a thumbnail needs to draw itself.
#[derive(Clone)]
pub struct CellData {
    /// The cached thumbnail's file, and the photo it's made from if it isn't there yet.
    pub thumb: String,
    pub source: String,
    pub mtime: u64,
    pub favorite: bool,
    /// Shown on the thumbnail's corner (the format of a video).
    pub badge: Option<String>,
}

/// A run of items under one heading. An empty `title` means no heading.
#[derive(Clone)]
pub struct Span {
    pub title: String,
    pub sub: String,
    pub start: usize,
    pub end: usize,
}

enum Row {
    Header { title: String, sub: String },
    Cells { start: usize, len: usize },
}

struct CellWidget {
    root: gtk::Overlay,
    pic: gtk::Picture,
    star: gtk::Button,
    check: gtk::Image,
    badge: gtk::Label,
    idx: Rc<Cell<usize>>,
    /// Bumped whenever the cell is given a new photo or recycled.
    token: Rc<Cell<u64>>,
}

struct RowWidget {
    root: gtk::Box,
    header: gtk::Box,
    title: gtk::Label,
    sub: gtk::Label,
    cells: gtk::Box,
    widgets: RefCell<Vec<CellWidget>>,
}

type ClickHandler = Rc<dyn Fn(usize, gdk::ModifierType, i32)>;
type FavoriteHandler = Rc<dyn Fn(usize)>;
/// The thumbnail's index, its widget, and where in the widget the right click landed.
type ContextHandler = Rc<dyn Fn(usize, &gtk::Widget, f64, f64)>;

struct Inner {
    stack: gtk::Stack,
    list: gtk::ListView,
    store: gio::ListStore,
    scroller: gtk::ScrolledWindow,
    items: RefCell<Vec<CellData>>,
    spans: RefCell<Vec<Span>>,
    rows: RefCell<Vec<Row>>,
    cols: Cell<usize>,
    cell: Cell<i32>,
    zoom: Cell<f64>,
    pad: Cell<i32>,
    selected: RefCell<HashSet<usize>>,
    cursor: Cell<Option<usize>>,
    /// Row widgets by pointer, so a selection change can restyle the ones on screen.
    live: RefCell<HashMap<usize, Rc<RowWidget>>>,
    on_click: RefCell<Option<ClickHandler>>,
    on_favorite: RefCell<Option<FavoriteHandler>>,
    on_context: RefCell<Option<ContextHandler>>,
    relayout_queued: Cell<bool>,
    empty_title: gtk::Label,
    empty_text: gtk::Label,
    empty_icon: gtk::Image,
}

#[derive(Clone)]
pub struct Gallery(Rc<Inner>);

thread_local! {
    static TEXTURES: RefCell<HashMap<String, gdk::Texture>> = RefCell::new(HashMap::new());
}

impl Gallery {
    pub fn new() -> Self {
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let selection = gtk::NoSelection::new(Some(store.clone()));
        let factory = gtk::SignalListItemFactory::new();
        let list = gtk::ListView::new(Some(selection), Some(factory.clone()));
        list.add_css_class("gallery-list");
        // Keys are handled by the window, not by the list's own focus movement.
        list.set_focusable(false);
        let scroller = gtk::ScrolledWindow::builder()
            // External, not Never: the rows must not set the window's minimum width, or
            // it could never get narrow enough to lay them out again.
            .hscrollbar_policy(gtk::PolicyType::External)
            // Scrolls with wheel, trackpad and keyboard; no visible scrollbar.
            .vscrollbar_policy(gtk::PolicyType::External)
            .vexpand(true)
            .hexpand(true)
            .child(&list)
            .build();

        let empty = gtk::Box::new(gtk::Orientation::Vertical, 8);
        empty.add_css_class("search-empty");
        empty.set_valign(gtk::Align::Start);
        let empty_icon = gtk::Image::from_icon_name("image-x-generic-symbolic");
        empty_icon.set_pixel_size(48);
        let empty_title = crate::widgets::label("", "search-empty-title");
        empty_title.set_halign(gtk::Align::Center);
        let empty_text = crate::widgets::label("", "dim");
        empty_text.set_halign(gtk::Align::Center);
        empty_text.set_wrap(true);
        empty_text.set_justify(gtk::Justification::Center);
        empty.append(&empty_icon);
        empty.append(&empty_title);
        empty.append(&empty_text);

        let stack = gtk::Stack::new();
        stack.add_named(&scroller, Some("grid"));
        stack.add_named(&empty, Some("empty"));

        let inner = Rc::new(Inner {
            stack,
            list,
            store,
            scroller,
            items: RefCell::default(),
            spans: RefCell::default(),
            rows: RefCell::default(),
            cols: Cell::new(4),
            cell: Cell::new(160),
            zoom: Cell::new(0.4),
            pad: Cell::new(PAD_WIDE),
            selected: RefCell::default(),
            cursor: Cell::new(None),
            live: RefCell::default(),
            on_click: RefCell::new(None),
            on_favorite: RefCell::new(None),
            on_context: RefCell::new(None),
            relayout_queued: Cell::new(false),
            empty_title,
            empty_text,
            empty_icon,
        });
        let gallery = Gallery(inner);
        gallery.wire(&factory);
        gallery
    }

    pub fn widget(&self) -> &gtk::Stack {
        &self.0.stack
    }

    pub fn on_click(&self, f: impl Fn(usize, gdk::ModifierType, i32) + 'static) {
        *self.0.on_click.borrow_mut() = Some(Rc::new(f));
    }

    /// Called with the thumbnail's index when its star is clicked.
    pub fn on_favorite(&self, f: impl Fn(usize) + 'static) {
        *self.0.on_favorite.borrow_mut() = Some(Rc::new(f));
    }

    /// Called when a thumbnail is right-clicked.
    pub fn on_context(&self, f: impl Fn(usize, &gtk::Widget, f64, f64) + 'static) {
        *self.0.on_context.borrow_mut() = Some(Rc::new(f));
    }

    fn wire(&self, factory: &gtk::SignalListItemFactory) {
        let weak = Rc::downgrade(&self.0);
        factory.connect_setup({
            let weak = weak.clone();
            move |_, item| {
                let Some(inner) = weak.upgrade() else { return };
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let row = Rc::new(RowWidget::new());
                inner.live.borrow_mut().insert(row.root.as_ptr() as usize, row.clone());
                item.set_child(Some(&row.root));
            }
        });
        factory.connect_bind({
            let weak = weak.clone();
            move |_, item| {
                let Some(inner) = weak.upgrade() else { return };
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let Some(root) = item.child() else { return };
                let Some(row) = inner.live.borrow().get(&(root.as_ptr() as usize)).cloned() else { return };
                let Some(obj) = item.item().and_downcast::<glib::BoxedAnyObject>() else { return };
                Gallery(inner.clone()).bind(&row, &obj.borrow::<Row>());
            }
        });
        factory.connect_unbind(move |_, item| {
            let Some(inner) = weak.upgrade() else { return };
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let Some(root) = item.child() else { return };
            if let Some(row) = inner.live.borrow().get(&(root.as_ptr() as usize)) {
                for c in row.widgets.borrow().iter() {
                    c.token.set(c.token.get() + 1);
                    c.pic.set_paintable(None::<&gdk::Paintable>);
                }
            }
        });

        // Lay out again when the viewport's width changes.
        let g = self.clone();
        self.0.scroller.hadjustment().connect_page_size_notify(move |_| g.queue_relayout());
    }

    fn bind(&self, row: &Rc<RowWidget>, data: &Row) {
        match data {
            Row::Header { title, sub } => {
                row.root.set_margin_start(self.0.pad.get());
                row.root.set_margin_end(self.0.pad.get());
                row.header.set_visible(true);
                row.cells.set_visible(false);
                row.title.set_text(title);
                row.sub.set_text(sub);
                row.sub.set_visible(!sub.is_empty());
            }
            Row::Cells { start, len } => {
                row.root.set_margin_start(self.0.pad.get());
                row.root.set_margin_end(self.0.pad.get());
                row.header.set_visible(false);
                row.cells.set_visible(true);
                let size = self.0.cell.get();
                let mut widgets = row.widgets.borrow_mut();
                while widgets.len() < *len {
                    let c = self.make_cell();
                    row.cells.append(&c.root);
                    widgets.push(c);
                }
                for (n, c) in widgets.iter().enumerate() {
                    if n >= *len {
                        c.root.set_visible(false);
                        continue;
                    }
                    c.root.set_visible(true);
                    c.root.set_size_request(size, size);
                    c.pic.set_size_request(size, size);
                    let idx = start + n;
                    c.idx.set(idx);
                    self.fill_cell(c, idx);
                }
            }
        }
    }

    fn make_cell(&self) -> CellWidget {
        let root = gtk::Overlay::new();
        root.add_css_class("photo-cell");
        root.set_overflow(gtk::Overflow::Hidden);
        let pic = gtk::Picture::new();
        pic.set_content_fit(gtk::ContentFit::Cover);
        pic.set_can_shrink(true);
        root.set_child(Some(&pic));
        // Appears when the pointer is over the thumbnail (CSS), and stays when it is a favourite.
        let star = gtk::Button::from_icon_name("non-starred-symbolic");
        star.add_css_class("cell-star");
        star.set_halign(gtk::Align::End);
        star.set_valign(gtk::Align::Start);
        star.set_focusable(false);
        root.add_overlay(&star);
        let check = gtk::Image::from_icon_name("object-select-symbolic");
        check.add_css_class("cell-check");
        check.set_halign(gtk::Align::Start);
        check.set_valign(gtk::Align::Start);
        check.set_visible(false);
        root.add_overlay(&check);
        let badge = crate::widgets::tag("");
        badge.add_css_class("cell-badge");
        badge.set_halign(gtk::Align::Start);
        badge.set_valign(gtk::Align::End);
        root.add_overlay(&badge);

        let idx = Rc::new(Cell::new(0usize));
        let click = gtk::GestureClick::new();
        click.set_button(gdk::BUTTON_PRIMARY);
        let (weak, i) = (Rc::downgrade(&self.0), idx.clone());
        click.connect_pressed(move |g, n, _, _| {
            let Some(inner) = weak.upgrade() else { return };
            let handler = inner.on_click.borrow().clone();
            if let Some(h) = handler {
                h(i.get(), g.current_event_state(), n);
            }
        });
        root.add_controller(click);
        let menu = gtk::GestureClick::new();
        menu.set_button(gdk::BUTTON_SECONDARY);
        let (weak, i) = (Rc::downgrade(&self.0), idx.clone());
        menu.connect_pressed(move |g, _, x, y| {
            let Some(inner) = weak.upgrade() else { return };
            let handler = inner.on_context.borrow().clone();
            if let (Some(h), Some(w)) = (handler, g.widget()) {
                h(i.get(), &w, x, y);
            }
        });
        root.add_controller(menu);
        // An invisible star must not take clicks meant for selecting the photo.
        star.set_can_target(false);
        let hover = gtk::EventControllerMotion::new();
        let s = star.clone();
        hover.connect_enter(move |_, _, _| s.set_can_target(true));
        let s = star.clone();
        hover.connect_leave(move |_| s.set_can_target(s.has_css_class("on")));
        root.add_controller(hover);
        let (weak, i) = (Rc::downgrade(&self.0), idx.clone());
        star.connect_clicked(move |_| {
            let Some(inner) = weak.upgrade() else { return };
            let handler = inner.on_favorite.borrow().clone();
            if let Some(h) = handler {
                h(i.get());
            }
        });
        CellWidget { root, pic, star, check, badge, idx, token: Rc::new(Cell::new(0)) }
    }

    fn fill_cell(&self, c: &CellWidget, idx: usize) {
        let items = self.0.items.borrow();
        let Some(data) = items.get(idx) else { return };
        style_star(&c.star, data.favorite);
        c.badge.set_visible(data.badge.is_some());
        if let Some(b) = &data.badge {
            c.badge.set_text(b);
        }
        self.style_cell(c, idx);
        c.token.set(c.token.get() + 1);
        load_texture(&c.pic, data, c.token.clone());
    }

    fn style_cell(&self, c: &CellWidget, idx: usize) {
        let selected = self.0.selected.borrow().contains(&idx);
        let cursor = self.0.cursor.get() == Some(idx);
        toggle_class(&c.root, "selected", selected);
        toggle_class(&c.root, "cursor", cursor);
        c.check.set_visible(selected && self.0.selected.borrow().len() > 1);
    }

    /// Replace what the grid shows. `spans` split `items` into headed groups.
    pub fn set_items(&self, items: Vec<CellData>, spans: Vec<Span>, scroll_top: bool) {
        *self.0.items.borrow_mut() = items;
        *self.0.spans.borrow_mut() = spans;
        self.relayout(scroll_top);
    }

    /// Use the narrow-window padding (or the normal one).
    pub fn set_narrow(&self, narrow: bool) {
        let pad = if narrow { PAD_NARROW } else { PAD_WIDE };
        if self.0.pad.replace(pad) != pad {
            self.queue_relayout();
        }
    }

    pub fn set_zoom(&self, zoom: f64) {
        self.0.zoom.set(zoom);
        self.queue_relayout();
    }

    /// Show a message instead of the grid (an empty section), or the grid again.
    pub fn set_empty(&self, message: Option<(&str, &str, &str)>) {
        match message {
            Some((icon, title, text)) => {
                self.0.empty_icon.set_icon_name(Some(icon));
                self.0.empty_title.set_text(title);
                self.0.empty_text.set_text(text);
                self.0.stack.set_visible_child_name("empty");
            }
            None => self.0.stack.set_visible_child_name("grid"),
        }
    }

    pub fn set_selection(&self, selected: HashSet<usize>, cursor: Option<usize>, reveal: bool) {
        *self.0.selected.borrow_mut() = selected;
        self.0.cursor.set(cursor);
        for row in self.0.live.borrow().values() {
            for c in row.widgets.borrow().iter() {
                self.style_cell(c, c.idx.get());
            }
        }
        if reveal && let Some(i) = cursor {
            self.scroll_to(i);
        }
    }

    /// Update one thumbnail's favourite star without rebuilding the grid.
    pub fn set_favorite(&self, idx: usize, favorite: bool) {
        if let Some(item) = self.0.items.borrow_mut().get_mut(idx) {
            item.favorite = favorite;
        }
        for row in self.0.live.borrow().values() {
            for c in row.widgets.borrow().iter() {
                if c.root.is_visible() && c.idx.get() == idx {
                    style_star(&c.star, favorite);
                }
            }
        }
    }

    fn row_of(&self, idx: usize) -> Option<usize> {
        self.0.rows.borrow().iter().position(|r| matches!(r, Row::Cells { start, len } if idx >= *start && idx < start + len))
    }

    fn scroll_to(&self, idx: usize) {
        let Some(row) = self.row_of(idx) else { return };
        // The first photo: show the heading above it too.
        let first = !self.0.rows.borrow()[..row].iter().any(|r| matches!(r, Row::Cells { .. }));
        self.0.list.scroll_to(if first { 0 } else { row as u32 }, gtk::ListScrollFlags::NONE, None);
    }

    /// The item above or below `idx` in the same column, across group headings.
    pub fn vertical_target(&self, idx: usize, down: bool) -> Option<usize> {
        let rows = self.0.rows.borrow();
        let at = self.row_of(idx)?;
        let Row::Cells { start, .. } = &rows[at] else { return None };
        let col = idx - start;
        let mut r = at as isize;
        loop {
            r += if down { 1 } else { -1 };
            if r < 0 || r as usize >= rows.len() {
                return None;
            }
            if let Row::Cells { start, len } = &rows[r as usize] {
                return Some(start + col.min(len - 1));
            }
        }
    }

    fn queue_relayout(&self) {
        if self.0.relayout_queued.replace(true) {
            return;
        }
        let g = self.clone();
        glib::idle_add_local_once(move || {
            g.0.relayout_queued.set(false);
            g.relayout(false);
        });
    }

    fn relayout(&self, scroll_top: bool) {
        let width = self.0.scroller.hadjustment().page_size() as i32;
        let width = if width > 0 { width } else { self.0.scroller.width().max(800) };
        let avail = (width - 2 * self.0.pad.get()).max(80);
        let target = 110.0 + self.0.zoom.get() * 150.0;
        let cols = (((avail + GAP) as f64 / (target + GAP as f64)).floor() as usize).max(1);
        let cell = ((avail - GAP * (cols as i32 - 1)) / cols as i32).max(40);
        self.0.cols.set(cols);
        self.0.cell.set(cell);

        let mut rows = Vec::new();
        for span in self.0.spans.borrow().iter() {
            if !span.title.is_empty() {
                rows.push(Row::Header { title: span.title.clone(), sub: span.sub.clone() });
            }
            let mut at = span.start;
            while at < span.end {
                let len = cols.min(span.end - at);
                rows.push(Row::Cells { start: at, len });
                at += len;
            }
        }
        let objs: Vec<glib::BoxedAnyObject> = rows.iter().map(row_object).collect();
        // The rows are described twice: for the list, and for navigation.
        *self.0.rows.borrow_mut() = rows;
        let n = self.0.store.n_items();
        self.0.store.splice(0, n, &objs);
        if scroll_top {
            self.0.scroller.vadjustment().set_value(0.0);
        } else if let Some(i) = self.0.cursor.get() {
            self.scroll_to(i);
        }
    }
}

fn row_object(row: &Row) -> glib::BoxedAnyObject {
    glib::BoxedAnyObject::new(match row {
        Row::Header { title, sub } => Row::Header { title: title.clone(), sub: sub.clone() },
        Row::Cells { start, len } => Row::Cells { start: *start, len: *len },
    })
}

impl RowWidget {
    fn new() -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        header.add_css_class("gallery-header");
        let title = crate::widgets::label("", "group-title gallery-title");
        let sub = crate::widgets::label("", "dim gallery-sub");
        sub.set_valign(gtk::Align::Baseline);
        title.set_valign(gtk::Align::Baseline);
        header.append(&title);
        header.append(&sub);
        let cells = gtk::Box::new(gtk::Orientation::Horizontal, GAP);
        cells.set_margin_bottom(GAP);
        root.append(&header);
        root.append(&cells);
        RowWidget { root, header, title, sub, cells, widgets: RefCell::new(Vec::new()) }
    }
}

/// A filled, always-visible star for a favourite; an outline that shows on hover otherwise.
fn style_star(star: &gtk::Button, favorite: bool) {
    star.set_icon_name(if favorite { "starred-symbolic" } else { "non-starred-symbolic" });
    star.set_tooltip_text(Some(if favorite { "Remove from favorites (F)" } else { "Add to favorites (F)" }));
    toggle_class(star, "on", favorite);
    // A favourite's star is always visible, so it can always be clicked; others only on hover.
    star.set_can_target(favorite);
}

fn toggle_class(w: &impl IsA<gtk::Widget>, class: &str, on: bool) {
    if on {
        w.add_css_class(class);
    } else {
        w.remove_css_class(class);
    }
}

/// Put a photo's thumbnail into `pic`, making and decoding it off the UI thread. A change
/// of `token` while that runs means the cell was recycled for another photo: drop it.
fn load_texture(pic: &gtk::Picture, data: &CellData, token: Rc<Cell<u64>>) {
    if let Some(tex) = TEXTURES.with(|t| t.borrow().get(&data.thumb).cloned()) {
        pic.set_paintable(Some(&tex));
        return;
    }
    pic.set_paintable(None::<&gdk::Paintable>);
    let (pic, mine) = (pic.clone(), token.get());
    let (thumb, source, mtime) = (data.thumb.clone(), data.source.clone(), data.mtime);
    glib::spawn_future_local(async move {
        let key = thumb.clone();
        let tex = gio::spawn_blocking(move || {
            let file = if std::path::Path::new(&thumb).exists() {
                thumb
            } else {
                crate::thumbnails::generate_thumbnail(std::path::Path::new(&source), mtime)
            };
            gdk::Texture::from_filename(file).ok()
        })
        .await
        .ok()
        .flatten();
        let Some(tex) = tex else { return };
        TEXTURES.with(|t| {
            let mut t = t.borrow_mut();
            if t.len() >= CACHE_LIMIT {
                t.clear();
            }
            t.insert(key, tex.clone());
        });
        if token.get() == mine {
            pic.set_paintable(Some(&tex));
        }
    });
}
