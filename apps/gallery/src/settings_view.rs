//! The settings page: the app's own window settings, the library, and the keys.

use crate::widgets::{self, hbox, vbox};
use crate::{clamp, cmd, paths, prefs, theme};
use gtk::prelude::*;
use std::rc::Rc;

const MAX_CONTENT_WIDTH: i32 = 920;

pub struct Info {
    pub library: String,
    pub items: usize,
    pub size: String,
    pub on_rescan: Rc<dyn Fn()>,
}

pub fn build(info: Info) -> (gtk::ScrolledWindow, gtk::Box) {
    let body = vbox(0);
    body.add_css_class("settings-page");
    body.set_hexpand(true);

    let (header, actions) = widgets::page_header("Settings", "How Nexus Photos looks, and where it finds your photos.");
    actions.append(&widgets::open_config_button(&[paths::prefs_file()]));
    body.append(&header);

    body.append(&window_group().wrapper);
    body.append(&library_group(info).wrapper);
    body.append(&opening_group().wrapper);
    body.append(&editing_group().wrapper);
    body.append(&shortcuts_group().wrapper);

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::External)
        .vexpand(true)
        .child(&clamp::Clamp::new(&body, MAX_CONTENT_WIDTH))
        .build();
    (scroller, body)
}

fn window_group() -> widgets::Group {
    let group = widgets::group("Nexus Photos window", "");
    let p = prefs::get();
    let omarchy = theme::omarchy_available();
    let following = omarchy && p.mode == prefs::ThemeMode::Omarchy;

    let themes = theme::all();
    let options: Vec<(String, String)> = themes.iter().map(|t| (t.id.clone(), t.name.clone())).collect();
    let swatches = hbox(8);
    swatches.set_halign(gtk::Align::End);
    refresh_swatches(&swatches);

    let (theme_row, dropdown) =
        widgets::choice_row("Theme", "Colours for the window, when it isn't following Omarchy.", options, &p.theme, {
            let swatches = swatches.clone();
            move |id| {
                prefs::update(|p| {
                    p.mode = prefs::ThemeMode::Theme;
                    p.theme = id.clone();
                });
                theme::apply();
                refresh_swatches(&swatches);
            }
        });
    dropdown.set_sensitive(!following);

    if omarchy {
        let (row, switch) = widgets::switch_row(
            "Follow Omarchy theme",
            "Match the colours of the current Omarchy theme as it changes.",
            following,
            {
                let (dropdown, swatches) = (dropdown.clone(), swatches.clone());
                move |on| {
                    prefs::update(|p| p.mode = if on { prefs::ThemeMode::Omarchy } else { prefs::ThemeMode::Theme });
                    theme::apply();
                    dropdown.set_sensitive(!on);
                    refresh_swatches(&swatches);
                }
            },
        );
        group.add(&row);
        // Picking a theme by hand stops following.
        dropdown.connect_selected_notify(move |_| {
            if switch.is_active() {
                switch.set_active(false);
            }
        });
    }
    group.add(&theme_row);
    group.add(&widgets::row("Current colours", "", Some(swatches.upcast_ref())));

    let (glow, _) = widgets::switch_row("Glow", "A soft light around selected and focused items.", p.glow, |on| {
        prefs::update(|p| p.glow = on);
        theme::apply();
    });
    group.add(&glow);
    let (motion, _) = widgets::switch_row("Reduce motion", "Skip fades and slides.", p.reduce_motion, |on| {
        prefs::update(|p| p.reduce_motion = on);
        theme::apply();
        crate::window::apply_motion();
    });
    group.add(&motion);
    group
}

/// Squares for the window's background, surface, muted, text, accent and danger colours.
fn refresh_swatches(row: &gtk::Box) {
    while let Some(c) = row.first_child() {
        row.remove(&c);
    }
    let p = theme::current_palette();
    for (colour, name) in [
        (&p.bg, "Background"),
        (&p.surface, "Surface"),
        (&p.muted, "Muted"),
        (&p.text, "Text"),
        (&p.accent, "Accent"),
        (&p.danger, "Danger"),
    ] {
        let s = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        s.add_css_class("swatch");
        s.set_tooltip_text(Some(&format!("{name} {colour}")));
        widgets::paint(&s, colour);
        row.append(&s);
    }
}

