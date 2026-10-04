//! The thumbnail grid. A `ListView` whose rows are either a timeline heading or a
//! row of thumbnails, so scrolling stays virtualised (only visible rows exist)
//! and headings can sit between groups, which a plain `GridView` can't do.
//!
//! Rows are square thumbnails, or (justified) thumbnails that keep each photo's shape
//! and are scaled so every full row fills the width. A date scrubber on the right edge
//! shows where you are and jumps when dragged.

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

/// Space between thumbnails, and the padding either side of the grid.
const GAP: i32 = 8;
/// Side padding of the grid: the page padding, or the narrow one.
const PAD_WIDE: i32 = 20;
const PAD_NARROW: i32 = 12;
/// Decoded thumbnails kept in memory; the least recently used go first.
const CACHE_LIMIT: usize = 1200;
/// About how tall a heading row is, for placing the scrubber's marks.
const HEADER_HEIGHT: i32 = 46;
/// How long the pointer rests on a video before it starts playing.
const PREVIEW_DELAY: Duration = Duration::from_millis(600);
/// The scrubber's width: inside the grid's side padding, and wider (with year labels)
/// while the pointer is on it.
const SCRUB_NARROW: i32 = 18;
const SCRUB_WIDE: i32 = 72;

/// What a thumbnail needs to draw itself.
#[derive(Clone)]
pub struct CellData {
    /// The cached thumbnail's file, and the photo it's made from if it isn't there yet.
    pub thumb: String,
    pub source: String,
    pub mtime: u64,
    pub favorite: bool,
    /// Shown on the thumbnail's corner (a video's length).
    pub badge: Option<String>,
    pub video: bool,
    /// Width over height, upright.
    pub aspect: f64,
}

/// A run of items under one heading. An empty `title` means no heading.
#[derive(Clone, Default)]
pub struct Span {
    pub title: String,
    pub sub: String,
    /// A short label for the scrubber ("May 2024").
    pub short: String,
    /// Per-item labels, for a group with no heading (one label per item, or empty).
    pub shorts: Vec<String>,
    pub start: usize,
    pub end: usize,
}

enum Row {
    Header {
        title: String,
        sub: String,
    },
    /// Items `start..start + widths.len()`, each `widths[n]` wide and `height` tall.
    Cells {
        start: usize,
        height: i32,
        widths: Vec<i32>,
    },
}

impl Row {
    fn cells(&self) -> Option<(usize, usize)> {
        match self {
            Row::Cells { start, widths, .. } => Some((*start, widths.len())),
            Row::Header { .. } => None,
        }
    }

