//! The photo editor: a full-window overlay with the picture on a canvas and a panel of
//! tools beside it. The maths and pixels live in `edit/`; this file is the widgets, the
//! gestures on the canvas, and the flow of opening, saving and cancelling.

use crate::edit::crop::{self, Handle};
use crate::edit::marks::{draw_marks, surface_from_image};
use crate::edit::pipeline;
use crate::edit::save::{self, Mode};
use crate::edit::{Colour, EditState, Filter, History, Mark, Rect};
use crate::widgets::{self, hbox, label, vbox};
use crate::{cmd, prefs, theme};
use gtk::cairo;
use gtk::prelude::*;
use gtk::{gdk, glib};
use image::RgbaImage;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

/// The preview works on a copy no bigger than this; the saved file uses the full picture.
const WORK_EDGE: u32 = 2048;
const PANEL_WIDTH: i32 = 320;
const CANVAS_PAD: f64 = 24.0;
const HANDLE_TOL: f64 = 12.0;
const MIN_CROP: f64 = 24.0;
/// How long after the last slider movement it counts as one undo step.
const PUSH_DELAY_MS: u64 = 450;

const SWATCHES: &[&str] = &["#ffffff", "#ff3b30", "#ff8c1a", "#ffd60a", "#34c759", "#32d7e6", "#0a84ff", "#8e5cf7", "#ff4fa3"];

/// What the window should do once the editor is done.
pub enum Outcome {
    Saved { path: PathBuf, original: PathBuf, backup: Option<PathBuf> },
    Closed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tool {
    Pen,
    Highlighter,
    Arrow,
    Rect,
    Ellipse,
    Text,
    Blur,
}

const TOOLS: &[(Tool, &str)] = &[
    (Tool::Pen, "Pen"),
    (Tool::Highlighter, "Highlighter"),
    (Tool::Arrow, "Arrow"),
    (Tool::Rect, "Rectangle"),
    (Tool::Ellipse, "Ellipse"),
    (Tool::Text, "Text"),
    (Tool::Blur, "Blur"),
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Aspect {
    Free,
    Original,
    Fixed(f64, f64),
}

/// A mark being drawn, before the pointer is let go.
enum Draft {
    Stroke(Vec<(f64, f64)>),
    Shape { from: (f64, f64), to: (f64, f64) },
}

enum Drag {
    Crop { handle: Handle, start: Rect },
    Mark,
}

struct Session {
    path: PathBuf,
    name: String,
    /// Bumped for every picture opened, so a late load for an earlier one is ignored.
    id: u64,
    src_dims: (u32, u32),
    src: Option<Arc<RgbaImage>>,
    work: Option<Arc<RgbaImage>>,
    history: History,
    st: EditState,
    baseline: EditState,
    crop_mode: bool,
    /// Normalised to the whole (rotated) picture.
    crop_draft: Rect,
    aspect: Aspect,
    swapped: bool,
    preview: Option<cairo::ImageSurface>,
    preview_size: (f64, f64),
    inflight: bool,
    dirty: bool,
    tool: Tool,
    colour: Colour,
    size: f64,
    draft: Option<Draft>,
    drag: Option<Drag>,
}

struct Slider {
    scale: gtk::Scale,
    readout: gtk::Label,
    reset: gtk::Button,
}

struct Inner {
    root: gtk::Box,
    canvas: gtk::DrawingArea,
    title: gtk::Label,
    size_label: gtk::Label,
    undo: gtk::Button,
    redo: gtk::Button,
    reset: gtk::Button,
    cancel: gtk::Button,
    copy: gtk::Button,
    save_primary: gtk::Button,
    save_copy_item: gtk::Button,
    save_replace_item: gtk::Button,
    tabs: gtk::Stack,
    tab_adjust: gtk::ToggleButton,
    geometry_note: gtk::Widget,
    geometry_widgets: Vec<gtk::Widget>,
    crop_enter: gtk::Button,
    crop_enter_row: gtk::Box,
    crop_applied: gtk::Box,
    crop_tools: gtk::Box,
    aspect_buttons: Vec<(Aspect, gtk::ToggleButton)>,
    resize_w: gtk::SpinButton,
    resize_h: gtk::SpinButton,
    resize_link: gtk::ToggleButton,
    resize_note: gtk::Label,
    sliders: Vec<Slider>,
    filter_buttons: Vec<(Filter, gtk::ToggleButton)>,
    tool_buttons: Vec<(Tool, gtk::ToggleButton)>,
    swatch_buttons: Vec<(Colour, gtk::Button)>,
    hex: gtk::Entry,
    custom_colour: gtk::ColorDialogButton,
    size_slider: Slider,
    image_rect: Cell<Rect>,
    s: RefCell<Option<Session>>,
    next_id: Cell<u64>,
    /// A control is being set by the program, not the user.
    updating: Cell<bool>,
    push_timer: RefCell<Option<glib::SourceId>>,
    replace_armed: Cell<bool>,
    cancel_armed: Cell<bool>,
    saving: Cell<bool>,
    notify: Rc<dyn Fn(&str)>,
    on_outcome: Rc<dyn Fn(Outcome)>,
}

#[derive(Clone)]
pub struct Editor(Rc<Inner>);

// ----- Building -----

impl Editor {
    pub fn new(notify: impl Fn(&str) + 'static, on_outcome: impl Fn(Outcome) + 'static) -> Editor {
        let root = vbox(0);
        root.add_css_class("editor");
        root.set_visible(false);

        // Top bar
        let bar = hbox(8);
        bar.add_css_class("editor-bar");
        let cancel = gtk::Button::new();
        cancel.set_child(Some(&widgets::button_content("go-previous-symbolic", "Cancel")));
        cancel.set_tooltip_text(Some("Esc"));
        bar.append(&cancel);
        let titles = vbox(0);
        titles.set_hexpand(true);
        titles.set_margin_start(8);
        let title = label("", "lightbox-title");
        title.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        let size_label = label("", "dim mono");
        titles.append(&title);
        titles.append(&size_label);
        bar.append(&titles);
        let undo = widgets::icon_button("edit-undo-symbolic", "Undo (Ctrl+Z)");
        let redo = widgets::icon_button("edit-redo-symbolic", "Redo (Ctrl+Shift+Z)");
        let copy = widgets::icon_button("edit-copy-symbolic", "Copy the edited picture");
        for b in [&copy, &undo, &redo] {
            bar.append(b);
        }
        let save_primary = gtk::Button::with_label("Save a copy");
        save_primary.add_css_class("suggested-action");
        let (save_menu, save_copy_item, save_replace_item, reset) = save_menu();
        let save_box = hbox(0);
        save_box.add_css_class("split-button");
        save_box.append(&save_primary);
        save_box.append(&save_menu);
        bar.append(&save_box);
        root.append(&bar);

        // Canvas and panel
        let canvas = gtk::DrawingArea::new();
        canvas.add_css_class("editor-canvas");
        canvas.set_hexpand(true);
        canvas.set_vexpand(true);
        canvas.set_focusable(false);

        let panel = vbox(0);
        panel.add_css_class("editor-panel");
        panel.set_size_request(PANEL_WIDTH, -1);
        let tab_adjust = gtk::ToggleButton::with_label("Adjust");
        let tab_markup = gtk::ToggleButton::with_label("Markup");
        tab_markup.set_group(Some(&tab_adjust));
        tab_adjust.set_active(true);
        for t in [&tab_adjust, &tab_markup] {
            t.add_css_class("segment");
        }
        let switcher = hbox(0);
        switcher.add_css_class("segmented");
        switcher.append(&tab_adjust);
        switcher.append(&tab_markup);
        switcher.set_halign(gtk::Align::Start);
        switcher.set_margin_bottom(4);
        panel.append(&switcher);

        let tabs = gtk::Stack::new();
        let adjust = AdjustParts::build();
        let markup = MarkupParts::build();
        let adjust_scroll = scrolled(&adjust.root);
        let markup_scroll = scrolled(&markup.root);
        tabs.add_named(&adjust_scroll, Some("adjust"));
        tabs.add_named(&markup_scroll, Some("markup"));
        // The tools' own natural width would stretch the panel; this keeps it at its width.
        let tools_clamp = crate::clamp::Clamp::new(&tabs, PANEL_WIDTH - 32);
        tools_clamp.set_vexpand(true);
        tools_clamp.set_halign(gtk::Align::Fill);
        panel.append(&tools_clamp);

        let body = hbox(0);
        body.append(&canvas);
        body.append(&panel);
        root.append(&body);

        let inner = Rc::new(Inner {
            root,
            canvas,
            title,
            size_label,
            undo,
            redo,
            reset,
            cancel,
            copy,
            save_primary,
            save_copy_item,
            save_replace_item,
            tabs,
            tab_adjust,
            geometry_note: adjust.note,
            geometry_widgets: adjust.geometry_widgets,
            crop_enter: adjust.crop_enter,
            crop_enter_row: adjust.crop_enter_row,
            crop_applied: adjust.crop_applied,
            crop_tools: adjust.crop_tools,
            aspect_buttons: adjust.aspects,
            resize_w: adjust.resize_w,
            resize_h: adjust.resize_h,
            resize_link: adjust.resize_link,
            resize_note: adjust.resize_note,
            sliders: adjust.sliders,
            filter_buttons: adjust.filters,
            tool_buttons: markup.tools,
            swatch_buttons: markup.swatches,
            hex: markup.hex,
            custom_colour: markup.buttons.custom.clone(),
            size_slider: markup.size,
            image_rect: Cell::new(Rect::new(0.0, 0.0, 1.0, 1.0)),
            s: RefCell::new(None),
            next_id: Cell::new(1),
            updating: Cell::new(false),
            push_timer: RefCell::new(None),
            replace_armed: Cell::new(false),
            cancel_armed: Cell::new(false),
            saving: Cell::new(false),
            notify: Rc::new(notify),
            on_outcome: Rc::new(on_outcome),
        });
        let editor = Editor(inner);
        editor.wire(&adjust.buttons, &markup.buttons, &tab_markup);
        editor
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.0.root
    }

    pub fn is_open(&self) -> bool {
        self.0.root.is_visible()
    }

    fn notify(&self, message: &str) {
        (self.0.notify)(message);
    }
}

fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        // External, not Never: the tools must not decide how wide the panel is.
        .hscrollbar_policy(gtk::PolicyType::External)
        .vscrollbar_policy(gtk::PolicyType::External)
        .min_content_width(PANEL_WIDTH - 32)
        .vexpand(true)
        .child(child)
        .build()
}

/// The Save menu: save a copy, replace the original, undo everything.
fn save_menu() -> (gtk::MenuButton, gtk::Button, gtk::Button, gtk::Button) {
    let menu = gtk::MenuButton::new();
    menu.set_icon_name("pan-down-symbolic");
    menu.set_tooltip_text(Some("More ways to save"));
    menu.add_css_class("split-arrow");
    let list = vbox(2);
    let make = |text: &str| {
        let b = gtk::Button::with_label(text);
        b.add_css_class("flat");
        if let Some(l) = b.child().and_downcast::<gtk::Label>() {
            l.set_xalign(0.0);
        }
        b
    };
    let copy_file = make("Save a copy");
    let replace = make("Replace original");
    let undo_all = make("Undo all changes");
    list.append(&copy_file);
    list.append(&replace);
    list.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    list.append(&undo_all);
    let popover = gtk::Popover::new();
    popover.set_child(Some(&list));
    menu.set_popover(Some(&popover));
    // The first click of Replace original only arms it, so the menu stays open for the second.
    for b in [&copy_file, &undo_all] {
        let p = popover.clone();
        b.connect_clicked(move |_| p.popdown());
    }
    (menu, copy_file, replace, undo_all)
}

/// A title and a full-width slider with a mono readout and a reset button.
fn slider_row(title: &str, min: f64, max: f64, value: f64) -> (gtk::Box, Slider) {
    let row = vbox(2);
    row.add_css_class("slider-row");
    let head = hbox(8);
    let t = label(title, "settings-option-title");
    t.set_hexpand(true);
    let readout = label("", "value-readout mono dim");
    readout.set_xalign(1.0);
    let reset = widgets::icon_button("edit-undo-symbolic", "Reset");
    reset.add_css_class("reset-button");
    head.append(&t);
    head.append(&readout);
    head.append(&reset);
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, min, max, 1.0);
    scale.set_value(value);
    scale.set_draw_value(false);
    scale.set_hexpand(true);
    row.append(&head);
    row.append(&scale);
    (row, Slider { scale, readout, reset })
}

