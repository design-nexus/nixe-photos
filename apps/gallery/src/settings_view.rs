//! The settings page: the app's own window settings, the library, and the keys.

use crate::widgets::{self, hbox, vbox};
use crate::{clamp, cmd, paths, prefs, theme};
use gtk::prelude::*;
use std::rc::Rc;

const MAX_CONTENT_WIDTH: i32 = 920;

pub struct Info {
    pub items: usize,
    pub size: String,
    pub on_rescan: Rc<dyn Fn()>,
    /// Called after the library folders changed in the settings.
    pub on_folders: Rc<dyn Fn()>,
    /// Called after the thumbnail layout changed.
    pub on_layout: Rc<dyn Fn()>,
}

/// Every shortcut, by where it works. Shown in Settings and with `?`.
pub const SHORTCUTS: &[(&str, &[(&str, &str)])] = &[
    (
        "Gallery",
        &[
            ("Move between photos", "←, →, ↑, ↓"),
            ("Select a range", "Shift+←, Shift+→"),
            ("First and last photo", "Home, End"),
            ("A page up or down", "PgUp, PgDn"),
            ("Open in the viewer", "Space"),
            ("Favorite", "F"),
            ("Edit", "E"),
            ("Rotate the view", "R"),
            ("Rename", "F2"),
            ("Move to the trash", "Delete"),
            ("Copy the image", "Ctrl+C"),
            ("Show or hide the photo info", "I"),
            ("Select every photo", "Ctrl+A"),
            ("Bigger or smaller thumbnails", "Ctrl++, Ctrl+−"),
            ("Search", "Ctrl+F"),
            ("Collapse the sidebar", "Ctrl+B"),
            ("Reveal in the file manager", "Ctrl+O"),
            ("Show these shortcuts", "?"),
        ],
    ),
    (
        "Viewer",
        &[
            ("Previous and next photo", "←, →"),
            ("Zoom in, out, fit, 100%", "+, −, 0, 1"),
            ("Slideshow", "S"),
            ("Save the rotation to the file", "Ctrl+S"),
            ("Set as the background", "W"),
            ("Back to the gallery", "Esc"),
        ],
    ),
    ("Window", &[("Close", "Ctrl+W")]),
];

/// Key caps for a shortcut: `Ctrl+A` joined with "+", `A, B` side by side.
pub fn keys_widget(keys: &str) -> gtk::Box {
    if keys.contains(", ") {
        let b = hbox(4);
        for k in keys.split(", ") {
            b.append(&widgets::key_combo(k));
        }
        b
    } else {
        widgets::key_combo(keys)
    }
}

pub fn build(info: Info) -> (gtk::ScrolledWindow, gtk::Box) {
    let body = vbox(0);
    body.add_css_class("settings-page");
    body.set_hexpand(true);

    let (header, actions) = widgets::page_header("Settings", "How Nexus Photos looks, and where it finds your photos.");
    actions.append(&widgets::open_config_button(&[paths::prefs_file()]));
    body.append(&header);

    let on_layout = info.on_layout.clone();
    body.append(&window_group().wrapper);
    body.append(&library_group(info).wrapper);
    body.append(&viewing_group(on_layout).wrapper);
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
    let group = widgets::group("Library", "Photos and videos are looked for in these folders and their subfolders.");
    let folders = vbox(0);
    folders.add_css_class("plain-list");
    fill_folders(&folders, info.on_folders.clone());
    group.add(&folders);

    let add = gtk::Button::new();
    add.set_child(Some(&widgets::button_content("list-add-symbolic", "Add folder")));
    let (list, changed) = (folders.clone(), info.on_folders.clone());
    add.connect_clicked(move |_| {
        let (list, changed) = (list.clone(), changed.clone());
        crate::dialog::choose_folder("Add a library folder", Some(&crate::paths::home()), move |path| {
            let text = paths::pretty(&path);
            prefs::update(|p| {
                if p.folders.is_empty() {
                    // Keep the Pictures folder that was in use until now.
                    p.folders.push(paths::pretty(&crate::library::default_root()));
                }
                if !p.folders.contains(&text) {
                    p.folders.push(text.clone());
                }
            });
            fill_folders(&list, changed.clone());
            changed();
        });
    });
    group.add(&widgets::row("Add a folder", "Downloads, a camera card, a synced folder…", Some(add.upcast_ref())));

    let rescan = gtk::Button::new();
    rescan.set_child(Some(&widgets::button_content("view-refresh-symbolic", "Rescan")));
    let on_rescan = info.on_rescan;
    rescan.connect_clicked(move |_| on_rescan());
    let desc = format!("{} items, {}. New and deleted files are noticed by themselves.", info.items, info.size);
    group.add(&widgets::row("Scan the folders again", &desc, Some(rescan.upcast_ref())));
    group
}

