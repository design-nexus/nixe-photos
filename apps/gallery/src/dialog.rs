//! A small modal form with one text field, for naming things.

use crate::{widgets, window};

/// The window a dialog belongs to: the gallery's, or else whichever app window is open
/// (the stand-alone viewer).
fn parent() -> Option<gtk::Window> {
    if let Some(w) = window::window() {
        return Some(w.upcast());
    }
    let all = gtk::Window::toplevels();
    (0..all.n_items())
        .filter_map(|i| all.item(i).and_downcast::<gtk::Window>())
        .find(|w| w.is_visible() && w.is::<gtk::ApplicationWindow>())
}
use gtk::prelude::*;
use std::rc::Rc;

/// Ask for a line of text. `on_ok` gets it trimmed and non-empty.
pub fn ask_text(title: &str, intro: &str, initial: &str, ok: &str, on_ok: impl Fn(String) + 'static) {
    let Some(win) = parent() else { return };
    let dialog = gtk::Window::builder().transient_for(&win).modal(true).title(title).build();
    dialog.add_css_class("photos-window");
    dialog.set_titlebar(Some(&gtk::Box::new(gtk::Orientation::Horizontal, 0)));
    let card = widgets::vbox(12);
    card.add_css_class("dialog-card");
    card.append(&widgets::label(title, "section-title"));
    if !intro.is_empty() {
        let l = widgets::label(intro, "dim");
        l.set_wrap(true);
        card.append(&l);
    }
    let entry = gtk::Entry::new();
    entry.set_text(initial);
    entry.set_width_chars(28);
    card.append(&entry);
    let buttons = widgets::hbox(8);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let go = gtk::Button::with_label(ok);
    go.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&go);
    card.append(&buttons);
    dialog.set_child(Some(&card));

    let d = dialog.clone();
    cancel.connect_clicked(move |_| d.close());
    let submit: Rc<dyn Fn()> = Rc::new({
        let (dialog, entry) = (dialog.clone(), entry.clone());
        move || {
            let text = entry.text().trim().to_string();
            if !text.is_empty() {
                on_ok(text);
                dialog.close();
            }
        }
    });
    let s = submit.clone();
    go.connect_clicked(move |_| s());
    entry.connect_activate(move |_| submit());
    let keys = gtk::EventControllerKey::new();
    let d = dialog.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            d.close();
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    dialog.add_controller(keys);
    dialog.present();
    entry.grab_focus();
    entry.select_region(0, -1);
}

/// A modal card with a title, for the dialogs below. Esc closes it.
fn card_window(title: &str) -> Option<(gtk::Window, gtk::Box)> {
    let win = parent()?;
    let dialog = gtk::Window::builder().transient_for(&win).modal(true).title(title).build();
    dialog.add_css_class("photos-window");
    dialog.set_titlebar(Some(&gtk::Box::new(gtk::Orientation::Horizontal, 0)));
    let card = widgets::vbox(12);
    card.add_css_class("dialog-card");
    card.append(&widgets::label(title, "section-title"));
    dialog.set_child(Some(&card));
    let keys = gtk::EventControllerKey::new();
    let d = dialog.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            d.close();
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    dialog.add_controller(keys);
    Some((dialog, card))
}

/// Pick a folder with the system's file chooser.
pub fn choose_folder(title: &str, start: Option<&std::path::Path>, on_pick: impl Fn(std::path::PathBuf) + 'static) {
    let Some(win) = parent() else { return };
    let chooser = gtk::FileDialog::builder().title(title).modal(true).build();
    if let Some(s) = start.filter(|s| s.is_dir()) {
        chooser.set_initial_folder(Some(&gtk::gio::File::for_path(s)));
    }
    chooser.select_folder(Some(&win), gtk::gio::Cancellable::NONE, move |result| {
        if let Some(path) = result.ok().and_then(|f| f.path()) {
            on_pick(path);
        }
    });
}