fn chip_toggle(text: &str) -> gtk::ToggleButton {
    let b = gtk::ToggleButton::with_label(text);
    b.add_css_class("chip");
    b
}

fn chip_flow(max_per_line: u32) -> gtk::FlowBox {
    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_homogeneous(false);
    flow.set_max_children_per_line(max_per_line);
    flow.set_row_spacing(4);
    flow.set_column_spacing(4);
    flow.add_css_class("album-chips");
    flow
}

fn card() -> gtk::Box {
    let c = vbox(0);
    c.add_css_class("settings-card");
    c.set_overflow(gtk::Overflow::Hidden);
    c
}

fn pad_row(content: &impl IsA<gtk::Widget>) -> gtk::Box {
    let r = vbox(0);
    r.add_css_class("settings-option");
    r.add_css_class("card-row");
    r.add_css_class("tall");
    r.append(content);
    r
}

/// Buttons that carry out actions, kept apart from the state widgets.
struct AdjustButtons {
    rotate_left: gtk::Button,
    rotate_right: gtk::Button,
    flip_h: gtk::Button,
    flip_v: gtk::Button,
    crop_apply: gtk::Button,
    crop_cancel: gtk::Button,
    crop_swap: gtk::Button,
    crop_edit: gtk::Button,
    crop_remove: gtk::Button,
    presets: Vec<(f64, gtk::Button)>,
}

struct AdjustParts {
    root: gtk::Box,
    note: gtk::Widget,
    geometry_widgets: Vec<gtk::Widget>,
    crop_enter: gtk::Button,
    crop_enter_row: gtk::Box,
    crop_applied: gtk::Box,
    crop_tools: gtk::Box,
    aspects: Vec<(Aspect, gtk::ToggleButton)>,
    resize_w: gtk::SpinButton,
    resize_h: gtk::SpinButton,
    resize_link: gtk::ToggleButton,
    resize_note: gtk::Label,
    sliders: Vec<Slider>,
    filters: Vec<(Filter, gtk::ToggleButton)>,
    buttons: AdjustButtons,
}

