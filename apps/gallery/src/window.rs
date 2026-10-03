//! The main window: the sidebar with search, the gallery page and the settings page
//! side by side in a stack, the photo info drawer, the viewer and toasts.

use crate::editor::{Editor, Outcome};
use crate::grid::{CellData, Gallery};
use crate::inspector::{self, Inspector};
use crate::lightbox::{self, Lightbox};
use crate::state::{LIBRARY_SECTIONS, MEDIA_SECTIONS, State};
use crate::widgets::{self, hbox, label, vbox};
use crate::{cmd, dialog, library, paths, prefs, settings_view, theme};
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

const APP_NAME: &str = "Nexus Photos";
const VIEWER: &str = "nexus-photos-viewer";

/// What a context-menu item does when clicked.
type MenuAction = Box<dyn Fn(&Rc<App>)>;
const SETTINGS_PAGE: &str = "settings";
/// Below this the sidebar always shows only icons.
const NARROW_BELOW: i32 = 980;

pub struct App {
    window: gtk::ApplicationWindow,
    overlay: gtk::Overlay,
    state: RefCell<State>,
    gallery: Gallery,
    inspector: Inspector,
    lightbox: Lightbox,
    editor: Editor,
    pages: gtk::Stack,
    gallery_page: gtk::Box,
    settings_page: RefCell<Option<gtk::ScrolledWindow>>,
    page: RefCell<String>,
    // Header
    header_actions: gtk::Box,
    info_button: gtk::Button,
    /// The thumbnail-size slider: inline when there is room, in a popover when narrow.
    zoom_inline: gtk::Box,
    zoom_menu: gtk::MenuButton,
    deselect: gtk::Button,
    // Status bar
    status_items: gtk::Label,
    status_selection: gtk::Label,
    status_hints: gtk::Box,
    // Sidebar
    nav: gtk::Box,
    nav_items: RefCell<HashMap<String, gtk::Button>>,
    albums_box: gtk::Box,
    search: gtk::SearchEntry,
    search_icon: gtk::Button,
    collapse_button: gtk::Button,
    compact_fixed: RefCell<Vec<gtk::Widget>>,
    compact_items: RefCell<Vec<gtk::Widget>>,
    counts: RefCell<HashMap<String, gtk::Label>>,
    compact: Cell<bool>,
    narrow: Cell<bool>,
    temp_expanded: Cell<bool>,
    // Misc
    toast: RefCell<Option<gtk::Revealer>>,
    scan_depth: Cell<usize>,
    scanning: Cell<bool>,
    /// Open the editor on the file named on the command line, once it has been found.
    edit_when_opened: Cell<bool>,
    /// A file to open in the viewer once the next scan has found it.
    open_after_scan: RefCell<Option<PathBuf>>,
}

thread_local! {
    static APP: RefCell<Option<Rc<App>>> = const { RefCell::new(None) };
}

fn app() -> Option<Rc<App>> {
    APP.with(|a| a.borrow().clone())
}

pub fn window() -> Option<gtk::ApplicationWindow> {
    app().map(|a| a.window.clone())
}

/// Called when Reduce motion changes: the toolkit follows the setting by itself, but
/// the page crossfade is ours.
pub fn apply_motion() {
    if let Some(a) = app() {
        a.pages.set_transition_duration(if prefs::get().reduce_motion { 0 } else { 160 });
    }
}

pub fn present(application: &gtk::Application, section: Option<&str>, open: Option<PathBuf>, edit: bool) {
    if let Some(a) = app() {
        if let Some(s) = section {
            a.navigate(s);
        }
        if let Some(path) = open {
            a.edit_when_opened.set(edit);
            a.open_path(path);
        }
        a.window.present();
        return;
    }
    theme::install();
    let root = open.as_ref().and_then(|f| f.parent().map(PathBuf::from)).unwrap_or_else(library::default_root);
    let a = App::build(application, root);
    if open.is_some() {
        a.scan_depth.set(1);
    }
    APP.with(|slot| *slot.borrow_mut() = Some(a.clone()));
    let p = prefs::get();
    let start = section.map(String::from).unwrap_or(p.last_section);
    a.navigate(&start);
    a.window.present();
    a.edit_when_opened.set(edit && open.is_some());
    *a.open_after_scan.borrow_mut() = open;
    a.rescan();
}

impl App {
    fn build(application: &gtk::Application, root: PathBuf) -> Rc<App> {
        let p = prefs::get();
        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .title(APP_NAME)
            .default_width(1120)
            .default_height(800)
            .build();
        window.add_css_class("photos-window");
        // No client-side titlebar: Hyprland manages the window.
        window.set_titlebar(Some(&gtk::Box::new(gtk::Orientation::Horizontal, 0)));
        window.set_icon_name(Some("io.github.nexus.photos"));

        let mut state = State::new(library::Library::load(root), p.view_mode.clone());
        state.recompute();

        let gallery = Gallery::new();
        gallery.set_zoom(p.zoom);
        let inspector = Inspector::new();
        let lightbox = Lightbox::new();
        let editor = Editor::new(
            |message| {
                if let Some(a) = app() {
                    a.show_toast(message);
                }
            },
            |outcome| {
                if let Some(a) = app() {
                    a.on_editor_outcome(outcome);
                }
            },
        );

        // ----- Sidebar -----
        let nav = vbox(0);
        nav.add_css_class("settings-navigation");
        nav.set_hexpand(false);
        let heading = label(APP_NAME, "menu-heading");
        heading.set_hexpand(true);
        let head = hbox(0);
        head.add_css_class("nav-head");
        head.append(&heading);
        let collapse_button = gtk::Button::from_icon_name("sidebar-show-symbolic");
        collapse_button.add_css_class("nav-collapse");
        collapse_button.set_tooltip_text(Some("Collapse the sidebar (Ctrl+B)"));
        collapse_button.set_valign(gtk::Align::Center);
        head.append(&collapse_button);
        nav.append(&head);

        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search photos"));
        search.add_css_class("settings-search");
        nav.append(&search);
        let search_icon = gtk::Button::from_icon_name("system-search-symbolic");
        search_icon.add_css_class("nav-search");
        search_icon.set_tooltip_text(Some("Search photos (Ctrl+F)"));
        search_icon.set_halign(gtk::Align::Center);
        search_icon.set_visible(false);
        nav.append(&search_icon);

        let list = vbox(0);
        let albums_box = vbox(0);
        let nav_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            // Scrolls with wheel, trackpad and keyboard; no visible scrollbar.
            .vscrollbar_policy(gtk::PolicyType::External)
            .vexpand(true)
            .child(&list)
            .build();
        nav.append(&nav_scroll);

        let settings_holder = vbox(0);
        nav.append(&settings_holder);
        let footer = hbox(6);
        footer.add_css_class("nav-footer");
        let version = label(concat!("Nexus Photos ", env!("CARGO_PKG_VERSION")), "dim");
        version.set_hexpand(true);
        footer.append(&version);
        nav.append(&footer);

        // ----- Gallery page -----
        let gallery_page = vbox(0);
        gallery_page.set_hexpand(true);
        // One slim bar on top: how to group, how big, and a few actions. No title: the
        // sidebar already says where you are.
        let bar = hbox(8);
        bar.add_css_class("gallery-bar");
        let header_actions = hbox(8);
        let deselect = gtk::Button::with_label("Deselect");
        gallery_page.append(&bar);
        // The info drawer sits below the bar, beside the grid, so the bar stays usable.
        let body_overlay = gtk::Overlay::new();
        body_overlay.set_child(Some(gallery.widget()));
        body_overlay.set_vexpand(true);
        body_overlay.add_overlay(&inspector.root);
        gallery_page.append(&body_overlay);

        let status = hbox(16);
        status.add_css_class("status-bar");
        let status_items = label("", "dim");
        let status_selection = label("", "accent-text");
        status_selection.set_margin_start(4);
        let status_hints = hbox(14);
        status_hints.set_hexpand(true);
        status_hints.set_halign(gtk::Align::End);
        for (keys, text) in [("Space", "View"), ("E", "Edit"), ("F", "Favorite"), ("I", "Info")] {
            let hint = hbox(6);
            hint.append(&widgets::key_cap(keys));
            hint.append(&label(text, "dim"));
            status_hints.append(&hint);
        }
        status.append(&status_items);
        status.append(&status_selection);
        status.append(&status_hints);
        gallery_page.append(&status);

        // ----- Pages and the info drawer -----
        let pages = gtk::Stack::new();
        pages.add_css_class("settings-content");
        pages.set_hexpand(true);
        pages.set_transition_type(gtk::StackTransitionType::Crossfade);
        pages.set_transition_duration(if p.reduce_motion { 0 } else { 160 });
        // The bar and status bar must not set the window's minimum width, or a half-screen
        // window could never get narrow enough to switch to the compact layout.
        let gallery_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::External)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .child(&gallery_page)
            .build();
        pages.add_named(&gallery_scroll, Some("gallery"));

