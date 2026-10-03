//! A small modal form with one text field, for naming things.

use crate::{widgets, window};
use gtk::prelude::*;
use std::rc::Rc;

/// Ask for a line of text. `on_ok` gets it trimmed and non-empty.
pub fn ask_text(title: &str, intro: &str, initial: &str, ok: &str, on_ok: impl Fn(String) + 'static) {
    let Some(win) = window::window() else { return };
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