impl AdjustParts {
    fn build() -> AdjustParts {
        let root = vbox(0);
        root.add_css_class("editor-tools");
        let mut geometry_widgets: Vec<gtk::Widget> = Vec::new();

        let note = widgets::label("Remove the markup to rotate, crop or resize again.", "dim");
        note.set_wrap(true);
        note.set_visible(false);
        note.set_margin_top(4);

        // Rotate and flip
        root.append(&label("Rotate and flip", "group-title"));
        let rf = hbox(4);
        rf.set_halign(gtk::Align::Start);
        let rotate_left = widgets::icon_button("object-rotate-left-symbolic", "Rotate left");
        let rotate_right = widgets::icon_button("object-rotate-right-symbolic", "Rotate right");
        let flip_h = widgets::icon_button("object-flip-horizontal-symbolic", "Flip horizontally");
        let flip_v = widgets::icon_button("object-flip-vertical-symbolic", "Flip vertically");
        for b in [&rotate_left, &rotate_right, &flip_h, &flip_v] {
            b.add_css_class("tool-button");
            rf.append(b);
            geometry_widgets.push(b.clone().upcast());
        }
        let card_rf = card();
        card_rf.append(&pad_row(&rf));
        root.append(&card_rf);
        root.append(&note);

        // Crop
        root.append(&label("Crop", "group-title"));
        let crop_card = card();
        let crop_enter = gtk::Button::new();
        crop_enter.set_child(Some(&widgets::button_content("zoom-fit-best-symbolic", "Crop")));
        crop_enter.set_halign(gtk::Align::Start);
        let crop_enter_row = pad_row(&crop_enter);
        crop_card.append(&crop_enter_row);
        geometry_widgets.push(crop_enter.clone().upcast());

        let crop_applied = vbox(0);
        let crop_applied_flow = chip_flow(2);
        let crop_edit = gtk::Button::with_label("Edit crop");
        let crop_remove = gtk::Button::with_label("Remove crop");
        crop_applied_flow.append(&crop_edit);
        crop_applied_flow.append(&crop_remove);
        crop_applied.append(&crop_applied_flow);
        let crop_applied_row = pad_row(&crop_applied);
        crop_applied_row.set_visible(false);
        crop_card.append(&crop_applied_row);
        geometry_widgets.push(crop_edit.clone().upcast());
        geometry_widgets.push(crop_remove.clone().upcast());

        let crop_tools = vbox(8);
        let flow = chip_flow(6);
        let mut aspects = Vec::new();
        let mut first: Option<gtk::ToggleButton> = None;
        for (aspect, text) in [
            (Aspect::Free, "Free"),
            (Aspect::Original, "Original"),
            (Aspect::Fixed(1.0, 1.0), "1:1"),
            (Aspect::Fixed(4.0, 3.0), "4:3"),
            (Aspect::Fixed(3.0, 2.0), "3:2"),
            (Aspect::Fixed(16.0, 9.0), "16:9"),
        ] {
            let b = chip_toggle(text);
            match &first {
                Some(f) => b.set_group(Some(f)),
                None => first = Some(b.clone()),
            }
            flow.append(&b);
            aspects.push((aspect, b));
        }
        aspects[0].1.set_active(true);
        crop_tools.append(&flow);
        let crop_buttons = chip_flow(3);
        let crop_swap = gtk::Button::with_label("Swap sides");
        let crop_cancel = gtk::Button::with_label("Cancel");
        let crop_apply = gtk::Button::with_label("Apply crop");
        crop_apply.add_css_class("suggested-action");
        for b in [&crop_swap, &crop_cancel, &crop_apply] {
            crop_buttons.append(b);
        }
        crop_tools.append(&crop_buttons);
        let crop_tools_row = pad_row(&crop_tools);
        crop_tools_row.set_visible(false);
        crop_card.append(&crop_tools_row);
        root.append(&crop_card);

        // Resize
        root.append(&label("Resize", "group-title"));
        let resize_card = card();
        let resize_w = gtk::SpinButton::with_range(1.0, 32768.0, 1.0);
        let resize_h = gtk::SpinButton::with_range(1.0, 32768.0, 1.0);
        let dims = vbox(8);
        for (text, spin) in [("Width", &resize_w), ("Height", &resize_h)] {
            spin.add_css_class("mono");
            spin.set_width_chars(5);
            spin.set_hexpand(true);
            let r = hbox(8);
            let l = label(text, "dim");
            l.set_size_request(56, -1);
            r.append(&l);
            r.append(spin);
            dims.append(&r);
        }
        let resize_link = gtk::ToggleButton::new();
        resize_link.set_icon_name("changes-prevent-symbolic");
        resize_link.set_active(true);
        resize_link.set_tooltip_text(Some("Keep the proportions"));
        resize_link.add_css_class("flat");
        let link_row = hbox(8);
        link_row.append(&resize_link);
        link_row.append(&label("Keep the proportions", "dim"));
        dims.append(&link_row);
        resize_card.append(&pad_row(&dims));
        let presets = chip_flow(4);
        let mut preset_buttons = Vec::new();
        for (k, text) in [(1.0, "Original"), (0.75, "75%"), (0.5, "50%"), (0.25, "25%")] {
            let b = gtk::Button::with_label(text);
            b.add_css_class("chip");
            presets.append(&b);
            preset_buttons.push((k, b));
        }
        resize_card.append(&pad_row(&presets));
        let resize_note = label("", "dim mono");
        resize_note.set_margin_top(4);
        for w in [resize_w.clone().upcast::<gtk::Widget>(), resize_h.clone().upcast(), resize_link.clone().upcast()] {
            geometry_widgets.push(w);
        }
        for (_, b) in &preset_buttons {
            geometry_widgets.push(b.clone().upcast());
        }
        root.append(&resize_card);
        root.append(&resize_note);

        // Light and colour
        root.append(&label("Light and color", "group-title"));
        let light = card();
        let mut sliders = Vec::new();
        for title in ["Brightness", "Contrast", "Saturation"] {
            let (row, slider) = slider_row(title, -100.0, 100.0, 0.0);
            light.append(&pad_row(&row));
            sliders.push(slider);
        }
        let filter_row = vbox(6);
        filter_row.append(&label("Filter", "settings-option-title"));
        let seg = hbox(0);
        seg.add_css_class("segmented");
        seg.set_halign(gtk::Align::Start);
        let mut filters = Vec::new();
        let mut first: Option<gtk::ToggleButton> = None;
        for (f, text) in [(Filter::None, "None"), (Filter::Mono, "Mono"), (Filter::Sepia, "Sepia")] {
            let b = gtk::ToggleButton::with_label(text);
            b.add_css_class("segment");
            match &first {
                Some(g) => b.set_group(Some(g)),
                None => first = Some(b.clone()),
            }
            seg.append(&b);
            filters.push((f, b));
        }
        filters[0].1.set_active(true);
        filter_row.append(&seg);
        light.append(&pad_row(&filter_row));
        root.append(&light);

        let buttons = AdjustButtons {
            rotate_left,
            rotate_right,
            flip_h,
            flip_v,
            crop_apply,
            crop_cancel,
            crop_swap,
            crop_edit,
            crop_remove,
            presets: preset_buttons,
        };
        AdjustParts {
            root,
            note: note.upcast(),
            geometry_widgets,
            crop_enter,
            crop_enter_row,
            crop_applied: crop_applied_row,
            crop_tools: crop_tools_row,
            aspects,
            resize_w,
            resize_h,
            resize_link,
            resize_note,
            sliders,
            filters,
            buttons,
        }
    }
}

struct MarkupButtons {
    /// Where the two-click "remove all markup" button goes (it needs the editor to exist first).
    clear_slot: gtk::Box,
    custom: gtk::ColorDialogButton,
}

struct MarkupParts {
    root: gtk::Box,
    tools: Vec<(Tool, gtk::ToggleButton)>,
    swatches: Vec<(Colour, gtk::Button)>,
    hex: gtk::Entry,
    size: Slider,
    buttons: MarkupButtons,
}