/// One row per library folder, each with a remove button (not for the last one).
fn fill_folders(list: &gtk::Box, changed: Rc<dyn Fn()>) {
    while let Some(c) = list.first_child() {
        list.remove(&c);
    }
    let roots = crate::library::configured_roots(&prefs::get().folders);
    let single = roots.len() == 1;
    for root in roots {
        let text = paths::pretty(&root);
        let remove = widgets::icon_button("list-remove-symbolic", "Stop looking in this folder");
        remove.set_sensitive(!single);
        let missing = if root.is_dir() { "" } else { "This folder doesn't exist." };
        let row = widgets::row(&text, missing, Some(remove.upcast_ref()));
        row.add_css_class("mono-title");
        let (l, c) = (list.clone(), changed.clone());
        remove.connect_clicked(move |_| {
            prefs::update(|p| {
                if p.folders.is_empty() {
                    p.folders.push(paths::pretty(&crate::library::default_root()));
                }
                p.folders.retain(|f| *f != text);
            });
            fill_folders(&l, c.clone());
            c();
        });
        list.append(&row);
    }
}

fn viewing_group(on_layout: Rc<dyn Fn()>) -> widgets::Group {
    let group = widgets::group("Gallery and viewer", "");
    let p = prefs::get();
    let (layout, _) = widgets::choice_row(
        "Thumbnails",
        "Squares line up neatly; keeping the shape shows each photo whole.",
        vec![("square".into(), "Squares".into()), ("justified".into(), "Keep each photo's shape".into())],
        &p.layout,
        move |l| {
            prefs::update(|p| p.layout = l);
            on_layout();
        },
    );
    group.add(&layout);
    let (slides, _) = widgets::choice_row(
        "Slideshow speed",
        "How long each photo shows for. Press S in the viewer.",
        [2, 4, 6, 10, 20].iter().map(|n| (n.to_string(), format!("{n} seconds"))).collect(),
        &p.slideshow_seconds.to_string(),
        |v| prefs::update(|p| p.slideshow_seconds = v.parse().unwrap_or(4)),
    );
    group.add(&slides);
    group
}

fn opening_group() -> widgets::Group {
    let group = widgets::group("Opening photos", "What a double-click or Enter on a photo opens.");
    let installed = cmd::present("npv");
    let desc = if installed {
        "Nexus Photo Viewer (npv) opens the photo in its own light window, with buttons to bring it back here."
    } else {
        "Nexus Photo Viewer (npv) isn't installed, so photos open in the built-in viewer."
    };
    let current = if installed { prefs::get().open_with } else { "builtin".into() };
    let (row, _) = widgets::choice_row(
        "Open photos in",
        desc,
        vec![("builtin".into(), "Built-in viewer".into()), ("viewer".into(), "Nexus Photo Viewer".into())],
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
    let group = widgets::group("Keyboard shortcuts", "Press ? anywhere in the gallery to see these.");
    for (_, keys) in SHORTCUTS {
        for (title, combo) in *keys {
            group.add(&widgets::row(title, "", Some(keys_widget(combo).upcast_ref())));
        }
    }
    group
}