/// Ask how to export `count` photos: the size, the format and the folder.
pub fn ask_export(count: usize, on_ok: impl Fn(crate::export::Options, std::path::PathBuf) + 'static) {
    use crate::edit::pipeline::Kind;
    let Some((dialog, card)) = card_window(&format!("Export {count} {}", if count == 1 { "photo" } else { "photos" })) else {
        return;
    };
    let intro = widgets::label("Copies are written to the folder; the originals don't change.", "dim");
    intro.set_wrap(true);
    card.append(&intro);

    let sizes: Vec<(String, String)> = [
        ("0", "Original size"),
        ("3840", "3840 px (4K)"),
        ("2560", "2560 px"),
        ("1920", "1920 px"),
        ("1280", "1280 px"),
        ("800", "800 px"),
    ]
    .iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect();
    let size = widgets::dropdown(&sizes, "0");
    let formats: Vec<(String, String)> = [("same", "Same as the original"), ("jpeg", "JPEG"), ("png", "PNG"), ("webp", "WebP")]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let format = widgets::dropdown(&formats, "same");
    let folder = Rc::new(std::cell::RefCell::new(crate::export::default_folder()));
    let folder_button = gtk::Button::with_label(&crate::paths::pretty(&folder.borrow()));
    if let Some(l) = folder_button.child().and_downcast::<gtk::Label>() {
        l.set_ellipsize(gtk::pango::EllipsizeMode::Start);
        l.set_max_width_chars(28);
    }
    let (f, b) = (folder.clone(), folder_button.clone());
    folder_button.connect_clicked(move |_| {
        let (f2, b2) = (f.clone(), b.clone());
        let start = f.borrow().clone();
        choose_folder("Export to", start.parent(), move |p| {
            b2.set_label(&crate::paths::pretty(&p));
            *f2.borrow_mut() = p;
        });
    });
    let form = gtk::Grid::new();
    form.set_row_spacing(8);
    form.set_column_spacing(12);
    for (n, (name, w)) in [
        ("Longest side", size.clone().upcast::<gtk::Widget>()),
        ("Format", format.clone().upcast()),
        ("Folder", folder_button.clone().upcast()),
    ]
    .into_iter()
    .enumerate()
    {
        let l = widgets::label(name, "dim");
        form.attach(&l, 0, n as i32, 1, 1);
        w.set_hexpand(true);
        form.attach(&w, 1, n as i32, 1, 1);
    }
    card.append(&form);

    let buttons = widgets::hbox(8);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let go = gtk::Button::with_label("Export");
    go.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&go);
    card.append(&buttons);
    let d = dialog.clone();
    cancel.connect_clicked(move |_| d.close());
    let d = dialog.clone();
    go.connect_clicked(move |_| {
        let max_edge = sizes.get(size.selected() as usize).and_then(|(v, _)| v.parse::<u32>().ok()).filter(|v| *v > 0);
        let format = match formats.get(format.selected() as usize).map(|(v, _)| v.as_str()) {
            Some("jpeg") => Some(Kind::Jpeg),
            Some("png") => Some(Kind::Png),
            Some("webp") => Some(Kind::WebP),
            _ => None,
        };
        let opts = crate::export::Options { max_edge, format, quality: crate::prefs::get().jpeg_quality };
        on_ok(opts, folder.borrow().clone());
        d.close();
    });
    dialog.present();
    go.grab_focus();
}

/// Keyboard shortcuts, by where they work.
pub type Shortcuts = [(&'static str, &'static [(&'static str, &'static str)])];

/// Every keyboard shortcut (F1 or `?`): a heading and a card of rows for each section.
pub fn show_shortcuts(shortcuts: &Shortcuts) {
    let Some((dialog, card)) = card_window("Keyboard shortcuts") else { return };
    dialog.set_default_width(520);
    let body = widgets::vbox(0);
    for (i, (section, keys)) in shortcuts.iter().enumerate() {
        let heading = widgets::label(&section.to_uppercase(), "group-title");
        if i == 0 {
            heading.add_css_class("page-first");
        }
        body.append(&heading);
        let list = widgets::vbox(0);
        list.add_css_class("settings-card");
        list.set_overflow(gtk::Overflow::Hidden);
        for (title, combo) in *keys {
            list.append(&widgets::row(title, "", Some(crate::settings_view::keys_widget(combo).upcast_ref())));
        }
        body.append(&list);
    }
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        // Scrolls without a scrollbar, which would cover the key caps.
        .vscrollbar_policy(gtk::PolicyType::External)
        .propagate_natural_height(true)
        .max_content_height(560)
        .child(&body)
        .build();
    card.append(&scroll);
    let close = gtk::Button::with_label("Close");
    close.set_halign(gtk::Align::End);
    let d = dialog.clone();
    close.connect_clicked(move |_| d.close());
    card.append(&close);
    let keys = gtk::EventControllerKey::new();
    let d = dialog.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::question {
            d.close();
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    dialog.add_controller(keys);
    dialog.present();
}