impl MarkupParts {
    fn build() -> MarkupParts {
        let root = vbox(0);
        root.add_css_class("editor-tools");

        root.append(&label("Tool", "group-title"));
        let flow = chip_flow(4);
        let mut tools = Vec::new();
        let mut first: Option<gtk::ToggleButton> = None;
        for (tool, text) in TOOLS {
            let b = chip_toggle(text);
            match &first {
                Some(f) => b.set_group(Some(f)),
                None => first = Some(b.clone()),
            }
            flow.append(&b);
            tools.push((*tool, b));
        }
        tools[0].1.set_active(true);
        root.append(&flow);
        let hint = widgets::label("Blur hides what is underneath: drag over it.", "dim");
        hint.set_wrap(true);
        hint.set_margin_top(4);
        root.append(&hint);

        root.append(&label("Color", "group-title"));
        let swatch_row = gtk::FlowBox::new();
        swatch_row.set_selection_mode(gtk::SelectionMode::None);
        swatch_row.set_max_children_per_line(12);
        swatch_row.set_homogeneous(false);
        swatch_row.set_column_spacing(2);
        swatch_row.add_css_class("album-chips");
        let mut swatches = Vec::new();
        for hex in SWATCHES {
            let colour = Colour::from_hex(hex).unwrap();
            let b = gtk::Button::new();
            b.add_css_class("swatch-button");
            let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            dot.add_css_class("swatch-dot");
            widgets::paint(&dot, hex);
            b.set_child(Some(&dot));
            b.set_tooltip_text(Some(hex));
            swatch_row.append(&b);
            swatches.push((colour, b));
        }
        root.append(&swatch_row);
        let custom_row = hbox(8);
        custom_row.set_margin_top(8);
        let custom = gtk::ColorDialogButton::new(Some(gtk::ColorDialog::new()));
        custom.set_tooltip_text(Some("Choose another color"));
        let hex = gtk::Entry::new();
        hex.add_css_class("mono");
        hex.set_width_chars(8);
        hex.set_max_length(7);
        hex.set_placeholder_text(Some("#rrggbb"));
        custom_row.append(&custom);
        custom_row.append(&hex);
        root.append(&custom_row);

        root.append(&label("Size", "group-title"));
        let size_card = card();
        let (row, size) = slider_row("Size", 1.0, 20.0, 6.0);
        // The title above already says it.
        if let Some(t) = row.first_child().and_then(|h| h.first_child()) {
            t.set_visible(false);
        }
        size.reset.set_visible(false);
        size.readout.set_hexpand(true);
        size_card.append(&pad_row(&row));
        root.append(&size_card);

        root.append(&label("Markup", "group-title"));
        let clear_slot = hbox(0);
        root.append(&clear_slot);

        MarkupParts { root, tools, swatches, hex, size, buttons: MarkupButtons { clear_slot, custom } }
    }
}

// ----- Wiring -----

impl Editor {
    fn wire(&self, a: &AdjustButtons, m: &MarkupButtons, tab_markup: &gtk::ToggleButton) {
        let i = &self.0;

        // Tabs
        let me = self.clone();
        tab_markup.connect_toggled(move |t| {
            me.0.tabs.set_visible_child_name(if t.is_active() { "markup" } else { "adjust" });
            if t.is_active() && me.crop_mode() {
                me.exit_crop(false);
            }
            me.update_cursor(None);
            me.0.canvas.queue_draw();
        });

        // Top bar
        let me = self.clone();
        i.undo.connect_clicked(move |_| me.undo());
        let me = self.clone();
        i.redo.connect_clicked(move |_| me.redo());
        let me = self.clone();
        i.reset.connect_clicked(move |_| me.reset());
        let me = self.clone();
        i.copy.connect_clicked(move |_| me.copy_to_clipboard());
        let me = self.clone();
        i.cancel.connect_clicked(move |_| me.request_cancel());
        let me = self.clone();
        i.save_primary.connect_clicked(move |_| match Mode::from_pref(&prefs::get().save_mode) {
            Mode::Copy => me.save(Mode::Copy),
            Mode::Replace => me.request_replace(),
        });
        let me = self.clone();
        i.save_copy_item.connect_clicked(move |_| me.save(Mode::Copy));
        let me = self.clone();
        i.save_replace_item.connect_clicked(move |_| me.request_replace());

        // Rotate and flip
        let me = self.clone();
        a.rotate_left.connect_clicked(move |_| me.commit_geometry(|st| st.rotate(-1)));
        let me = self.clone();
        a.rotate_right.connect_clicked(move |_| me.commit_geometry(|st| st.rotate(1)));
        let me = self.clone();
        a.flip_h.connect_clicked(move |_| me.commit_geometry(|st| st.flip_shown_horizontally()));
        let me = self.clone();
        a.flip_v.connect_clicked(move |_| me.commit_geometry(|st| st.flip_shown_vertically()));

        // Crop
        let me = self.clone();
        i.crop_enter.connect_clicked(move |_| me.enter_crop());
        let me = self.clone();
        a.crop_edit.connect_clicked(move |_| me.enter_crop());
        let me = self.clone();
        a.crop_remove.connect_clicked(move |_| me.commit_geometry(|st| st.set_crop(None)));
        let me = self.clone();
        a.crop_apply.connect_clicked(move |_| me.exit_crop(true));
        let me = self.clone();
        a.crop_cancel.connect_clicked(move |_| me.exit_crop(false));
        let me = self.clone();
        a.crop_swap.connect_clicked(move |_| {
            me.with_s(|s| s.swapped = !s.swapped);
            me.refit_aspect();
            me.0.canvas.queue_draw();
        });
        for (aspect, b) in &i.aspect_buttons {
            let (me, aspect) = (self.clone(), *aspect);
            b.connect_toggled(move |b| {
                if b.is_active() && !me.0.updating.get() {
                    me.with_s(|s| {
                        s.aspect = aspect;
                        s.swapped = false;
                    });
                    me.refit_aspect();
                    me.0.canvas.queue_draw();
                }
            });
        }

        // Resize
        let me = self.clone();
        i.resize_w.connect_value_changed(move |_| me.resize_changed(true));
        let me = self.clone();
        i.resize_h.connect_value_changed(move |_| me.resize_changed(false));
        let me = self.clone();
        i.resize_link.connect_toggled(move |b| {
            b.set_icon_name(if b.is_active() { "changes-prevent-symbolic" } else { "changes-allow-symbolic" });
            me.resize_changed(true);
        });
        for (k, b) in &a.presets {
            let (me, k) = (self.clone(), *k);
            b.connect_clicked(move |_| {
                let (bw, bh) = me.base_size();
                let target = if k >= 1.0 {
                    None
                } else {
                    Some((((bw as f64 * k).round() as u32).max(1), ((bh as f64 * k).round() as u32).max(1)))
                };
                me.commit_geometry(|st| st.resize = target);
            });
        }

        // Light and colour
        for (n, slider) in i.sliders.iter().enumerate() {
            let me = self.clone();
            slider.scale.connect_value_changed(move |sc| {
                if me.0.updating.get() {
                    return;
                }
                let v = sc.value().round() as i32;
                me.live(|st| match n {
                    0 => st.brightness = v,
                    1 => st.contrast = v,
                    _ => st.saturation = v,
                });
            });
            let sc = slider.scale.clone();
            slider.reset.connect_clicked(move |_| sc.set_value(0.0));
        }
        for (filter, b) in &i.filter_buttons {
            let (me, filter) = (self.clone(), *filter);
            b.connect_toggled(move |b| {
                if b.is_active() && !me.0.updating.get() {
                    me.commit(|st| st.filter = filter);
                }
            });
        }

        // Markup
        for (tool, b) in &i.tool_buttons {
            let (me, tool) = (self.clone(), *tool);
            b.connect_toggled(move |b| {
                if b.is_active() && !me.0.updating.get() {
                    me.with_s(|s| s.tool = tool);
                    me.update_cursor(None);
                }
            });
        }
        for (colour, b) in &i.swatch_buttons {
            let (me, colour) = (self.clone(), *colour);
            b.connect_clicked(move |_| me.set_colour(colour));
        }
        let me = self.clone();
        i.hex.connect_changed(move |e| {
            if me.0.updating.get() {
                return;
            }
            match Colour::from_hex(e.text().as_str()) {
                Some(c) => {
                    e.remove_css_class("error");
                    me.with_s(|s| s.colour = c);
                    me.sync_ui();
                }
                None => e.add_css_class("error"),
            }
        });
        let me = self.clone();
        m.custom.connect_rgba_notify(move |b| {
            if me.0.updating.get() {
                return;
            }
            let c = b.rgba();
            me.set_colour(Colour::rgb(c.red() as f64, c.green() as f64, c.blue() as f64));
        });
        let me = self.clone();
        i.size_slider.scale.connect_value_changed(move |sc| {
            me.with_s(|s| s.size = sc.value());
            me.0.size_slider.readout.set_text(&format!("{:.0}", sc.value()));
        });
        let me = self.clone();
        let clear = widgets::confirm_button("Remove all markup", "Click again to remove it all", move |_| {
            me.commit(|st| st.marks.clear())
        });
        m.clear_slot.append(&clear);

        self.wire_canvas();
    }