fn library_group(info: Info) -> widgets::Group {
    let group = widgets::group("Library", "");
    let path = gtk::Label::new(Some(&info.library));
    path.add_css_class("mono");
    path.add_css_class("dim");
    path.set_ellipsize(gtk::pango::EllipsizeMode::Start);
    path.set_xalign(1.0);
    path.set_max_width_chars(36);
    group.add(&widgets::row(
        "Photo folder",
        "Where photos and videos are looked for, including subfolders.",
        Some(path.upcast_ref()),
    ));
    let rescan = gtk::Button::new();
    rescan.set_child(Some(&widgets::button_content("view-refresh-symbolic", "Rescan")));
    let on_rescan = info.on_rescan;
    rescan.connect_clicked(move |_| on_rescan());
    let desc = format!("{} items, {}", info.items, info.size);
    group.add(&widgets::row("Scan the folder again", &desc, Some(rescan.upcast_ref())));
    group
}

fn opening_group() -> widgets::Group {
    let group = widgets::group("Opening photos", "What a double-click or Enter on a photo opens.");
    let installed = cmd::present("nexus-photos-viewer");
    let desc = if installed {
        "The stand-alone viewer opens in its own light window, with buttons to bring the photo back here."
    } else {
        "The stand-alone viewer (nexus-photos-viewer) isn't installed, so photos open in the built-in viewer."
    };
    let current = if installed { prefs::get().open_with } else { "builtin".into() };
    let (row, _) = widgets::choice_row(
        "Open photos in",
        desc,
        vec![("builtin".into(), "Built-in viewer".into()), ("viewer".into(), "Standalone viewer".into())],
        &current,
        |with| prefs::update(|p| p.open_with = with),
    );
    group.add(&row);
    group
}

fn editing_group() -> widgets::Group {
    let group = widgets::group("Editing", "Rotate, crop, resize, adjust and mark up photos. Press E on a photo.");
    let p = prefs::get();
    let (save, _) = widgets::choice_row(
        "Save button",
        "Save a copy keeps the original as it is. Replace original asks you to click twice.",
        vec![("copy".into(), "Save a copy".into()), ("replace".into(), "Replace original".into())],
        &p.save_mode,
        |mode| prefs::update(|p| p.save_mode = mode),
    );
    group.add(&save);

    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 60.0, 100.0, 1.0);
    scale.set_value(p.jpeg_quality as f64);
    scale.set_draw_value(false);
    scale.set_width_request(180);
    let readout = gtk::Label::new(Some(&p.jpeg_quality.to_string()));
    readout.add_css_class("value-readout");
    readout.set_xalign(1.0);
    let r = readout.clone();
    scale.connect_value_changed(move |s| {
        let q = s.value().round() as u8;
        r.set_text(&q.to_string());
        prefs::update(|p| p.jpeg_quality = q);
    });
    let control = hbox(10);
    control.append(&scale);
    control.append(&readout);
    group.add(&widgets::row(
        "JPEG quality",
        "Higher keeps more detail and makes bigger files. PNG and WebP are saved without loss.",
        Some(control.upcast_ref()),
    ));

    let (backup, _) = widgets::switch_row(
        "Back up originals before replacing",
        "Keeps a copy in ~/.local/share/nexus-photos/backups.",
        p.backup_originals,
        |on| prefs::update(|p| p.backup_originals = on),
    );
    group.add(&backup);
    group
}

fn shortcuts_group() -> widgets::Group {
    let group = widgets::group("Keyboard shortcuts", "");
    for (title, keys) in [
        ("Move between photos", "←, →, ↑, ↓"),
        ("Open in the viewer", "Space"),
        ("Favorite", "F"),
        ("Edit", "E"),
        ("Rotate the view", "R"),
        ("Show or hide the photo info", "I"),
        ("Select every photo", "Ctrl+A"),
        ("Search", "Ctrl+F"),
        ("Collapse the sidebar", "Ctrl+B"),
        ("Reveal in the file manager", "Ctrl+O"),
        ("Zoom in the viewer", "+, −, 0"),
        ("Close", "Ctrl+W"),
    ] {
        let combo = if keys.contains(", ") {
            let b = hbox(4);
            for k in keys.split(", ") {
                b.append(&widgets::key_cap(k));
            }
            b
        } else {
            widgets::key_combo(keys)
        };
        group.add(&widgets::row(title, "", Some(combo.upcast_ref())));
    }
    group
}