        let content = gtk::Overlay::new();
        content.set_child(Some(&pages));
        content.set_hexpand(true);
        inspector.root.set_halign(gtk::Align::End);
        inspector.root.set_valign(gtk::Align::Fill);

        let body = hbox(0);
        body.append(&nav);
        body.append(&content);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&body));
        overlay.add_overlay(lightbox.widget());
        overlay.add_overlay(editor.widget());
        window.set_child(Some(&overlay));

        let app = Rc::new(App {
            window,
            overlay,
            state: RefCell::new(state),
            gallery,
            inspector,
            lightbox,
            editor,
            pages,
            gallery_page,
            settings_page: RefCell::new(None),
            page: RefCell::new(String::new()),
            header_actions,
            info_button: widgets::icon_button("dialog-information-symbolic", "Photo info (I)"),
            zoom_inline: hbox(6),
            zoom_menu: gtk::MenuButton::new(),
            deselect,
            status_items,
            status_selection,
            status_hints,
            nav,
            nav_items: RefCell::default(),
            albums_box,
            search,
            search_icon,
            collapse_button,
            compact_fixed: RefCell::new(vec![heading.upcast(), footer.upcast()]),
            compact_items: RefCell::default(),
            counts: RefCell::default(),
            compact: Cell::new(false),
            narrow: Cell::new(false),
            temp_expanded: Cell::new(false),
            toast: RefCell::new(None),
            scan_depth: Cell::new(8),
            scanning: Cell::new(false),
            edit_when_opened: Cell::new(false),
            open_after_scan: RefCell::new(None),
        });
        app.compact_fixed.borrow_mut().push(app.search.clone().upcast());
        app.build_toolbar(&bar);
        app.build_nav(&list, &settings_holder);
        app.wire(&nav_scroll);
        app
    }

    // ----- Building -----

    fn build_toolbar(self: &Rc<Self>, bar: &gtk::Box) {
        let p = prefs::get();
        let a = self.clone();
        let mode = widgets::segmented(
            &[("years", "Years"), ("months", "Months"), ("days", "Days"), ("all", "All")],
            &p.view_mode,
            move |m| {
                prefs::update(|p| p.view_mode = m.clone());
                a.state.borrow_mut().view_mode = m;
                a.refresh(true);
            },
        );
        bar.append(&mode);
        // Rename and delete, only while an album is open.
        bar.append(&self.header_actions);

        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        bar.append(&spacer);

        self.deselect.set_visible(false);
        self.deselect.add_css_class("flat");
        let a = self.clone();
        self.deselect.connect_clicked(move |_| {
            a.state.borrow_mut().clear_selection();
            a.refresh_selection(false);
        });
        bar.append(&self.deselect);

        let apply_zoom = |v: f64| {
            prefs::update(|p| p.zoom = v);
            if let Some(a) = app() {
                a.gallery.set_zoom(v);
            }
        };
        let make_scale = |width: i32| {
            let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.02);
            scale.set_value(p.zoom);
            scale.set_draw_value(false);
            scale.set_width_request(width);
            scale.set_tooltip_text(Some("Thumbnail size"));
            scale
        };
        let inline = make_scale(120);
        let popover_scale = make_scale(180);
        // Moving either slider moves the other; setting an equal value does nothing, so it settles.
        for (this, other) in [(&inline, &popover_scale), (&popover_scale, &inline)] {
            let other = other.clone();
            this.connect_value_changed(move |s| {
                if (other.value() - s.value()).abs() > 1e-6 {
                    other.set_value(s.value());
                }
                apply_zoom(s.value());
            });
        }
        let zoom = &self.zoom_inline;
        zoom.add_css_class("zoom-control");
        let small = gtk::Image::from_icon_name("zoom-out-symbolic");
        small.add_css_class("dim");
        zoom.append(&small);
        zoom.append(&inline);
        let big = gtk::Image::from_icon_name("zoom-in-symbolic");
        big.add_css_class("dim");
        zoom.append(&big);
        bar.append(zoom);

        self.zoom_menu.set_icon_name("zoom-in-symbolic");
        self.zoom_menu.set_tooltip_text(Some("Thumbnail size"));
        self.zoom_menu.add_css_class("icon-button");
        self.zoom_menu.add_css_class("flat");
        self.zoom_menu.set_valign(gtk::Align::Center);
        self.zoom_menu.set_visible(false);
        let pop = gtk::Popover::new();
        let pop_box = hbox(8);
        pop_box.append(&gtk::Image::from_icon_name("zoom-out-symbolic"));
        pop_box.append(&popover_scale);
        pop_box.append(&gtk::Image::from_icon_name("zoom-in-symbolic"));
        pop.set_child(Some(&pop_box));
        self.zoom_menu.set_popover(Some(&pop));
        bar.append(&self.zoom_menu);

        let select_all = widgets::icon_button("edit-select-all-symbolic", "Select all (Ctrl+A)");
        let a = self.clone();
        select_all.connect_clicked(move |_| {
            a.state.borrow_mut().select_all();
            a.refresh_selection(false);
        });
        bar.append(&select_all);
        let rescan = widgets::icon_button("view-refresh-symbolic", "Scan the folder again");
        let a = self.clone();
        rescan.connect_clicked(move |_| a.rescan());
        bar.append(&rescan);
        let a = self.clone();
        self.info_button.connect_clicked(move |_| a.toggle_inspector());
        bar.append(&self.info_button);
    }

    fn nav_item(self: &Rc<Self>, list: &gtk::Box, id: &str, icon: &str, text: &str, count: Option<usize>) {
        let button = gtk::Button::new();
        button.add_css_class("nav-item");
        let content = hbox(12);
        content.append(&gtk::Image::from_icon_name(icon));
        let l = label(text, "nav-label");
        l.set_hexpand(true);
        l.set_ellipsize(gtk::pango::EllipsizeMode::End);
        self.compact_items.borrow_mut().push(l.clone().upcast());
        content.append(&l);
        if let Some(n) = count {
            let c = label(&n.to_string(), "nav-count mono");
            self.compact_items.borrow_mut().push(c.clone().upcast());
            self.counts.borrow_mut().insert(id.to_string(), c.clone());
            content.append(&c);
        }
        button.set_child(Some(&content));
        button.set_tooltip_text(Some(text));
        let (a, id2) = (self.clone(), id.to_string());
        button.connect_clicked(move |_| a.navigate(&id2));
        list.append(&button);
        self.nav_items.borrow_mut().insert(id.to_string(), button);
    }

    fn make_group_label(&self, text: &str) -> gtk::Label {
        let g = label(&text.to_uppercase(), "nav-group");
        self.compact_items.borrow_mut().push(g.clone().upcast());
        g
    }

    fn group_label(&self, list: &gtk::Box, text: &str) {
        list.append(&self.make_group_label(text));
    }

    fn build_nav(self: &Rc<Self>, list: &gtk::Box, settings_holder: &gtk::Box) {
        self.compact_items.borrow_mut().clear();
        self.counts.borrow_mut().clear();
        self.nav_items.borrow_mut().clear();
        self.group_label(list, "Library");
        for (id, title, icon) in LIBRARY_SECTIONS {
            let n = self.state.borrow().count(id);
            self.nav_item(list, id, icon, title, Some(n));
        }
        self.group_label(list, "Media");
        for (id, title, icon) in MEDIA_SECTIONS {
            let n = self.state.borrow().count(id);
            self.nav_item(list, id, icon, title, Some(n));
        }
        // Albums: a heading with an add button, then one item per album.
        let head = hbox(0);
        head.add_css_class("nav-group-row");
        let g = self.make_group_label("Albums");
        g.set_hexpand(true);
        head.append(&g);
        let add = gtk::Button::from_icon_name("list-add-symbolic");
        add.add_css_class("nav-collapse");
        add.add_css_class("nav-add");
        add.set_tooltip_text(Some("New album"));
        add.set_valign(gtk::Align::Center);
        let a = self.clone();
        add.connect_clicked(move |_| a.new_album());
        head.append(&add);
        // The heading's text hides when compact; the add button stays, centred.
        list.append(&head);
        list.append(&self.albums_box);
        self.build_albums();
        self.nav_item(settings_holder, SETTINGS_PAGE, "emblem-system-symbolic", "Settings", None);
        if let Some(b) = self.nav_items.borrow().get(SETTINGS_PAGE) {
            b.add_css_class("nav-settings");
        }
    }

    fn build_albums(self: &Rc<Self>) {
        // Forget the old album items before they are removed.
        self.compact_items.borrow_mut().retain(|w| !w.is_ancestor(&self.albums_box));
        self.counts.borrow_mut().retain(|_, l| !l.is_ancestor(&self.albums_box));
        while let Some(c) = self.albums_box.first_child() {
            self.albums_box.remove(&c);
        }
        let albums: Vec<(String, String)> =
            self.state.borrow().lib.albums.iter().map(|a| (a.id.clone(), a.name.clone())).collect();
        {
            let mut items = self.nav_items.borrow_mut();
            items.retain(|k, _| self.state.borrow().lib.album(k).is_some() || !k.starts_with("album_") && k != "highlight_album");
        }
        for (id, name) in albums {
            let n = self.state.borrow().count(&id);
            self.nav_item(&self.albums_box, &id, "folder-symbolic", &name, Some(n));
        }
        let compact = self.compact.get();
        for w in self.compact_items.borrow().iter() {
            w.set_visible(!compact);
        }
        centre_icons(self.nav.upcast_ref(), compact);
        self.mark_active_nav();
    }

    fn wire(self: &Rc<Self>, _nav_scroll: &gtk::ScrolledWindow) {
        let w = &self.window;

        let a = self.clone();
        self.collapse_button.connect_clicked(move |_| a.toggle_sidebar());
        let a = self.clone();
        self.search_icon.connect_clicked(move |_| a.focus_search());
        let a = self.clone();
        self.search.connect_search_changed(move |e| {
            a.state.borrow_mut().query = e.text().to_string();
            if *a.page.borrow() == SETTINGS_PAGE && !e.text().is_empty() {
                let section = a.state.borrow().section.clone();
                a.navigate(&section);
            }
            a.refresh(true);
        });
        let a = self.clone();
        self.search.connect_activate(move |_| a.leave_search());
        // Back to icons once the search is done with.
        let focus = gtk::EventControllerFocus::new();
        let a = self.clone();
        focus.connect_leave(move |_| {
            if a.search.text().is_empty() && a.temp_expanded.replace(false) {
                a.apply_compact(a.narrow.get() || prefs::get().sidebar_collapsed);
            }
        });
        self.search.add_controller(focus);

        // Keys. The capture phase sees them before the focused widget does.
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let a = self.clone();
        keys.connect_key_pressed(move |_, key, _, mods| a.on_key(key, mods));
        w.add_controller(keys);

        // Thumbnails and the viewer.
        let a = self.clone();
        self.gallery.on_click(move |idx, mods, n| a.on_cell_click(idx, mods, n));
        // The star on a thumbnail favourites that photo only, whatever is selected.
        let a = self.clone();
        self.gallery.on_favorite(move |idx| {
            let id = a.state.borrow().photo(idx).map(|p| p.id.clone());
            if let Some(id) = id {
                a.toggle_favorites(vec![id]);
            }
        });
        let a = self.clone();
        self.gallery.on_context(move |idx, anchor, x, y| a.show_context_menu(idx, anchor, x, y));
        let a = self.clone();
        self.inspector.on_action(move |action| a.on_inspector(action));
        let a = self.clone();
        self.lightbox.on_action(move |action| a.on_lightbox(action));

        // Narrow windows (a tiled half-screen) get an icon-only sidebar.
        let a = self.clone();
        w.connect_default_width_notify(move |_| a.apply_width());
        let a = self.clone();
        w.connect_realize(move |_| a.apply_width());
        // Tiled windows are resized by the compositor; watch the real size too.
        let a = self.clone();
        glib::timeout_add_local(Duration::from_millis(400), move || {
            a.apply_width();
            glib::ControlFlow::Continue
        });
        w.connect_close_request(|_| {
            prefs::flush();
            glib::Propagation::Proceed
        });
        self.apply_inspector_visibility();
    }

    // ----- Keys -----

    fn typing(&self) -> bool {
        GtkWindowExt::focus(&self.window).is_some_and(|f| f.dynamic_cast_ref::<gtk::Editable>().is_some())
    }

    fn on_key(self: &Rc<Self>, key: gdk::Key, mods: gdk::ModifierType) -> glib::Propagation {
        let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
        let stop = glib::Propagation::Stop;
        if self.editor.is_open() {
            if ctrl && matches!(key, gdk::Key::q | gdk::Key::w) {
                self.window.close();
                return stop;
            }
            return self.editor.handle_key(key, mods);
        }
        if ctrl {
            match key {
                gdk::Key::f | gdk::Key::k => self.focus_search(),
                gdk::Key::b => self.toggle_sidebar(),
                gdk::Key::q | gdk::Key::w => self.window.close(),
                gdk::Key::o => self.reveal(),
                gdk::Key::a if !self.typing() && self.on_gallery() => {
                    self.state.borrow_mut().select_all();
                    self.refresh_selection(false);
                }
                _ => return glib::Propagation::Proceed,
            }
            return stop;
        }
        if key == gdk::Key::Escape {
            self.escape();
            return stop;
        }
        if self.typing() {
            return glib::Propagation::Proceed;
        }
        if self.lightbox.is_open() {
            match key {
                gdk::Key::Left => self.step_viewer(-1),
                gdk::Key::Right => self.step_viewer(1),
                gdk::Key::space | gdk::Key::Return | gdk::Key::KP_Enter => self.close_viewer(),
                gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => self.lightbox.zoom_by(1.25),
                gdk::Key::minus | gdk::Key::KP_Subtract => self.lightbox.zoom_by(0.8),
                gdk::Key::_0 | gdk::Key::KP_0 => self.lightbox.zoom_fit(),
                gdk::Key::_1 | gdk::Key::KP_1 => self.lightbox.zoom_actual(),
                gdk::Key::r => self.rotate(),
                gdk::Key::e => self.open_editor(),
                gdk::Key::f | gdk::Key::period => self.toggle_favorite(),
                _ => return glib::Propagation::Proceed,
            }
            return stop;
        }
        if !self.on_gallery() {
            return glib::Propagation::Proceed;
        }
        match key {
            gdk::Key::Left => self.move_cursor(-1),
            gdk::Key::Right => self.move_cursor(1),
            gdk::Key::Up => self.move_vertical(false),
            gdk::Key::Down => self.move_vertical(true),
            gdk::Key::space | gdk::Key::Return | gdk::Key::KP_Enter => self.open_viewer(),
            gdk::Key::i => self.toggle_inspector(),
            gdk::Key::r => self.rotate(),
            gdk::Key::e => self.open_editor(),
            gdk::Key::f | gdk::Key::period => self.toggle_favorite(),
            _ => return glib::Propagation::Proceed,
        }
        stop
    }

    fn escape(self: &Rc<Self>) {
        if self.lightbox.is_open() {
            self.close_viewer();
        } else if !self.search.text().is_empty() {
            self.search.set_text("");
            self.leave_search();
        } else if self.typing() {
            self.leave_search();
        } else if !self.on_gallery() {
            let section = self.state.borrow().section.clone();
            self.navigate(&section);
        } else if self.temp_expanded.replace(false) {
            self.apply_compact(self.narrow.get() || prefs::get().sidebar_collapsed);
        } else {
            self.state.borrow_mut().clear_selection();
            self.refresh_selection(false);
        }
    }

    fn on_gallery(&self) -> bool {
        *self.page.borrow() != SETTINGS_PAGE
    }

    // ----- Navigation -----

    pub fn navigate(self: &Rc<Self>, id: &str) {
        let id = if id == "timeline" { "all_photos" } else { id };
        self.lightbox.hide();
        if id == SETTINGS_PAGE {
            self.show_settings();
        } else {
            let known = self.state.borrow().lib.album(id).is_some()
                || LIBRARY_SECTIONS.iter().chain(MEDIA_SECTIONS).any(|(s, _, _)| *s == id);
            let id = if known { id } else { "all_photos" };
            self.state.borrow_mut().section = id.to_string();
            *self.page.borrow_mut() = id.to_string();
            self.pages.set_visible_child_name("gallery");
            self.refresh(true);
            prefs::update(|p| p.last_section = id.to_string());
        }
        if id == SETTINGS_PAGE {
            *self.page.borrow_mut() = SETTINGS_PAGE.into();
        }
        self.mark_active_nav();
        self.apply_inspector_visibility();
    }

    fn show_settings(self: &Rc<Self>) {
        if let Some(old) = self.settings_page.borrow_mut().take() {
            self.pages.remove(&old);
        }
        let (items, size, library) = {
            let s = self.state.borrow();
            (s.lib.photos.len(), s.lib.total_bytes_formatted(), paths::pretty(&s.lib.root))
        };
        let a = self.clone();
        let (page, body) =
            settings_view::build(settings_view::Info { library, items, size, on_rescan: Rc::new(move || a.rescan()) });
        if self.narrow.get() {
            body.add_css_class("narrow");
        }
        self.pages.add_named(&page, Some("settings"));
        *self.settings_page.borrow_mut() = Some(page);
        self.pages.set_visible_child_name("settings");
        *self.page.borrow_mut() = SETTINGS_PAGE.into();
    }

    fn mark_active_nav(&self) {
        let page = self.page.borrow().clone();
        for (id, b) in self.nav_items.borrow().iter() {
            if *id == page {
                b.add_css_class("active");
            } else {
                b.remove_css_class("active");
            }
        }
    }

    // ----- Refreshing what is on screen -----

    fn refresh(self: &Rc<Self>, scroll_top: bool) {
        self.state.borrow_mut().recompute();
        self.update_counts();
        let s = self.state.borrow();
        let items: Vec<CellData> = s
            .shown
            .iter()
            .map(|&i| {
                let p = &s.lib.photos[i];
                CellData {
                    thumb: p.thumbnail_path.clone(),
                    source: p.path.clone(),
                    mtime: p.mtime,
                    favorite: p.is_favorite,
                    badge: (p.media_type == "videos").then(|| p.format.clone()),
                }
            })
            .collect();
        let empty = items.is_empty();
        let message = self.empty_message(&s);
        let n = s.shown.len();
        self.status_items.set_text(&format!("{} · {n} {}", s.section_title(), if n == 1 { "item" } else { "items" }));
        let spans = s.spans.clone();
        let album = s.album_id().map(String::from);
        drop(s);
        self.gallery.set_items(items, spans, scroll_top);
        self.gallery.set_empty(if empty { Some((message.0, message.1.as_str(), message.2.as_str())) } else { None });
        self.update_album_actions(album);
        self.refresh_selection(true);
    }

    fn empty_message(&self, s: &State) -> (&'static str, String, String) {
        let root = paths::pretty(&s.lib.root);
        if self.scanning.get() {
            return ("view-refresh-symbolic", "Scanning your photos".into(), format!("Looking in {root}."));
        }
        if !s.query.trim().is_empty() {
            return (
                "system-search-symbolic",
                format!("No photos match “{}”", s.query.trim()),
                "Try a file name, a camera, a lens or a date.".into(),
            );
        }
        match s.section.as_str() {
            "favorites" => ("starred-symbolic", "No favorites yet".into(), "Press F on a photo to add it here.".into()),
            "videos" => {
                ("video-x-generic-symbolic", "No videos found".into(), format!("Add video files (.mp4, .mov, .mkv) to {root}."))
            }
            "screenshots" => {
                ("video-display-symbolic", "No screenshots found".into(), format!("Screenshots in {root} appear here."))
            }
            id if s.lib.album(id).is_some() => (
                "folder-symbolic",
                "This album is empty".into(),
                "Select photos and use “Add to album” in the photo info.".into(),
            ),
            _ => ("image-x-generic-symbolic", "No photos found".into(), format!("Add photos to {root}.")),
        }
    }

    /// Rename and delete, shown in the bar while an album is open.
    fn update_album_actions(self: &Rc<Self>, album: Option<String>) {
        while let Some(c) = self.header_actions.first_child() {
            self.header_actions.remove(&c);
        }
        let Some(id) = album else { return };
        let rename = widgets::icon_button("document-edit-symbolic", "Rename album");
        let (a, rid) = (self.clone(), id.clone());
        rename.connect_clicked(move |_| a.rename_album(&rid));
        self.header_actions.append(&rename);
        let a = self.clone();
        let delete = widgets::confirm_button("Delete album", "Click again to delete", move |_| a.delete_album(&id));
        delete.add_css_class("destructive-action");
        self.header_actions.append(&delete);
    }

    fn update_counts(&self) {
        let s = self.state.borrow();
        for (id, l) in self.counts.borrow().iter() {
            l.set_text(&s.count(id).to_string());
        }
    }

    fn refresh_selection(self: &Rc<Self>, reveal: bool) {
        let s = self.state.borrow();
        self.gallery.set_selection(s.selected_indices(), s.cursor_index(), reveal);
        let n = s.selected.len();
        self.status_selection.set_text(&if n > 1 { format!("· {n} selected") } else { String::new() });
        self.deselect.set_visible(n > 1);
        let view = inspector::View {
            photo: s.cursor_photo().cloned(),
            selected: n,
            albums: s.lib.albums.iter().map(|a| (a.id.clone(), a.name.clone())).collect(),
            in_album: s.album_id().is_some(),
            rotation: s.cursor.as_ref().and_then(|id| s.rotation.get(id)).copied().unwrap_or(0),
        };
        drop(s);
        self.inspector.update(&view);
    }

    // ----- Library -----

    fn rescan(self: &Rc<Self>) {
        if self.scanning.replace(true) {
            return;
        }
        self.status_items.set_text("Scanning…");
        let (root, depth) = (self.state.borrow().lib.root.clone(), self.scan_depth.get());
        let a = self.clone();
        cmd::background(
            move || library::scan(&root, depth),
            move |photos| {
                a.scanning.set(false);
                let n = photos.len();
                a.state.borrow_mut().lib.set_photos(photos);
                a.refresh(true);
                if !a.on_gallery() {
                    a.show_settings();
                }
                if let Some(path) = a.open_after_scan.borrow_mut().take() {
                    a.open_path(path);
                }
                a.show_toast(&format!("Found {n} {}", if n == 1 { "item" } else { "items" }));
            },
        );
        // Show the scanning message right away.
        if self.state.borrow().shown.is_empty() {
            self.refresh(true);
        }
    }

    /// Show one file in the viewer, looking in its folder if it isn't in the library yet.
    pub fn open_path(self: &Rc<Self>, path: PathBuf) {
        let id = self.state.borrow().lib.photos.iter().find(|p| std::path::Path::new(&p.path) == path).map(|p| p.id.clone());
        match id {
            Some(id) => {
                self.search.set_text("");
                let idx = {
                    let mut s = self.state.borrow_mut();
                    s.query.clear();
                    s.recompute();
                    s.index_of(&id)
                };
                if let Some(i) = idx {
                    self.state.borrow_mut().select_single(i);
                    self.refresh_selection(true);
                    if self.edit_when_opened.take() {
                        self.open_editor();
                    } else {
                        self.open_builtin_viewer();
                    }
                }
            }
            None => {
                if let Some(dir) = path.parent() {
                    self.state.borrow_mut().lib.root = dir.to_path_buf();
                }
                self.scan_depth.set(1);
                *self.open_after_scan.borrow_mut() = Some(path);
                self.scanning.set(false);
                self.rescan();
            }
        }
    }

    // ----- Selection and actions -----

    fn on_cell_click(self: &Rc<Self>, idx: usize, mods: gdk::ModifierType, presses: i32) {
        {
            let mut s = self.state.borrow_mut();
            if presses >= 2 {
                s.select_single(idx);
            } else if mods.contains(gdk::ModifierType::CONTROL_MASK) {
                s.toggle(idx);
            } else if mods.contains(gdk::ModifierType::SHIFT_MASK) {
                s.select_range(idx);
            } else {
                s.select_single(idx);
            }
        }
        self.refresh_selection(false);
        if presses >= 2 {
            self.open_viewer();
        }
    }

    fn move_cursor(self: &Rc<Self>, delta: isize) {
        let cur = self.state.borrow().cursor_index().map(|i| i as isize).unwrap_or(-1);
        self.state.borrow_mut().move_to(cur + delta);
        self.refresh_selection(true);
    }

    fn move_vertical(self: &Rc<Self>, down: bool) {
        let Some(cur) = self.state.borrow().cursor_index() else {
            self.move_cursor(1);
            return;
        };
        if let Some(target) = self.gallery.vertical_target(cur, down) {
            self.state.borrow_mut().select_single(target);
            self.refresh_selection(true);
        }
    }

    /// Open the photo under the cursor in the viewer chosen in Settings.
    fn open_viewer(self: &Rc<Self>) {
        let standalone = {
            let s = self.state.borrow();
            let Some(photo) = s.cursor_photo() else { return };
            (prefs::get().open_with == "viewer" && photo.media_type != "videos" && cmd::present(VIEWER))
                .then(|| photo.path.clone())
        };
        match standalone {
            Some(path) => cmd::spawn(&[VIEWER, &path]),
            None => self.open_builtin_viewer(),
        }
    }

    fn open_builtin_viewer(self: &Rc<Self>) {
        if self.state.borrow().cursor_photo().is_some() {
            self.show_in_viewer();
        }
    }

    fn show_in_viewer(&self) {
        let s = self.state.borrow();
        let Some(photo) = s.cursor_photo() else { return };
        let rotation = s.rotation.get(&photo.id).copied().unwrap_or(0);
        self.lightbox.show(photo, rotation, s.cursor_index().map(|i| i + 1).unwrap_or(1), s.shown.len());
    }

    fn close_viewer(self: &Rc<Self>) {
        self.lightbox.hide();
        self.refresh_selection(true);
    }

    fn step_viewer(self: &Rc<Self>, delta: isize) {
        self.state.borrow_mut().step_wrapping(delta);
        self.show_in_viewer();
        self.refresh_selection(false);
    }

    /// Open the editor on the selected photo, starting from how the viewer has it turned.
    fn open_editor(self: &Rc<Self>) {
        let (path, name, rotation) = {
            let s = self.state.borrow();
            let Some(p) = s.cursor_photo() else { return };
            (p.path.clone(), p.file_name.clone(), s.rotation.get(&p.id).copied().unwrap_or(0))
        };
        self.editor.open(std::path::Path::new(&path), &name, rotation);
    }

    fn on_editor_outcome(self: &Rc<Self>, outcome: Outcome) {
        let Outcome::Saved { path, original, backup } = outcome else {
            self.refresh_selection(false);
            return;
        };
        let replaced = path == original;
        let id = {
            let mut s = self.state.borrow_mut();
            // The editor started from the viewer's turn, so it is in the file now.
            let original_id = s.lib.photos.iter().find(|p| std::path::Path::new(&p.path) == original).map(|p| p.id.clone());
            if let Some(oid) = original_id {
                s.rotation.remove(&oid);
            }
            s.lib.add_or_refresh(&path)
        };
        self.refresh(false);
        if let Some(i) = id.and_then(|id| self.state.borrow().index_of(&id)) {
            self.state.borrow_mut().select_single(i);
        }
        self.refresh_selection(true);
        if self.lightbox.is_open() {
            self.show_in_viewer();
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let message = match (replaced, backup) {
            (false, _) => format!("Saved “{name}” next to the original"),
            (true, Some(b)) => format!("Replaced the original. A backup is in {}", paths::pretty(b.parent().unwrap_or(&b))),
            (true, None) => "Replaced the original".to_string(),
        };
        self.show_toast(&message);
    }

    fn toggle_favorite(self: &Rc<Self>) {
        let ids = self.state.borrow().target_ids();
        self.toggle_favorites(ids);
    }

    fn toggle_favorites(self: &Rc<Self>, ids: Vec<String>) {
        if ids.is_empty() {
            return;
        }
        // A mixed selection becomes all favourites; otherwise everything flips.
        let all_fav = ids.iter().all(|id| self.state.borrow().lib.is_favorite(id));
        for id in &ids {
            let now = self.state.borrow().lib.is_favorite(id);
            if now == all_fav
                && let Err(e) = self.state.borrow_mut().lib.toggle_favorite(id)
            {
                self.show_toast(&format!("Couldn't save favorites: {e}"));
                return;
            }
        }
        if self.state.borrow().section == "favorites" {
            self.refresh(false);
        } else {
            let s = self.state.borrow();
            for id in &ids {
                if let Some(i) = s.index_of(id) {
                    self.gallery.set_favorite(i, s.photo(i).is_some_and(|p| p.is_favorite));
                }
            }
            drop(s);
            self.update_counts();
            self.refresh_selection(false);
        }
        if self.lightbox.is_open()
            && let Some(p) = self.state.borrow().cursor_photo()
        {
            self.lightbox.set_favorite(p.is_favorite);
        }
        self.show_toast(if all_fav { "Removed from favorites" } else { "Added to favorites" });
    }

    /// The right-click menu. Right-clicking a photo outside the selection selects it first.
    fn show_context_menu(self: &Rc<Self>, idx: usize, anchor: &gtk::Widget, x: f64, y: f64) {
        {
            let mut s = self.state.borrow_mut();
            let Some(id) = s.photo(idx).map(|p| p.id.clone()) else { return };
            if !s.selected.contains(&id) {
                s.select_single(idx);
            } else {
                s.cursor = Some(id);
            }
        }
        self.refresh_selection(false);
        let (count, in_album, albums, favorite, video) = {
            let s = self.state.borrow();
            let ids = s.target_ids();
            let all_fav = ids.iter().all(|id| s.lib.is_favorite(id));
            let video = s.cursor_photo().is_some_and(|p| p.media_type == "videos");
            (
                ids.len(),
                s.album_id().is_some(),
                s.lib.albums.iter().map(|a| (a.id.clone(), a.name.clone())).collect::<Vec<_>>(),
                all_fav,
                video,
            )
        };
        let single = count == 1;

        let pop = gtk::Popover::new();
        pop.set_parent(anchor);
        pop.set_has_arrow(false);
        pop.set_halign(gtk::Align::Start);
        pop.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        let list = vbox(2);
        list.set_size_request(240, -1);
        let item = |icon: &str, text: &str, keys: &str, run: MenuAction| {
            let b = gtk::Button::new();
            b.add_css_class("flat");
            b.add_css_class("action-button");
            let content = hbox(8);
            content.append(&gtk::Image::from_icon_name(icon));
            let l = gtk::Label::new(Some(text));
            l.set_xalign(0.0);
            l.set_hexpand(true);
            content.append(&l);
            if !keys.is_empty() {
                content.append(&widgets::key_combo(keys));
            }
            b.set_child(Some(&content));
            let (a, pop) = (self.clone(), pop.clone());
            b.connect_clicked(move |_| {
                pop.popdown();
                run(&a);
            });
            b
        };
        if single {
            list.append(&item("document-open-symbolic", "Open", "Enter", Box::new(|a| a.open_viewer())));
            if !video {
                list.append(&item("document-edit-symbolic", "Edit", "E", Box::new(|a| a.open_editor())));
                list.append(&item("object-rotate-right-symbolic", "Rotate", "R", Box::new(|a| a.rotate())));
            }
        }
        let (icon, text) = if favorite {
            ("non-starred-symbolic", if single { "Remove from favorites" } else { "Remove all from favorites" })
        } else {
            ("starred-symbolic", if single { "Add to favorites" } else { "Add all to favorites" })
        };
        list.append(&item(icon, text, "F", Box::new(|a| a.toggle_favorite())));
        if !albums.is_empty() {
            list.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            let heading = label("Add to album", "dim");
            heading.set_xalign(0.0);
            heading.set_margin_start(10);
            list.append(&heading);
            let names = vbox(2);
            for (id, name) in albums {
                names.append(&item("folder-symbolic", &name, "", Box::new(move |a| a.add_to_album(&id))));
            }
            let scroll = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .propagate_natural_height(true)
                .max_content_height(150)
                .child(&names)
                .build();
            list.append(&scroll);
        }
        list.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        if in_album {
            list.append(&item("list-remove-symbolic", "Remove from this album", "", Box::new(|a| a.remove_from_album())));
        }
        if single {
            list.append(&item("edit-copy-symbolic", "Copy path", "", Box::new(|a| a.copy_path())));
        }
        list.append(&item("folder-open-symbolic", "Reveal in file manager", "Ctrl+O", Box::new(|a| a.reveal())));
        list.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        let label_text = if single { "Delete".to_string() } else { format!("Delete {count} photos") };
        let (a, p) = (self.clone(), pop.clone());
        let delete = widgets::confirm_button(&label_text, "Click again to delete", move |_| {
            p.popdown();
            a.delete_selected();
        });
        delete.add_css_class("flat");
        delete.add_css_class("action-button");
        delete.add_css_class("danger");
        if let Some(c) = delete.child() {
            c.set_halign(gtk::Align::Start);
        }
        list.append(&delete);

        pop.set_child(Some(&list));
        pop.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        pop.popup();
    }

    /// Move the selected photos to the trash and forget them here.
    fn delete_selected(self: &Rc<Self>) {
        let (ids, next) = {
            let s = self.state.borrow();
            let ids = s.target_ids();
            let first = ids.iter().filter_map(|id| s.index_of(id)).min().unwrap_or(0);
            (ids, first)
        };
        let paths: Vec<(String, String)> = {
            let s = self.state.borrow();
            ids.iter()
                .filter_map(|id| s.lib.photos.iter().find(|p| &p.id == id))
                .map(|p| (p.id.clone(), p.path.clone()))
                .collect()
        };
        let mut gone = Vec::new();
        let mut failed = 0;
        for (id, path) in paths {
            match gtk::gio::File::for_path(&path).trash(gtk::gio::Cancellable::NONE) {
                Ok(()) => gone.push(id),
                Err(_) => failed += 1,
            }
        }
        if !gone.is_empty() {
            let result = {
                let mut s = self.state.borrow_mut();
                for id in &gone {
                    s.rotation.remove(id);
                }
                s.lib.remove_photos(&gone)
            };
            if let Err(e) = result {
                self.show_toast(&format!("Couldn't save the library: {e}"));
            }
            self.state.borrow_mut().selected.clear();
            self.state.borrow_mut().cursor = None;
            self.refresh(false);
            let n = self.state.borrow().shown.len();
            if n > 0 {
                self.state.borrow_mut().select_single(next.min(n - 1));
            }
            self.refresh_selection(true);
        }
        let n = gone.len();
        let deleted = format!("Deleted {n} {} (moved to the trash)", if n == 1 { "photo" } else { "photos" });
        self.show_toast(&match (n, failed) {
            (_, 0) => deleted,
            (0, f) => format!("Couldn't delete {f} {}", if f == 1 { "photo" } else { "photos" }),
            (_, f) => format!("{deleted}. {f} couldn't be deleted"),
        });
    }

    fn rotate(self: &Rc<Self>) {
        let Some(id) = self.state.borrow().cursor.clone() else { return };
        {
            let mut s = self.state.borrow_mut();
            let angle = s.rotation.entry(id).or_insert(0);
            *angle = (*angle + 90) % 360;
        }
        if self.lightbox.is_open() {
            self.show_in_viewer();
        }
        self.refresh_selection(false);
    }

    fn reveal(self: &Rc<Self>) {
        let target = {
            let s = self.state.borrow();
            s.cursor_photo().map(|p| p.path.clone()).unwrap_or_else(|| s.lib.root.to_string_lossy().to_string())
        };
        library::reveal(&target);
    }

    fn copy_path(self: &Rc<Self>) {
        if let Some(p) = self.state.borrow().cursor_photo() {
            self.window.clipboard().set_text(&p.path);
            self.show_toast("Copied the path");
        }
    }

    fn add_to_album(self: &Rc<Self>, album: &str) {
        let ids = self.state.borrow().target_ids();
        let result = self.state.borrow_mut().lib.add_to_album(album, &ids);
        match result {
            Ok(added) => {
                self.update_counts();
                self.show_toast(&format!("Added {added} {} to the album", if added == 1 { "photo" } else { "photos" }));
            }
            Err(e) => self.show_toast(&format!("Couldn't save the album: {e}")),
        }
    }

    fn remove_from_album(self: &Rc<Self>) {
        let (album, ids) = {
            let s = self.state.borrow();
            (s.album_id().map(String::from), s.target_ids())
        };
        let Some(album) = album else { return };
        for id in ids {
            if let Err(e) = self.state.borrow_mut().lib.remove_from_album(&album, &id) {
                self.show_toast(&format!("Couldn't save the album: {e}"));
                return;
            }
        }
        self.refresh(false);
        self.show_toast("Removed from the album");
    }

    fn new_album(self: &Rc<Self>) {
        let n = self.state.borrow().lib.albums.len() + 1;
        let created = self.state.borrow_mut().lib.create_album(&format!("New album {n}"));
        match created {
            Ok(id) => {
                self.build_albums();
                self.navigate(&id);
                self.rename_album(&id);
            }
            Err(e) => self.show_toast(&format!("Couldn't create the album: {e}")),
        }
    }

    fn rename_album(self: &Rc<Self>, id: &str) {
        let Some(name) = self.state.borrow().lib.album(id).map(|a| a.name.clone()) else { return };
        let (a, id) = (self.clone(), id.to_string());
        dialog::ask_text("Rename album", "", &name, "Save", move |new| {
            if let Err(e) = a.state.borrow_mut().lib.rename_album(&id, &new) {
                a.show_toast(&format!("Couldn't save the album: {e}"));
                return;
            }
            a.build_albums();
            a.refresh(false);
        });
    }

    fn delete_album(self: &Rc<Self>, id: &str) {
        let name = self.state.borrow().lib.album(id).map(|a| a.name.clone()).unwrap_or_default();
        if let Err(e) = self.state.borrow_mut().lib.delete_album(id) {
            self.show_toast(&format!("Couldn't delete the album: {e}"));
            return;
        }
        self.build_albums();
        self.navigate("all_photos");
        self.show_toast(&format!("Deleted the album “{name}”"));
    }

    fn on_inspector(self: &Rc<Self>, action: inspector::Action) {
        use inspector::Action::*;
        match action {
            Close => self.toggle_inspector(),
            Open => self.open_viewer(),
            Edit => self.open_editor(),
            Rotate => self.rotate(),
            CopyPath => self.copy_path(),
            Reveal => self.reveal(),
            ToggleFavorite => self.toggle_favorite(),
            AddToAlbum(id) => self.add_to_album(&id),
            RemoveFromAlbum => self.remove_from_album(),
        }
    }

    fn on_lightbox(self: &Rc<Self>, action: lightbox::Action) {
        use lightbox::Action::*;
        match action {
            Close => self.close_viewer(),
            Prev => self.step_viewer(-1),
            Next => self.step_viewer(1),
            Rotate => self.rotate(),
            Reveal => self.reveal(),
            ToggleFavorite => self.toggle_favorite(),
            Edit => self.open_editor(),
            Play => {
                if let Some(p) = self.state.borrow().cursor_photo() {
                    cmd::spawn(&["xdg-open", &p.path]);
                }
            }
        }
    }

    // ----- Inspector, sidebar and sizes -----

    fn toggle_inspector(self: &Rc<Self>) {
        prefs::update(|p| p.inspector_open = !p.inspector_open);
        self.apply_inspector_visibility();
    }

    /// The info drawer, beside the grid below the bar: the grid makes room for it.
    fn apply_inspector_visibility(&self) {
        let show = prefs::get().inspector_open;
        self.inspector.root.set_visible(show);
        self.gallery.widget().set_margin_end(if show { inspector::WIDTH } else { 0 });
        if show {
            self.info_button.add_css_class("active");
        } else {
            self.info_button.remove_css_class("active");
        }
    }

    fn toggle_sidebar(self: &Rc<Self>) {
        self.temp_expanded.set(false);
        prefs::update(|p| p.sidebar_collapsed = !p.sidebar_collapsed);
        self.apply_compact(self.narrow.get() || prefs::get().sidebar_collapsed);
    }

    fn focus_search(self: &Rc<Self>) {
        if self.compact.get() {
            self.temp_expanded.set(true);
            self.apply_compact(false);
        }
        if !self.on_gallery() {
            let section = self.state.borrow().section.clone();
            self.navigate(&section);
        }
        self.search.grab_focus();
    }

    /// Hand the keyboard back to the gallery.
    fn leave_search(&self) {
        GtkWindowExt::set_focus(&self.window, None::<&gtk::Widget>);
        if self.temp_expanded.replace(false) {
            self.apply_compact(self.narrow.get() || prefs::get().sidebar_collapsed);
        }
    }

    fn apply_width(self: &Rc<Self>) {
        let w = &self.window;
        let width = if w.width() > 0 { w.width() } else { w.default_width() };
        if width <= 0 {
            return;
        }
        let narrow = width < NARROW_BELOW;
        let first = !self.nav.has_css_class("sized");
        if narrow == self.narrow.get() && !first {
            return;
        }
        self.nav.add_css_class("sized");
        self.narrow.set(narrow);
        self.gallery.set_narrow(narrow);
        self.status_hints.set_visible(!narrow);
        self.zoom_inline.set_visible(!narrow);
        self.zoom_menu.set_visible(narrow);
        for page in [Some(self.gallery_page.clone().upcast::<gtk::Widget>()), self.settings_body()].into_iter().flatten() {
            if narrow {
                page.add_css_class("narrow");
            } else {
                page.remove_css_class("narrow");
            }
        }
        self.apply_compact(narrow || prefs::get().sidebar_collapsed);
        self.apply_inspector_visibility();
    }

    fn settings_body(&self) -> Option<gtk::Widget> {
        self.settings_page.borrow().as_ref().and_then(|p| p.child()).and_then(|v| v.first_child()).and_then(|c| c.first_child())
    }

    /// The sidebar shows only icons: hide the labels, centre the icons and the toggle.
    fn apply_compact(&self, compact: bool) {
        if compact {
            self.nav.add_css_class("compact");
        } else {
            self.nav.remove_css_class("compact");
        }
        self.compact.set(compact);
        for w in self.compact_fixed.borrow().iter().chain(self.compact_items.borrow().iter()) {
            w.set_visible(!compact);
        }
        self.search_icon.set_visible(compact);
        self.collapse_button.set_tooltip_text(Some(if compact {
            "Expand the sidebar (Ctrl+B)"
        } else {
            "Collapse the sidebar (Ctrl+B)"
        }));
        centre_icons(self.nav.upcast_ref(), compact);
    }

    // ----- Toasts -----

    fn show_toast(self: &Rc<Self>, message: &str) {
        if let Some(old) = self.toast.borrow_mut().take() {
            self.overlay.remove_overlay(&old);
        }
        let l = gtk::Label::new(Some(message));
        l.set_wrap(true);
        l.set_max_width_chars(70);
        let bx = hbox(0);
        bx.add_css_class("toast");
        bx.append(&l);
        let motion = !prefs::get().reduce_motion;
        let revealer = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .transition_duration(if motion { 180 } else { 0 })
            .child(&bx)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::End)
            .can_target(false)
            .build();
        self.overlay.add_overlay(&revealer);
        revealer.set_reveal_child(true);
        *self.toast.borrow_mut() = Some(revealer.clone());
        let a = self.clone();
        glib::timeout_add_local_once(Duration::from_millis(3500), move || {
            revealer.set_reveal_child(false);
            glib::timeout_add_local_once(Duration::from_millis(if motion { 200 } else { 0 }), move || {
                // Only if it hasn't been replaced already.
                let current = a.toast.borrow().as_ref() == Some(&revealer);
                if current {
                    *a.toast.borrow_mut() = None;
                    a.overlay.remove_overlay(&revealer);
                }
            });
        });
    }
}

fn centre_icons(w: &gtk::Widget, compact: bool) {
    if w.has_css_class("nav-item")
        && let Some(content) = w.downcast_ref::<gtk::Button>().and_then(|b| b.child())
    {
        content.set_halign(if compact { gtk::Align::Center } else { gtk::Align::Fill });
    }
    if w.has_css_class("nav-collapse")
        && w.parent().is_some_and(|p| p.has_css_class("nav-head") || p.has_css_class("nav-group-row"))
    {
        w.set_halign(if compact { gtk::Align::Center } else { gtk::Align::End });
        w.set_hexpand(compact);
    }
    let mut child = w.first_child();
    while let Some(c) = child {
        centre_icons(&c, compact);
        child = c.next_sibling();
    }
}