    fn wire_canvas(&self) {
        let canvas = &self.0.canvas;
        let me = self.clone();
        canvas.set_draw_func(move |_, cr, w, h| me.draw(cr, w as f64, h as f64));

        let drag = gtk::GestureDrag::new();
        drag.set_button(gdk::BUTTON_PRIMARY);
        let me = self.clone();
        drag.connect_drag_begin(move |_, x, y| me.drag_begin(x, y));
        let me = self.clone();
        drag.connect_drag_update(move |g, dx, dy| {
            if let Some((x, y)) = g.start_point() {
                me.drag_update(x, y, dx, dy);
            }
        });
        let me = self.clone();
        drag.connect_drag_end(move |g, dx, dy| {
            if let Some((x, y)) = g.start_point() {
                me.drag_end(x, y, dx, dy);
            }
        });
        canvas.add_controller(drag);

        let motion = gtk::EventControllerMotion::new();
        let me = self.clone();
        motion.connect_motion(move |_, x, y| me.update_cursor(Some((x, y))));
        canvas.add_controller(motion);
    }
}

// ----- State -----

impl Editor {
    fn with_s<R>(&self, f: impl FnOnce(&mut Session) -> R) -> Option<R> {
        self.0.s.borrow_mut().as_mut().map(f)
    }

    fn crop_mode(&self) -> bool {
        self.0.s.borrow().as_ref().is_some_and(|s| s.crop_mode)
    }

    fn markup_tab(&self) -> bool {
        self.0.tabs.visible_child_name().as_deref() == Some("markup")
    }

    /// The size after rotating and cropping, before any resize.
    fn base_size(&self) -> (u32, u32) {
        self.0
            .s
            .borrow()
            .as_ref()
            .map(|s| {
                let mut st = s.st.clone();
                st.resize = None;
                st.output_size(s.src_dims.0, s.src_dims.1)
            })
            .unwrap_or((1, 1))
    }

    /// Record a finished change as one undo step.
    fn commit(&self, change: impl FnOnce(&mut EditState)) {
        self.flush_push();
        self.with_s(|s| {
            change(&mut s.st);
            s.history.push(s.st.clone());
        });
        self.after_change();
    }

    /// A change to rotation, flips, crop or size: leaves crop mode first.
    fn commit_geometry(&self, change: impl FnOnce(&mut EditState)) {
        self.with_s(|s| s.crop_mode = false);
        self.commit(change);
    }

    /// A change that is still going on (a slider being dragged): shown at once, one undo step later.
    fn live(&self, change: impl FnOnce(&mut EditState)) {
        self.with_s(|s| change(&mut s.st));
        self.schedule_push();
        self.sync_ui();
        self.request_preview();
    }

    fn schedule_push(&self) {
        if let Some(id) = self.0.push_timer.borrow_mut().take() {
            id.remove();
        }
        let me = self.clone();
        let id = glib::timeout_add_local_once(Duration::from_millis(PUSH_DELAY_MS), move || {
            me.0.push_timer.borrow_mut().take();
            me.with_s(|s| s.history.push(s.st.clone()));
            me.sync_ui();
        });
        *self.0.push_timer.borrow_mut() = Some(id);
    }

    fn flush_push(&self) {
        if let Some(id) = self.0.push_timer.borrow_mut().take() {
            id.remove();
            self.with_s(|s| s.history.push(s.st.clone()));
        }
    }

    fn after_change(&self) {
        self.sync_ui();
        self.request_preview();
        self.0.canvas.queue_draw();
    }

    fn undo(&self) {
        self.flush_push();
        self.with_s(|s| {
            s.st = s.history.undo().clone();
            s.crop_mode = false;
        });
        self.after_change();
    }

    fn redo(&self) {
        self.flush_push();
        self.with_s(|s| {
            s.st = s.history.redo().clone();
            s.crop_mode = false;
        });
        self.after_change();
    }

    fn reset(&self) {
        self.flush_push();
        self.with_s(|s| {
            s.history.reset();
            s.st = s.history.current().clone();
            s.crop_mode = false;
        });
        self.after_change();
    }

    fn set_colour(&self, c: Colour) {
        self.with_s(|s| s.colour = c);
        self.sync_ui();
    }

    /// Bring every control in line with the session.
    fn sync_ui(&self) {
        let i = &self.0;
        let Some(v) = self.with_s(|s| {
            (
                s.st.clone(),
                s.history.can_undo(),
                s.history.can_redo(),
                s.crop_mode,
                s.aspect,
                s.tool,
                s.colour,
                s.size,
                s.src.is_some(),
                s.st != s.baseline,
                s.src_dims,
                s.name.clone(),
            )
        }) else {
            return;
        };
        let (st, can_undo, can_redo, crop_mode, aspect, tool, colour, size, loaded, changed, dims, name) = v;
        i.updating.set(true);

        i.title.set_text(&name);
        let (ow, oh) = st.output_size(dims.0, dims.1);
        i.size_label.set_text(&if loaded { format!("{ow} × {oh}") } else { String::new() });
        i.undo.set_sensitive(can_undo);
        i.redo.set_sensitive(can_redo);
        i.reset.set_sensitive(changed);
        i.copy.set_sensitive(loaded);
        i.save_primary.set_sensitive(loaded && st.is_edited() && !i.saving.get());
        i.save_copy_item.set_sensitive(loaded && st.is_edited() && !i.saving.get());
        i.save_replace_item.set_sensitive(loaded && st.is_edited() && !i.saving.get());
        self.refresh_save_labels();

        let locked = !st.marks.is_empty();
        for w in &i.geometry_widgets {
            w.set_sensitive(loaded && !locked);
        }
        i.geometry_note.set_visible(locked);
        i.crop_enter_row.set_visible(!crop_mode && st.crop.is_none());
        i.crop_applied.set_visible(!crop_mode && st.crop.is_some());
        i.crop_tools.set_visible(crop_mode);
        for (a, b) in &i.aspect_buttons {
            b.set_active(*a == aspect);
        }

        let base = {
            let mut t = st.clone();
            t.resize = None;
            t.output_size(dims.0, dims.1)
        };
        let (rw, rh) = st.resize.unwrap_or(base);
        i.resize_w.set_value(rw as f64);
        i.resize_h.set_value(rh as f64);
        i.resize_note.set_text(&if st.resize.is_some() { format!("Output {rw} × {rh}") } else { String::new() });
        i.resize_note.set_visible(st.resize.is_some());

        for (slider, value) in i.sliders.iter().zip([st.brightness, st.contrast, st.saturation]) {
            slider.scale.set_value(value as f64);
            slider.readout.set_text(&if value == 0 { "0".into() } else { format!("{value:+}") });
            slider.reset.set_opacity(if value == 0 { 0.0 } else { 1.0 });
            slider.reset.set_can_target(value != 0);
        }
        for (f, b) in &i.filter_buttons {
            b.set_active(*f == st.filter);
        }

        for (t, b) in &i.tool_buttons {
            b.set_active(*t == tool);
        }
        for (c, b) in &i.swatch_buttons {
            if *c == colour {
                b.add_css_class("selected");
            } else {
                b.remove_css_class("selected");
            }
        }
        let hex = to_hex(colour);
        if !i.hex.has_focus() && i.hex.text().to_lowercase() != hex {
            i.hex.set_text(&hex);
            i.hex.remove_css_class("error");
        }
        i.custom_colour.set_rgba(&gdk::RGBA::new(colour.r as f32, colour.g as f32, colour.b as f32, 1.0));
        i.size_slider.scale.set_value(size);
        i.size_slider.readout.set_text(&format!("{size:.0}"));
        i.updating.set(false);
    }