    fn height(&self) -> i32 {
        match self {
            Row::Header { .. } => HEADER_HEIGHT,
            Row::Cells { height, .. } => height + GAP,
        }
    }
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
    /// A video playing under the pointer, and the timer that will start one.
    preview: Rc<RefCell<Option<gtk::MediaFile>>>,
    preview_timer: Rc<RefCell<Option<glib::SourceId>>>,
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
type IndexHandler = Rc<dyn Fn(usize)>;
/// The thumbnail's index, its widget, and where in the widget the right click landed.
type ContextHandler = Rc<dyn Fn(usize, &gtk::Widget, f64, f64)>;
/// The files to hand over when a thumbnail is dragged out.
type DragHandler = Rc<dyn Fn(usize) -> Vec<String>>;
type ZoomHandler = Rc<dyn Fn(f64)>;

struct Inner {
    stack: gtk::Stack,
    list: gtk::ListView,
    store: gio::ListStore,
    scroller: gtk::ScrolledWindow,
    items: RefCell<Vec<CellData>>,
    spans: RefCell<Vec<Span>>,
    rows: RefCell<Vec<Row>>,
    /// The scrubber's label for each row, and each row's estimated top.
    row_labels: RefCell<Vec<String>>,
    row_tops: RefCell<Vec<i32>>,
    zoom: Cell<f64>,
    justified: Cell<bool>,
    pad: Cell<i32>,
    selected: RefCell<HashSet<usize>>,
    cursor: Cell<Option<usize>>,
    /// Row widgets by pointer, so a selection change can restyle the ones on screen.
    live: RefCell<HashMap<usize, Rc<RowWidget>>>,
    on_click: RefCell<Option<ClickHandler>>,
    on_favorite: RefCell<Option<IndexHandler>>,
    on_context: RefCell<Option<ContextHandler>>,
    on_drag: RefCell<Option<DragHandler>>,
    on_zoom: RefCell<Option<ZoomHandler>>,
    relayout_queued: Cell<bool>,
    empty_title: gtk::Label,
    empty_text: gtk::Label,
    empty_icon: gtk::Image,
    empty_action: gtk::Button,
    empty_handler: RefCell<Option<Rc<dyn Fn()>>>,
    scrubber: gtk::DrawingArea,
    bubble: gtk::Label,
    bubble_timer: RefCell<Option<glib::SourceId>>,
    /// The pointer is on the scrubber (or dragging it).
    scrubbing: Cell<bool>,
}

#[derive(Clone)]
pub struct Gallery(Rc<Inner>);

/// Decoded thumbnails by file, with the order they were last used in.
#[derive(Default)]
struct TextureCache {
    map: HashMap<String, gdk::Texture>,
    order: VecDeque<String>,
}

impl TextureCache {
    fn get(&mut self, key: &str) -> Option<gdk::Texture> {
        let tex = self.map.get(key)?.clone();
        if let Some(at) = self.order.iter().rposition(|k| k == key) {
            self.order.remove(at);
        }
        self.order.push_back(key.to_string());
        Some(tex)
    }

