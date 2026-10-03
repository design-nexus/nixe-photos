# Nexus Photos — notes for working on this repo

- GTK4 (gtk4-rs 0.11) + Rust, no libadwaita. The look follows `~/Projects/STYLE.md`;
  the reference implementation is `~/Projects/settings` (`src/theme.rs`, `src/style.css`).
  Every colour in `src/style.css` is a `@theme_*` token; `src/theme.rs` loads the
  palette and the stylesheet as one provider, so a theme change restyles live.
- Layout: `window.rs` owns the `App` (sidebar, pages, toasts, key handling and all
  actions). `state.rs` is plain data (section, search, timeline groups, selection) with
  the unit tests. `grid.rs` is the thumbnail grid: a virtualised `ListView` whose rows are
  headings or rows of thumbnails. `lightbox.rs` is the viewer, `inspector.rs` the info
  drawer, `settings_view.rs` the settings page, `library.rs` scanning, favourites and
  albums (atomic writes).
- Two binaries share one library (`lib.rs`): `nexus-photos` (`main.rs`) and `nexus-photos-viewer`
  (`src/bin/nexus-photos-viewer.rs`, one photo, hands off to the gallery with `nexus-photos [--edit] FILE`).
  `stage.rs` is the picture area (fit/zoom/pan/pinch) used by both viewers. The `open_with` pref
  (`builtin`/`viewer`) picks what a double-click in the gallery opens; CLI file arguments always use the built-in viewer.
- Editing: `src/edit/` has no widgets. `mod.rs` is `EditState` (rotation, flips, crop, resize,
  adjustments, markup) and `History` (undo/redo); `crop.rs` is the crop-box maths in pixel
  units; `pipeline.rs` renders a state onto pixels in a fixed order (flips, rotation, crop,
  resize, adjustments, blur, markup) and encodes; `marks.rs` draws markup with cairo/pango from
  normalised coordinates, so the live preview and the saved file match; `save.rs` writes a
  copy or replaces the original (backup first, atomic). `editor.rs` is the overlay: the canvas,
  gestures, panels and the save/cancel flows. The preview renders a ≤2048 px copy on a worker
  thread; only blur changes preview pixels, other marks are drawn live over it. Markup and
  geometry exclude each other (marks are placed on the cropped picture).
- Thumbnails are made on demand: a scan only reads metadata (`metadata.rs`), and the grid
  calls `thumbnails::generate_thumbnail` on a worker thread when a cell is shown.
- Preferences: `~/.config/nexus-photos/settings.toml` (`prefs.rs`, debounced writes).
  Albums and favourites: `~/.local/share/nexus-photos/`. User themes:
  `~/.config/nexus-photos/themes/*.toml`.
- Checks: `cargo clippy -- -D warnings`, `cargo test`. Every path honours the XDG
  variables, so `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `XDG_CACHE_HOME` can point at a
  scratch directory to try the app without touching real data. To drive it without a mouse,
  run with `GDK_BACKEND=x11` and use cua-driver's accessibility clicks (Wayland windows aren't
  visible to it); its drag and hotkey calls need foreground input, so don't use them.