    fn refresh_save_labels(&self) {
        let i = &self.0;
        let replace_default = Mode::from_pref(&prefs::get().save_mode) == Mode::Replace;
        let armed = i.replace_armed.get();
        i.save_primary.set_label(match (replace_default, armed) {
            (false, _) => "Save a copy",
            (true, false) => "Replace original",
            (true, true) => "Click again to replace",
        });
        if replace_default {
            i.save_primary.add_css_class("destructive-action");
            i.save_primary.remove_css_class("suggested-action");
        } else {
            i.save_primary.remove_css_class("destructive-action");
            i.save_primary.add_css_class("suggested-action");
        }
        i.save_replace_item.set_label(if armed { "Click again to replace original" } else { "Replace original" });
        if armed {
            i.save_replace_item.add_css_class("armed");
        } else {
            i.save_replace_item.remove_css_class("armed");
        }
        i.cancel.set_child(Some(&widgets::button_content(
            "go-previous-symbolic",
            if i.cancel_armed.get() { "Click again to discard" } else { "Cancel" },
        )));
        if i.cancel_armed.get() {
            i.cancel.add_css_class("armed");
        } else {
            i.cancel.remove_css_class("armed");
        }
    }
}

fn to_hex(c: Colour) -> String {
    let b = |v: f64| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", b(c.r), b(c.g), b(c.b))
}

/// The size slider's meaning depends on the tool.
fn mark_size(tool: Tool, slider: f64) -> f64 {
    match tool {
        Tool::Pen | Tool::Highlighter | Tool::Arrow | Tool::Rect | Tool::Ellipse => slider * 0.002,
        Tool::Text => slider * 0.008,
        Tool::Blur => slider * 0.006,
    }
}

// ----- Opening, previews, resizing -----

impl Editor {
    /// Open `path` for editing, starting from the viewer's current turn (`rotation` degrees).
    pub fn open(&self, path: &Path, name: &str, rotation: u32) {
        let i = &self.0;
        if i.root.is_visible() {
            return;
        }
        let id = i.next_id.get();
        i.next_id.set(id + 1);
        let start = EditState { rotation: rotation % 360, ..EditState::default() };
        let (kind, session_path) = (pipeline::kind_of(path), path.to_path_buf());
        let session = Session {
            path: session_path.clone(),
            name: name.to_string(),
            id,
            src_dims: (1, 1),
            src: None,
            work: None,
            history: History::new(start.clone()),
            st: start.clone(),
            baseline: start,
            crop_mode: false,
            crop_draft: Rect::FULL,
            aspect: Aspect::Free,
            swapped: false,
            preview: None,
            preview_size: (1.0, 1.0),
            inflight: false,
            dirty: false,
            tool: Tool::Pen,
            colour: Colour::from_hex(SWATCHES[1]).unwrap(),
            size: 6.0,
            draft: None,
            drag: None,
        };
        *i.s.borrow_mut() = Some(session);
        i.replace_armed.set(false);
        i.cancel_armed.set(false);
        i.saving.set(false);
        i.tab_adjust.set_active(true);
        i.root.set_visible(true);
        self.sync_ui();
        i.canvas.queue_draw();
        if kind.is_none() {
            self.notify("Editing supports JPG, PNG and WebP pictures");
            self.finish(Outcome::Closed);
            return;
        }
        let me = self.clone();
        cmd::background(
            move || {
                pipeline::load(&session_path)
                    .map(|img| (Arc::new(img.clone()), Arc::new(pipeline::downscale(&img, WORK_EDGE))))
                    .map_err(|e| format!("{e:#}"))
            },
            move |result| me.loaded(id, result),
        );
    }

    fn loaded(&self, id: u64, result: Result<(Arc<RgbaImage>, Arc<RgbaImage>), String>) {
        let current = self.0.s.borrow().as_ref().is_some_and(|s| s.id == id);
        if !current {
            return;
        }
        match result {
            Ok((src, work)) => {
                self.with_s(|s| {
                    s.src_dims = src.dimensions();
                    s.src = Some(src);
                    s.work = Some(work);
                });
                self.sync_ui();
                self.request_preview();
            }
            Err(e) => {
                self.notify(&format!("Couldn't open the picture: {e}"));
                self.finish(Outcome::Closed);
            }
        }
    }

    fn request_preview(&self) {
        let job = self
            .with_s(|s| {
                let work = s.work.clone()?;
                if s.inflight {
                    s.dirty = true;
                    return None;
                }
                s.inflight = true;
                s.dirty = false;
                let mut preview_state = s.st.clone();
                if s.crop_mode {
                    preview_state.crop = None;
                }
                // Lines and text are drawn live over the preview; only blur changes pixels.
                preview_state.marks.retain(|m| matches!(m, Mark::Pixelate { .. }));
                Some((work, preview_state, s.id))
            })
            .flatten();
        let Some((work, preview_state, id)) = job else { return };
        let me = self.clone();
        cmd::background(move || pipeline::render(&work, &preview_state, false), move |img| me.preview_ready(id, img));
    }

    fn preview_ready(&self, id: u64, img: RgbaImage) {
        let again = self
            .with_s(|s| {
                if s.id != id {
                    return None;
                }
                s.inflight = false;
                s.preview_size = (img.width() as f64, img.height() as f64);
                s.preview = surface_from_image(&img);
                Some(s.dirty)
            })
            .flatten();
        self.0.canvas.queue_draw();
        if again == Some(true) {
            self.request_preview();
        }
    }

    fn resize_changed(&self, from_width: bool) {
        let i = &self.0;
        if i.updating.get() || self.0.s.borrow().is_none() {
            return;
        }
        let (bw, bh) = self.base_size();
        let (mut w, mut h) = (i.resize_w.value() as u32, i.resize_h.value() as u32);
        if i.resize_link.is_active() {
            if from_width {
                h = ((w as f64 * bh as f64 / bw as f64).round() as u32).max(1);
            } else {
                w = ((h as f64 * bw as f64 / bh as f64).round() as u32).max(1);
            }
        }
        let target = if (w, h) == (bw, bh) { None } else { Some((w, h)) };
        self.live(|st| st.resize = target);
    }
}

// ----- Crop -----

impl Editor {
    fn enter_crop(&self) {
        self.flush_push();
        self.with_s(|s| {
            s.crop_mode = true;
            s.crop_draft = s.st.crop.unwrap_or(Rect::new(0.05, 0.05, 0.9, 0.9));
        });
        self.refit_aspect();
        self.after_change();
    }

    /// Leave crop mode, keeping the box (`apply`) or throwing it away.
    fn exit_crop(&self, apply: bool) {
        let draft = self.with_s(|s| {
            s.crop_mode = false;
            s.crop_draft
        });
        match (apply, draft) {
            (true, Some(d)) => self.commit(|st| st.set_crop(Some(d))),
            _ => self.after_change(),
        }
    }

    /// The picture's shape after rotating, in pixels (the crop box lives on this).
    fn rotated_dims(&self) -> (f64, f64) {
        self.0
            .s
            .borrow()
            .as_ref()
            .map(|s| {
                if s.st.rotation % 180 == 90 {
                    (s.src_dims.1 as f64, s.src_dims.0 as f64)
                } else {
                    (s.src_dims.0 as f64, s.src_dims.1 as f64)
                }
            })
            .unwrap_or((1.0, 1.0))
    }

    /// Width over height of the locked crop shape, or `None` when it is free.
    fn crop_ratio(&self) -> Option<f64> {
        let (pw, ph) = self.rotated_dims();
        self.0.s.borrow().as_ref().and_then(|s| {
            let r = match s.aspect {
                Aspect::Free => return None,
                Aspect::Original => pw / ph,
                Aspect::Fixed(a, b) => a / b,
            };
            Some(if s.swapped && !matches!(s.aspect, Aspect::Original) { 1.0 / r } else { r })
        })
    }

    /// Shrink the crop box to the locked shape.
    fn refit_aspect(&self) {
        let Some(ratio) = self.crop_ratio() else { return };
        let (pw, ph) = self.rotated_dims();
        self.with_s(|s| {
            let fitted = crop::fit_ratio(s.crop_draft.scaled(pw, ph), ratio);
            s.crop_draft = fitted.scaled(1.0 / pw, 1.0 / ph);
        });
    }
}

// ----- Saving, copying, cancelling -----