    fn insert(&mut self, key: String, tex: gdk::Texture) {
        if self.map.insert(key.clone(), tex).is_none() {
            self.order.push_back(key);
        }
        while self.map.len() > CACHE_LIMIT {
            let Some(old) = self.order.pop_front() else { break };
            self.map.remove(&old);
        }
    }
}

thread_local! {
    static TEXTURES: RefCell<TextureCache> = RefCell::default();
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
            // Scrolls with wheel, trackpad and keyboard; the scrubber stands in for a scrollbar.
            .vscrollbar_policy(gtk::PolicyType::External)
            .vexpand(true)
            .hexpand(true)
            .child(&list)
            .build();

        let scrubber = gtk::DrawingArea::new();
        scrubber.add_css_class("scrubber");
        scrubber.set_halign(gtk::Align::End);
        scrubber.set_valign(gtk::Align::Fill);
        scrubber.set_content_width(SCRUB_NARROW);
        scrubber.set_cursor_from_name(Some("pointer"));
        scrubber.set_tooltip_text(Some("Drag to jump through time"));
        let bubble = crate::widgets::label("", "scrub-bubble");
        bubble.set_halign(gtk::Align::End);
        bubble.set_valign(gtk::Align::Start);
        bubble.set_margin_end(SCRUB_WIDE + 4);
        bubble.set_can_target(false);
        bubble.set_visible(false);
        let grid_box = gtk::Overlay::new();
        grid_box.set_child(Some(&scroller));
        grid_box.add_overlay(&scrubber);
        grid_box.add_overlay(&bubble);

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
        let empty_action = gtk::Button::new();
        empty_action.add_css_class("suggested-action");
        empty_action.set_halign(gtk::Align::Center);
        empty_action.set_margin_top(8);
        empty_action.set_visible(false);
        empty.append(&empty_icon);
        empty.append(&empty_title);
        empty.append(&empty_text);
        empty.append(&empty_action);

        let stack = gtk::Stack::new();
        stack.add_named(&grid_box, Some("grid"));
        stack.add_named(&empty, Some("empty"));

        let inner = Rc::new(Inner {
            stack,
            list,
            store,
            scroller,
            items: RefCell::default(),
            spans: RefCell::default(),
            rows: RefCell::default(),
            row_labels: RefCell::default(),
            row_tops: RefCell::default(),
            zoom: Cell::new(0.4),
            justified: Cell::new(false),
            pad: Cell::new(PAD_WIDE),
            selected: RefCell::default(),
            cursor: Cell::new(None),
            live: RefCell::default(),
            on_click: RefCell::new(None),
            on_favorite: RefCell::new(None),
            on_context: RefCell::new(None),
            on_drag: RefCell::new(None),
            on_zoom: RefCell::new(None),
            relayout_queued: Cell::new(false),
            empty_title,
            empty_text,
            empty_icon,
            empty_action,
            empty_handler: RefCell::new(None),
            scrubber,
            bubble,
            bubble_timer: RefCell::new(None),
            scrubbing: Cell::new(false),
        });
        let gallery = Gallery(inner);
        gallery.wire(&factory);
        gallery.wire_scrubber();
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

    /// Asked for the files to hand over when a thumbnail is dragged to another app.
    pub fn on_drag(&self, f: impl Fn(usize) -> Vec<String> + 'static) {
        *self.0.on_drag.borrow_mut() = Some(Rc::new(f));
    }

    /// Called with a factor when Ctrl + wheel asks for bigger or smaller thumbnails.
    pub fn on_zoom(&self, f: impl Fn(f64) + 'static) {
        *self.0.on_zoom.borrow_mut() = Some(Rc::new(f));
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
                    stop_preview(c);
                    c.pic.set_paintable(None::<&gdk::Paintable>);
                }
            }
        });

        // Lay out again when the viewport's width changes.
        let g = self.clone();
        self.0.scroller.hadjustment().connect_page_size_notify(move |_| g.queue_relayout());

        // Ctrl + wheel changes the thumbnail size.
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&self.0);
        scroll.connect_scroll(move |c, _, dy| {
            if !c.current_event_state().contains(gdk::ModifierType::CONTROL_MASK) {
                return glib::Propagation::Proceed;
            }
            if let Some(h) = weak.upgrade().and_then(|i| i.on_zoom.borrow().clone()) {
                h(if dy < 0.0 { 0.06 } else { -0.06 });
            }
            glib::Propagation::Stop
        });
        self.0.scroller.add_controller(scroll);

        let weak = Rc::downgrade(&self.0);
        self.0.empty_action.connect_clicked(move |_| {
            if let Some(h) = weak.upgrade().and_then(|i| i.empty_handler.borrow().clone()) {
                h();
            }
        });
    }

    fn bind(&self, row: &Rc<RowWidget>, data: &Row) {
        row.root.set_margin_start(self.0.pad.get());
        row.root.set_margin_end(self.0.pad.get());
        match data {
            Row::Header { title, sub } => {
                row.header.set_visible(true);
                row.cells.set_visible(false);
                row.title.set_text(title);
                row.sub.set_text(sub);
                row.sub.set_visible(!sub.is_empty());
            }
            Row::Cells { start, height, widths } => {
                row.header.set_visible(false);
                row.cells.set_visible(true);
                let mut cells = row.widgets.borrow_mut();
                while cells.len() < widths.len() {
                    let c = self.make_cell();
                    row.cells.append(&c.root);
                    cells.push(c);
                }
                for (n, c) in cells.iter().enumerate() {
                    let Some(&w) = widths.get(n) else {
                        c.token.set(c.token.get() + 1);
                        stop_preview(c);
                        c.root.set_visible(false);
                        continue;
                    };
                    c.root.set_visible(true);
                    c.root.set_size_request(w, *height);
                    c.pic.set_size_request(w, *height);
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

        // Drag the photo (or the selection it is part of) out to another app.
        let drag = gtk::DragSource::new();
        drag.set_actions(gdk::DragAction::COPY);
        let (weak, i, p) = (Rc::downgrade(&self.0), idx.clone(), pic.clone());
        drag.connect_prepare(move |src, _, _| {
            let inner = weak.upgrade()?;
            let handler = inner.on_drag.borrow().clone()?;
            let files: Vec<gio::File> = handler(i.get()).iter().map(gio::File::for_path).collect();
            if files.is_empty() {
                return None;
            }
            if let Some(paintable) = p.paintable() {
                src.set_icon(Some(&paintable), 16, 16);
            }
            Some(gdk::ContentProvider::for_value(&gdk::FileList::from_array(&files).to_value()))
        });
        root.add_controller(drag);

        // An invisible star must not take clicks meant for selecting the photo.
        star.set_can_target(false);
        let preview: Rc<RefCell<Option<gtk::MediaFile>>> = Rc::default();
        let preview_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
        let token = Rc::new(Cell::new(0u64));
        let hover = gtk::EventControllerMotion::new();
        let (s, weak, i, p, pv, timer, tok) = (
            star.clone(),
            Rc::downgrade(&self.0),
            idx.clone(),
            pic.clone(),
            preview.clone(),
            preview_timer.clone(),
            token.clone(),
        );
        hover.connect_enter(move |_, _, _| {
            s.set_can_target(true);
            let Some(inner) = weak.upgrade() else { return };
            let source = inner.items.borrow().get(i.get()).filter(|d| d.video).map(|d| d.source.clone());
            let Some(source) = source else { return };
            if crate::prefs::get().reduce_motion {
                return;
            }
            let (p, pv, mine, tok, timer2) = (p.clone(), pv.clone(), tok.get(), tok.clone(), timer.clone());
            let id = glib::timeout_add_local_once(PREVIEW_DELAY, move || {
                timer2.borrow_mut().take();
                if tok.get() != mine {
                    return;
                }
                let media = gtk::MediaFile::for_filename(&source);
                media.set_muted(true);
                media.set_loop(true);
                media.play();
                p.set_paintable(Some(&media));
                *pv.borrow_mut() = Some(media);
            });
            if let Some(old) = timer.borrow_mut().replace(id) {
                old.remove();
            }
        });
        let (s, weak, i, p, pv, timer) =
            (star.clone(), Rc::downgrade(&self.0), idx.clone(), pic.clone(), preview.clone(), preview_timer.clone());
        hover.connect_leave(move |_| {
            s.set_can_target(s.has_css_class("on"));
            if let Some(t) = timer.borrow_mut().take() {
                t.remove();
            }
            if let Some(media) = pv.borrow_mut().take() {
                media.pause();
                let data = weak.upgrade().and_then(|inner| inner.items.borrow().get(i.get()).cloned());
                if let Some(tex) = data.and_then(|d| TEXTURES.with(|t| t.borrow_mut().get(&d.thumb))) {
                    p.set_paintable(Some(&tex));
                }
            }
        });
        root.add_controller(hover);
        let (weak, i) = (Rc::downgrade(&self.0), idx.clone());
        star.connect_clicked(move |_| {
            let Some(inner) = weak.upgrade() else { return };
            let handler = inner.on_favorite.borrow().clone();
            if let Some(h) = handler {
                h(i.get());
            }
        });
        CellWidget { root, pic, star, check, badge, idx, token, preview, preview_timer }
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
        stop_preview(c);
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

    /// Thumbnails that keep each photo's shape (or squares).
    pub fn set_justified(&self, on: bool) {
        if self.0.justified.replace(on) != on {
            self.queue_relayout();
        }
    }

    /// Show a message instead of the grid (an empty section), or the grid again. The
    /// message may carry a button.
    pub fn set_empty(&self, message: Option<(&str, &str, &str)>, action: Option<crate::widgets::ButtonAction>) {
        match message {
            Some((icon, title, text)) => {
                self.0.empty_icon.set_icon_name(Some(icon));
                self.0.empty_title.set_text(title);
                self.0.empty_text.set_text(text);
                self.0.empty_action.set_visible(action.is_some());
                if let Some((label, f)) = action {
                    self.0.empty_action.set_label(label);
                    *self.0.empty_handler.borrow_mut() = Some(f);
                }
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
        self.0.rows.borrow().iter().position(|r| r.cells().is_some_and(|(start, len)| idx >= start && idx < start + len))
    }

    fn scroll_to(&self, idx: usize) {
        let Some(row) = self.row_of(idx) else { return };
        // The first photo: show the heading above it too.
        let first = !self.0.rows.borrow()[..row].iter().any(|r| r.cells().is_some());
        self.0.list.scroll_to(if first { 0 } else { row as u32 }, gtk::ListScrollFlags::NONE, None);
    }

    /// The item above or below `idx`, `steps` rows away across group headings: the one
    /// whose middle is nearest horizontally. Stops at the first or last row.
    pub fn vertical_target(&self, idx: usize, down: bool, steps: usize) -> Option<usize> {
        let rows = self.0.rows.borrow();
        let mut at = self.row_of(idx)?;
        let centre = |row: &Row, n: usize| match row {
            Row::Cells { widths, .. } => widths[..n].iter().map(|w| w + GAP).sum::<i32>() + widths[n] / 2,
            Row::Header { .. } => 0,
        };
        let x = centre(&rows[at], idx - rows[at].cells()?.0);
        let mut moved = 0;
        let mut r = at as isize;
        while moved < steps {
            r += if down { 1 } else { -1 };
            if r < 0 || r as usize >= rows.len() {
                break;
            }
            if rows[r as usize].cells().is_some() {
                at = r as usize;
                moved += 1;
            }
        }
        if moved == 0 {
            return None;
        }
        let (start, len) = rows[at].cells()?;
        (0..len).min_by_key(|&n| (centre(&rows[at], n) - x).abs()).map(|n| start + n)
    }

    /// About how many rows of thumbnails fit on screen.
    pub fn rows_per_page(&self) -> usize {
        let page = self.0.scroller.vadjustment().page_size() as i32;
        let row = self.0.rows.borrow().iter().find_map(|r| matches!(r, Row::Cells { .. }).then(|| r.height())).unwrap_or(200);
        (page / row.max(1)).max(1) as usize
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

        let items = self.0.items.borrow();
        let mut rows = Vec::new();
        let mut labels = Vec::new();
        for span in self.0.spans.borrow().iter() {
            if !span.title.is_empty() {
                rows.push(Row::Header { title: span.title.clone(), sub: span.sub.clone() });
                labels.push(span.short.clone());
            }
            let aspects: Vec<f64> = items[span.start..span.end.min(items.len())].iter().map(|d| d.aspect).collect();
            let laid =
                if self.0.justified.get() { justify(&aspects, avail, target) } else { squares(aspects.len(), avail, target) };
            for (offset, height, widths) in laid {
                let start = span.start + offset;
                let label = span.shorts.get(offset).cloned().unwrap_or_else(|| span.short.clone());
                rows.push(Row::Cells { start, height, widths });
                labels.push(label);
            }
        }
        drop(items);
        let mut tops = Vec::with_capacity(rows.len() + 1);
        let mut y = 0;
        for r in &rows {
            tops.push(y);
            y += r.height();
        }
        tops.push(y);
        let objs: Vec<glib::BoxedAnyObject> = rows.iter().map(row_object).collect();
        // The rows are described twice: for the list, and for navigation.
        *self.0.rows.borrow_mut() = rows;
        *self.0.row_labels.borrow_mut() = labels;
        *self.0.row_tops.borrow_mut() = tops;
        let n = self.0.store.n_items();
        self.0.store.splice(0, n, &objs);
        if scroll_top {
            self.0.scroller.vadjustment().set_value(0.0);
        } else if let Some(i) = self.0.cursor.get() {
            self.scroll_to(i);
        }
        self.0.scrubber.queue_draw();
    }

    // ----- The date scrubber -----

    fn wire_scrubber(&self) {
        let s = &self.0.scrubber;
        let weak = Rc::downgrade(&self.0);
        s.set_draw_func(move |area, cr, w, h| {
            if let Some(inner) = weak.upgrade() {
                Gallery(inner).draw_scrubber(area, cr, w, h);
            }
        });
        let jump = {
            let weak = Rc::downgrade(&self.0);
            move |y: f64| {
                if let Some(inner) = weak.upgrade() {
                    Gallery(inner).scrub_to(y);
                }
            }
        };
        let drag = gtk::GestureDrag::new();
        let start = Rc::new(Cell::new(0.0));
        let (j, st, weak) = (jump.clone(), start.clone(), Rc::downgrade(&self.0));
        drag.connect_drag_begin(move |_, _, y| {
            st.set(y);
            if let Some(inner) = weak.upgrade() {
                inner.scrubbing.set(true);
            }
            j(y);
        });
        let j = jump.clone();
        drag.connect_drag_update(move |_, _, dy| j(start.get() + dy));
        let weak = Rc::downgrade(&self.0);
        drag.connect_drag_end(move |_, _, _| {
            if let Some(inner) = weak.upgrade() {
                inner.scrubbing.set(false);
                if !inner.scrubber.has_css_class("hover") {
                    inner.scrubber.set_content_width(SCRUB_NARROW);
                }
                Gallery(inner).hide_bubble_soon();
            }
        });
        s.add_controller(drag);

        let motion = gtk::EventControllerMotion::new();
        let weak = Rc::downgrade(&self.0);
        motion.connect_enter(move |_, _, _| {
            if let Some(inner) = weak.upgrade() {
                inner.scrubber.add_css_class("hover");
                inner.scrubber.set_content_width(SCRUB_WIDE);
            }
        });
        let weak = Rc::downgrade(&self.0);
        motion.connect_motion(move |_, _, y| {
            if let Some(inner) = weak.upgrade() {
                let g = Gallery(inner);
                let frac = y / g.0.scrubber.height().max(1) as f64;
                g.show_bubble(frac, None);
            }
        });
        let weak = Rc::downgrade(&self.0);
        motion.connect_leave(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.scrubber.remove_css_class("hover");
                if !inner.scrubbing.get() {
                    inner.scrubber.set_content_width(SCRUB_NARROW);
                    Gallery(inner).hide_bubble_soon();
                }
            }
        });
        s.add_controller(motion);

        // While scrolling, say where you are for a moment.
        let weak = Rc::downgrade(&self.0);
        self.0.scroller.vadjustment().connect_value_changed(move |adj| {
            let Some(inner) = weak.upgrade() else { return };
            let g = Gallery(inner);
            g.0.scrubber.queue_draw();
            if g.0.scrubbing.get() || g.0.scrubber.has_css_class("hover") {
                return;
            }
            let upper = adj.upper().max(1.0);
            let label_frac = adj.value() / upper;
            let marker = (adj.value() / (upper - adj.page_size()).max(1.0)).clamp(0.0, 1.0);
            g.show_bubble(label_frac, Some(marker));
            g.hide_bubble_soon();
        });
    }

    /// The row whose estimated top is at `frac` of the whole list.
    fn row_at(&self, frac: f64) -> Option<usize> {
        let tops = self.0.row_tops.borrow();
        let total = *tops.last()? as f64;
        let y = (frac.clamp(0.0, 1.0) * total) as i32;
        let n = tops.len().saturating_sub(1);
        (n > 0).then(|| tops[..n].partition_point(|&t| t <= y).saturating_sub(1))
    }

    fn scrub_to(&self, y: f64) {
        let h = self.0.scrubber.height().max(1) as f64;
        let frac = (y / h).clamp(0.0, 1.0);
        let adj = self.0.scroller.vadjustment();
        adj.set_value((frac * adj.upper()).min(adj.upper() - adj.page_size()).max(0.0));
        self.show_bubble(frac, None);
    }

    /// Show the label for the row at `frac`, beside the pointer or at `marker` (0 to 1,
    /// the position of the visible page).
    fn show_bubble(&self, frac: f64, marker: Option<f64>) {
        let Some(row) = self.row_at(frac) else { return };
        let label = self.0.row_labels.borrow().get(row).cloned().unwrap_or_default();
        if label.is_empty() {
            self.0.bubble.set_visible(false);
            return;
        }
        if let Some(t) = self.0.bubble_timer.borrow_mut().take() {
            t.remove();
        }
        let h = self.0.scrubber.height() as f64;
        let y = marker.unwrap_or(frac) * h;
        self.0.bubble.set_text(&label);
        self.0.bubble.set_margin_top((y - 14.0).clamp(4.0, (h - 36.0).max(4.0)) as i32);
        self.0.bubble.set_visible(true);
    }

    fn hide_bubble_soon(&self) {
        let weak = Rc::downgrade(&self.0);
        let id = glib::timeout_add_local_once(Duration::from_millis(900), move || {
            if let Some(inner) = weak.upgrade() {
                inner.bubble_timer.borrow_mut().take();
                if !inner.scrubbing.get() && !inner.scrubber.has_css_class("hover") {
                    inner.bubble.set_visible(false);
                }
            }
        });
        if let Some(old) = self.0.bubble_timer.borrow_mut().replace(id) {
            old.remove();
        }
    }

    /// Year marks down the right edge, and a bar for the part on screen.
    fn draw_scrubber(&self, area: &gtk::DrawingArea, cr: &gtk::cairo::Context, w: i32, h: i32) {
        let tops = self.0.row_tops.borrow();
        let labels = self.0.row_labels.borrow();
        let Some(&total) = tops.last() else { return };
        let adj = self.0.scroller.vadjustment();
        if total <= 0 || adj.upper() <= adj.page_size() + 1.0 {
            return;
        }
        let fg = area.color();
        let (w, h) = (w as f64, h as f64);
        let x = w - 9.0;
        let wide = w > SCRUB_NARROW as f64 + 1.0;
        // The visible page.
        let top = adj.value() / adj.upper() * h;
        let len = (adj.page_size() / adj.upper() * h).max(16.0);
        cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, 0.35);
        rounded(cr, x - 1.5, top, 3.0, len);
        let _ = cr.fill();

        // A mark where each year starts; a label where there is room for one.
        let layout = pangocairo::functions::create_layout(cr);
        let mut font = area.pango_context().font_description().unwrap_or_default();
        font.set_size((font.size() as f64 * 0.78) as i32);
        layout.set_font_description(Some(&font));
        let mut last_year = String::new();
        let mut last_label_y = f64::NEG_INFINITY;
        for (r, label) in labels.iter().enumerate() {
            let year = label.rsplit(' ').next().unwrap_or("").to_string();
            if year == last_year || year.is_empty() {
                continue;
            }
            last_year = year.clone();
            let y = tops[r] as f64 / total as f64 * h;
            cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, 0.45);
            cr.rectangle(x - 4.0, y, 8.0, 1.0);
            let _ = cr.fill();
            if wide && y - last_label_y >= 18.0 && y < h - 8.0 {
                layout.set_text(&year);
                let (tw, th) = layout.pixel_size();
                cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, 0.62);
                cr.move_to(x - 8.0 - tw as f64, (y - th as f64 / 2.0).max(0.0));
                pangocairo::functions::show_layout(cr, &layout);
                last_label_y = y;
            }
        }
    }
}

fn rounded(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64) {
    let r = w / 2.0;
    cr.new_sub_path();
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 0.0);
    cr.arc(x + r, y + h - r, r, 0.0, std::f64::consts::PI);
    cr.close_path();
}

/// Rows of equal squares: (offset into the group, height, widths).
fn squares(n: usize, avail: i32, target: f64) -> Vec<(usize, i32, Vec<i32>)> {
    let cols = (((avail + GAP) as f64 / (target + GAP as f64)).floor() as usize).max(1);
    let cell = ((avail - GAP * (cols as i32 - 1)) / cols as i32).max(40);
    (0..n).step_by(cols).map(|at| (at, cell, vec![cell; cols.min(n - at)])).collect()
}

/// Rows that keep each photo's shape: fill a row at about `target` high, then scale it
/// so it spans the width exactly. The last row of a group stays at the target height.
fn justify(aspects: &[f64], avail: i32, target: f64) -> Vec<(usize, i32, Vec<i32>)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < aspects.len() {
        let mut sum = 0.0;
        let mut end = at;
        while end < aspects.len() {
            sum += aspects[end].clamp(0.4, 3.5);
            end += 1;
            let gaps = (GAP * (end - at - 1) as i32) as f64;
            if sum * target + gaps >= avail as f64 {
                break;
            }
        }
        let n = end - at;
        let gaps = GAP * (n as i32 - 1);
        let full = sum * target + gaps as f64 >= avail as f64;
        let height = if full { ((avail - gaps) as f64 / sum).round().max(40.0) } else { target.round() };
        let mut widths: Vec<i32> =
            aspects[at..end].iter().map(|a| (a.clamp(0.4, 3.5) * height).round().max(24.0) as i32).collect();
        if full {
            // Rounding leaves the row a pixel or two off: give it to the last one.
            let used: i32 = widths.iter().sum::<i32>() + gaps;
            if let Some(last) = widths.last_mut() {
                *last = (*last + avail - used).max(24);
            }
        }
        out.push((at, height as i32, widths));
        at = end;
    }
    out
}