impl Editor {
    fn save(&self, mode: Mode) {
        let i = &self.0;
        if i.saving.get() {
            return;
        }
        self.flush_push();
        let Some((path, st)) = self.with_s(|s| (s.path.clone(), s.st.clone())) else { return };
        let p = prefs::get();
        let opts = save::Options { mode, quality: p.jpeg_quality, backup: p.backup_originals };
        i.saving.set(true);
        i.replace_armed.set(false);
        self.sync_ui();
        i.save_primary.set_label("Saving…");
        let (me, original) = (self.clone(), path.clone());
        cmd::background(
            move || save::save(&path, &st, &opts).map_err(|e| format!("{e:#}")),
            move |result| {
                me.0.saving.set(false);
                match result {
                    Ok(saved) => me.finish(Outcome::Saved { path: saved.path, original, backup: saved.backup }),
                    Err(e) => {
                        me.notify(&format!("Couldn't save: {e}"));
                        me.sync_ui();
                    }
                }
            },
        );
    }

    /// Replacing the original needs two clicks.
    fn request_replace(&self) {
        let i = &self.0;
        if i.replace_armed.replace(true) {
            self.save(Mode::Replace);
            return;
        }
        self.refresh_save_labels();
        let me = self.clone();
        glib::timeout_add_seconds_local_once(4, move || {
            if me.0.replace_armed.replace(false) {
                me.refresh_save_labels();
            }
        });
    }

    fn request_cancel(&self) {
        let i = &self.0;
        let edited = self.with_s(|s| s.st != s.baseline).unwrap_or(false);
        if edited && !i.cancel_armed.replace(true) {
            self.refresh_save_labels();
            let me = self.clone();
            glib::timeout_add_seconds_local_once(4, move || {
                if me.0.cancel_armed.replace(false) {
                    me.refresh_save_labels();
                }
            });
            return;
        }
        self.finish(Outcome::Closed);
    }

    fn finish(&self, outcome: Outcome) {
        let i = &self.0;
        if let Some(id) = i.push_timer.borrow_mut().take() {
            id.remove();
        }
        i.root.set_visible(false);
        *i.s.borrow_mut() = None;
        (i.on_outcome)(outcome);
    }

    /// Put the edited picture on the clipboard.
    fn copy_to_clipboard(&self) {
        self.flush_push();
        let Some((src, st)) = self.with_s(|s| s.src.clone().map(|src| (src, s.st.clone()))).flatten() else { return };
        let me = self.clone();
        cmd::background(
            move || pipeline::render(&src, &st, true),
            move |img| {
                let (w, h) = img.dimensions();
                let bytes = glib::Bytes::from_owned(img.into_raw());
                let texture = gdk::MemoryTexture::new(w as i32, h as i32, gdk::MemoryFormat::R8g8b8a8, &bytes, w as usize * 4);
                me.0.canvas.clipboard().set_texture(&texture);
                me.notify("Copied the edited picture");
            },
        );
    }

    /// Editor shortcuts. Returns `Stop` when the key was used.
    pub fn handle_key(&self, key: gdk::Key, mods: gdk::ModifierType) -> glib::Propagation {
        let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
        let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);
        let typing = self
            .0
            .root
            .root()
            .and_downcast::<gtk::Window>()
            .and_then(|w| GtkWindowExt::focus(&w))
            .is_some_and(|f| f.dynamic_cast_ref::<gtk::Editable>().is_some());
        match key {
            gdk::Key::Escape => {
                if self.crop_mode() {
                    self.exit_crop(false);
                } else {
                    self.request_cancel();
                }
            }
            gdk::Key::z if ctrl && !typing && shift => self.redo(),
            gdk::Key::z if ctrl && !typing => self.undo(),
            gdk::Key::y if ctrl && !typing => self.redo(),
            gdk::Key::s if ctrl => match Mode::from_pref(&prefs::get().save_mode) {
                Mode::Copy => self.save(Mode::Copy),
                Mode::Replace => self.request_replace(),
            },
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    }
}

// ----- Canvas -----

fn norm(r: Rect, x: f64, y: f64) -> (f64, f64) {
    (((x - r.x) / r.w).clamp(0.0, 1.0), ((y - r.y) / r.h).clamp(0.0, 1.0))
}

/// The box between two corners, whichever way it was dragged.
fn corners(a: (f64, f64), b: (f64, f64)) -> Rect {
    Rect::new(a.0.min(b.0), a.1.min(b.1), (a.0 - b.0).abs(), (a.1 - b.1).abs())
}

fn cursor_for(handle: Handle) -> &'static str {
    match handle {
        Handle::TopLeft => "nw-resize",
        Handle::Top => "n-resize",
        Handle::TopRight => "ne-resize",
        Handle::Right => "e-resize",
        Handle::BottomRight => "se-resize",
        Handle::Bottom => "s-resize",
        Handle::BottomLeft => "sw-resize",
        Handle::Left => "w-resize",
        Handle::Move => "move",
    }
}

/// The mark being drawn, as it would be if the pointer were let go now.
fn draft_mark(s: &Session) -> Option<Mark> {
    let size = mark_size(s.tool, s.size);
    match (&s.draft, s.tool) {
        (Some(Draft::Stroke(points)), Tool::Pen | Tool::Highlighter) => {
            Some(Mark::Stroke { points: points.clone(), colour: s.colour, size, highlighter: s.tool == Tool::Highlighter })
        }
        (Some(Draft::Shape { from, to }), Tool::Arrow) => Some(Mark::Arrow { from: *from, to: *to, colour: s.colour, size }),
        (Some(Draft::Shape { from, to }), Tool::Rect) => Some(Mark::Rect { rect: corners(*from, *to), colour: s.colour, size }),
        (Some(Draft::Shape { from, to }), Tool::Ellipse) => {
            Some(Mark::Ellipse { rect: corners(*from, *to), colour: s.colour, size })
        }
        _ => None,
    }
}

fn palette_colour(hex: &str) -> Colour {
    Colour::from_hex(hex).unwrap_or(Colour::rgb(1.0, 1.0, 1.0))
}