fn row_object(row: &Row) -> glib::BoxedAnyObject {
    glib::BoxedAnyObject::new(match row {
        Row::Header { title, sub } => Row::Header { title: title.clone(), sub: sub.clone() },
        Row::Cells { start, height, widths } => Row::Cells { start: *start, height: *height, widths: widths.clone() },
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
        title.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
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

fn stop_preview(c: &CellWidget) {
    if let Some(t) = c.preview_timer.borrow_mut().take() {
        t.remove();
    }
    if let Some(media) = c.preview.borrow_mut().take() {
        media.pause();
    }
}

/// Put a photo's thumbnail into `pic`, making and decoding it off the UI thread. A change
/// of `token` while that runs means the cell was recycled for another photo: drop it.
/// A thumbnail that was already decoded shows at once; a new one fades in.
fn load_texture(pic: &gtk::Picture, data: &CellData, token: Rc<Cell<u64>>) {
    if let Some(tex) = TEXTURES.with(|t| t.borrow_mut().get(&data.thumb)) {
        pic.add_css_class("instant");
        pic.add_css_class("loaded");
        pic.set_paintable(Some(&tex));
        return;
    }
    pic.remove_css_class("instant");
    pic.remove_css_class("loaded");
    pic.set_paintable(None::<&gdk::Paintable>);
    let pic = pic.clone();
    let mine = token.get();
    thumbnail_texture(data, move |tex| {
        if token.get() == mine {
            pic.set_paintable(Some(&tex));
            pic.add_css_class("loaded");
        }
    });
}

/// Get a photo's thumbnail as a texture (from memory, or made and decoded off the UI
/// thread) and hand it to `done`. For the viewer's filmstrip and the photo info too.
pub fn thumbnail_texture(data: &CellData, done: impl FnOnce(gdk::Texture) + 'static) {
    if let Some(tex) = TEXTURES.with(|t| t.borrow_mut().get(&data.thumb)) {
        done(tex);
        return;
    }
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
        TEXTURES.with(|t| t.borrow_mut().insert(key, tex.clone()));
        done(tex);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn justified_rows_fill_the_width() {
        let rows = justify(&[1.5, 0.75, 1.0, 1.33, 1.5, 1.0, 2.0], 800, 200.0);
        for (_, _, widths) in &rows[..rows.len() - 1] {
            assert_eq!(widths.iter().sum::<i32>() + GAP * (widths.len() as i32 - 1), 800);
        }
        assert_eq!(rows.iter().map(|r| r.2.len()).sum::<usize>(), 7);
    }

    #[test]
    fn squares_wrap() {
        let rows = squares(5, 400, 120.0);
        assert_eq!(rows.iter().map(|r| r.2.len()).collect::<Vec<_>>(), vec![3, 2]);
    }

    #[test]
    fn the_cache_drops_the_least_recently_used() {
        let mut c = TextureCache::default();
        let tex = || gdk::MemoryTexture::new(1, 1, gdk::MemoryFormat::R8g8b8a8, &glib::Bytes::from(&[0u8; 4]), 4).upcast();
        for n in 0..CACHE_LIMIT {
            c.insert(n.to_string(), tex());
        }
        c.get("0");
        c.insert("new".into(), tex());
        assert!(c.map.contains_key("0"));
        assert!(!c.map.contains_key("1"));
    }
}