impl Editor {
    fn draw(&self, cr: &cairo::Context, w: f64, h: f64) {
        let guard = self.0.s.borrow();
        let Some(s) = guard.as_ref() else { return };
        let pal = theme::current_palette();
        let (text, accent, shadow) = (palette_colour(&pal.text), palette_colour(&pal.accent), palette_colour(&pal.shadow));
        let Some(surface) = &s.preview else {
            cr.set_source_rgba(text.r, text.g, text.b, 0.6);
            cr.set_font_size(14.0);
            cr.move_to(w / 2.0 - 48.0, h / 2.0);
            let _ = cr.show_text("Loading…");
            return;
        };
        let (pw, ph) = s.preview_size;
        let k = ((w - 2.0 * CANVAS_PAD) / pw).min((h - 2.0 * CANVAS_PAD) / ph).clamp(0.01, 8.0);
        let (iw, ih) = (pw * k, ph * k);
        let (ox, oy) = ((w - iw) / 2.0, (h - ih) / 2.0);
        self.0.image_rect.set(Rect::new(ox, oy, iw, ih));

        let _ = cr.save();
        cr.translate(ox, oy);
        cr.scale(k, k);
        let _ = cr.set_source_surface(surface, 0.0, 0.0);
        cr.source().set_filter(if k < 1.0 { cairo::Filter::Good } else { cairo::Filter::Nearest });
        let _ = cr.paint();
        let _ = cr.restore();
        cr.set_source_rgba(text.r, text.g, text.b, 0.18);
        cr.set_line_width(1.0);
        cr.rectangle(ox - 0.5, oy - 0.5, iw + 1.0, ih + 1.0);
        let _ = cr.stroke();

        // Markup, drawn live over the picture.
        let _ = cr.save();
        cr.translate(ox, oy);
        draw_marks(cr, iw, ih, &s.st.marks);
        if let Some(m) = draft_mark(s) {
            draw_marks(cr, iw, ih, &[m]);
        }
        if let (Tool::Blur, Some(Draft::Shape { from, to })) = (s.tool, &s.draft) {
            let r = corners(*from, *to).scaled(iw, ih);
            cr.set_source_rgba(text.r, text.g, text.b, 0.9);
            cr.set_line_width(1.5);
            cr.set_dash(&[6.0, 4.0], 0.0);
            cr.rectangle(r.x, r.y, r.w, r.h);
            let _ = cr.stroke();
        }
        let _ = cr.restore();

        if s.crop_mode {
            let c = s.crop_draft;
            let r = Rect::new(ox + c.x * iw, oy + c.y * ih, c.w * iw, c.h * ih);
            // Dim everything outside the box.
            let _ = cr.save();
            cr.set_fill_rule(cairo::FillRule::EvenOdd);
            cr.rectangle(ox, oy, iw, ih);
            cr.rectangle(r.x, r.y, r.w, r.h);
            cr.set_source_rgba(shadow.r, shadow.g, shadow.b, 0.6);
            let _ = cr.fill();
            let _ = cr.restore();
            // Thirds
            cr.set_source_rgba(text.r, text.g, text.b, 0.35);
            cr.set_line_width(1.0);
            for n in 1..3 {
                let f = n as f64 / 3.0;
                cr.move_to(r.x + r.w * f, r.y);
                cr.line_to(r.x + r.w * f, r.bottom());
                cr.move_to(r.x, r.y + r.h * f);
                cr.line_to(r.right(), r.y + r.h * f);
            }
            let _ = cr.stroke();
            cr.set_source_rgba(accent.r, accent.g, accent.b, 1.0);
            cr.set_line_width(1.5);
            cr.rectangle(r.x, r.y, r.w, r.h);
            let _ = cr.stroke();
            // Handles
            let mid = |a: f64, b: f64| (a + b) / 2.0;
            let points = [
                (r.x, r.y),
                (mid(r.x, r.right()), r.y),
                (r.right(), r.y),
                (r.right(), mid(r.y, r.bottom())),
                (r.right(), r.bottom()),
                (mid(r.x, r.right()), r.bottom()),
                (r.x, r.bottom()),
                (r.x, mid(r.y, r.bottom())),
            ];
            for (px, py) in points {
                cr.rectangle(px - 5.0, py - 5.0, 10.0, 10.0);
                cr.set_source_rgba(text.r, text.g, text.b, 1.0);
                let _ = cr.fill_preserve();
                cr.set_source_rgba(accent.r, accent.g, accent.b, 1.0);
                cr.set_line_width(1.5);
                let _ = cr.stroke();
            }
        }
    }

    fn crop_box_px(&self) -> Option<Rect> {
        let rect = self.0.image_rect.get();
        self.0
            .s
            .borrow()
            .as_ref()
            .filter(|s| s.crop_mode)
            .map(|s| s.crop_draft.scaled(rect.w, rect.h))
            .map(|r| Rect::new(rect.x + r.x, rect.y + r.y, r.w, r.h))
    }

    fn drag_begin(&self, x: f64, y: f64) {
        let rect = self.0.image_rect.get();
        let markup = self.markup_tab();
        let crop_box = self.crop_box_px();
        self.with_s(|s| {
            s.drag = None;
            if s.src.is_none() {
                return;
            }
            if let Some(r) = crop_box {
                if let Some(handle) = crop::hit_test(r, x, y, HANDLE_TOL) {
                    s.drag = Some(Drag::Crop { handle, start: r });
                }
            } else if markup && rect.contains(x, y) {
                let p = norm(rect, x, y);
                s.draft = Some(match s.tool {
                    Tool::Pen | Tool::Highlighter => Draft::Stroke(vec![p]),
                    _ => Draft::Shape { from: p, to: p },
                });
                s.drag = Some(Drag::Mark);
            }
        });
    }

    fn drag_update(&self, x: f64, y: f64, dx: f64, dy: f64) {
        let rect = self.0.image_rect.get();
        let ratio = self.crop_ratio();
        self.with_s(|s| match s.drag {
            Some(Drag::Crop { handle, start }) => {
                let n = crop::drag(start, handle, dx, dy, rect, ratio, MIN_CROP);
                s.crop_draft = Rect::new((n.x - rect.x) / rect.w, (n.y - rect.y) / rect.h, n.w / rect.w, n.h / rect.h);
            }
            Some(Drag::Mark) => {
                let p = norm(rect, x + dx, y + dy);
                match &mut s.draft {
                    Some(Draft::Stroke(points)) => {
                        if points.last().is_none_or(|l| (l.0 - p.0).hypot(l.1 - p.1) > 0.002) {
                            points.push(p);
                        }
                    }
                    Some(Draft::Shape { to, .. }) => *to = p,
                    None => {}
                }
            }
            None => {}
        });
        self.0.canvas.queue_draw();
    }

    fn drag_end(&self, x: f64, y: f64, dx: f64, dy: f64) {
        let rect = self.0.image_rect.get();
        let finished = self.with_s(|s| {
            let was_mark = matches!(s.drag, Some(Drag::Mark));
            s.drag = None;
            let draft = s.draft.take();
            was_mark.then_some((draft, s.tool, s.colour, s.size))
        });
        let Some(Some((draft, tool, colour, slider))) = finished else {
            self.0.canvas.queue_draw();
            return;
        };
        let size = mark_size(tool, slider);
        let mark = match (draft, tool) {
            (Some(Draft::Stroke(points)), Tool::Pen | Tool::Highlighter) => {
                Some(Mark::Stroke { points, colour, size, highlighter: tool == Tool::Highlighter })
            }
            (Some(Draft::Shape { from, to }), Tool::Arrow) if (from.0 - to.0).hypot(from.1 - to.1) > 0.01 => {
                Some(Mark::Arrow { from, to, colour, size })
            }
            (Some(Draft::Shape { from, to }), Tool::Rect | Tool::Ellipse | Tool::Blur) => {
                let r = corners(from, to);
                if r.w > 0.005 && r.h > 0.005 {
                    Some(match tool {
                        Tool::Rect => Mark::Rect { rect: r, colour, size },
                        Tool::Ellipse => Mark::Ellipse { rect: r, colour, size },
                        _ => Mark::Pixelate { rect: r, size },
                    })
                } else {
                    None
                }
            }
            (Some(Draft::Shape { .. }), Tool::Text) if dx.hypot(dy) < 6.0 => {
                self.ask_text(x, y, rect);
                None
            }
            _ => None,
        };
        match mark {
            Some(m) => self.commit(|st| st.marks.push(m)),
            None => self.0.canvas.queue_draw(),
        }
    }

    fn update_cursor(&self, point: Option<(f64, f64)>) {
        let name = if let Some(r) = self.crop_box_px() {
            point.and_then(|(x, y)| crop::hit_test(r, x, y, HANDLE_TOL)).map(cursor_for).unwrap_or("default")
        } else if self.markup_tab() {
            match self.0.s.borrow().as_ref().map(|s| s.tool) {
                Some(Tool::Text) => "text",
                _ => "crosshair",
            }
        } else {
            "default"
        };
        self.0.canvas.set_cursor_from_name(Some(name));
    }

    /// A small entry at the clicked spot for the text of a Text mark.
    fn ask_text(&self, x: f64, y: f64, rect: Rect) {
        let at = norm(rect, x, y);
        let popover = gtk::Popover::new();
        popover.set_parent(&self.0.canvas);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        let entry = gtk::Entry::new();
        entry.set_placeholder_text(Some("Type, then press Enter"));
        entry.set_width_chars(24);
        popover.set_child(Some(&entry));
        let (me, pop) = (self.clone(), popover.clone());
        entry.connect_activate(move |e| {
            let text = e.text().trim().to_string();
            pop.popdown();
            if !text.is_empty() {
                let Some((colour, slider)) = me.with_s(|s| (s.colour, s.size)) else { return };
                let size = mark_size(Tool::Text, slider);
                me.commit(|st| st.marks.push(Mark::Text { at, text, colour, size }));
            }
        });
        popover.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        popover.popup();
        entry.grab_focus();
    }
}
