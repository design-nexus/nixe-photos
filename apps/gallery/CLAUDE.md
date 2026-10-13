# Photos — notes for working on this repo

- GTK4 (gtk4-rs 0.11) + Rust, no libadwaita. The window is one flat, monospace surface
  split by hairlines (see `STYLE.md`): the sidebar (its head holds the toggle and
  search), the gallery (its own toolbar row, the grid, the info drawer) and a status bar
  (`F1 Shortcuts` · items, settings). Settings is a card over the window
  (`settings_dialog.rs`), rebuilt from `settings_view::build` each time it opens, listing
  its groups (named by `widgets::group`).
  Every colour in `src/style.css` is a `@theme_*` token; `src/theme.rs` loads the
  palette and the stylesheet as one provider, so a theme change restyles live.
- Layout: `window.rs` owns the `App` (sidebar, pages, toasts, key handling and all
  actions; the toolbar's quick actions and its album and ⋯ menus, built when opened). `state.rs` is plain data (section, search, timeline groups, selection) with
  the unit tests. `grid.rs` is the thumbnail grid: a virtualised `ListView` whose rows are
  headings or rows of thumbnails. `lightbox.rs` is the viewer, `inspector.rs` the info
  drawer (information only, no actions), `settings_view.rs` the settings page (shown in `settings_dialog.rs`), `library.rs` scanning, favourites and
  albums (atomic writes).
- Two binaries share one library (`lib.rs`): `nixe-photos` (`main.rs`) and `nixe-viewer`, Photo
  Viewer (`src/bin/nixe-viewer.rs`, one folder of photos, hands off to the gallery with `nixe-photos [--edit] FILE`).
  `nixe-viewer` is `Lightbox` in `Mode::Standalone`, so viewer changes land in both. With no photo, `nixe-viewer` shows its
  home page (`home_page`: drop zone and the recent photos from `recent.rs`, `~/.local/share/nixe-photos/recent.json`). It reads favourites and
  rotations fresh from disk for every change; the gallery re-reads them when its window is activated.
  `actions.rs` holds what both do to files (copy, trash, save rotation, set background).
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
  calls `thumbnails::generate_thumbnail` on a worker thread when a cell is shown. They are
  512 px, upright by EXIF, written under a hidden name and renamed (never read half-made),
  and pruned after each library scan. `grid::thumbnail_texture` is the shared LRU of decoded ones.
- Ids are `models::photo_id` (FNV-1a of the path, stable across builds). `legacy_photo_id`
  (std `DefaultHasher`) only exists so `Library::migrate_ids` can move old favourites, albums
  and rotations over. Dates are `taken`: wall-clock seconds (`metadata::naive`); sort and
  filter on that, never on `date_display`.
- Scanning: `scanner::scan` reuses the previous `PhotoItem` when a file's size and mtime
  match. The result is saved to `~/.cache/nixe-photos/library.json` and loaded at start-up.
  `window.rs` watches every scanned folder with `gio::FileMonitor` and rescans 1.5 s after a
  change. Library folders are the `folders` pref (empty = `~/Pictures`).
- `grid.rs` rows are `Row::Cells { start, height, widths }`: squares or justified (`justify`).
  The date scrubber is drawn from `row_tops`/`row_labels` (estimated heights) and maps to the
  scroll adjustment proportionally. `Span::short`/`shorts` are its labels.
- Preferences: `~/.config/nixe-photos/settings.toml` (`prefs.rs`, debounced writes).
  Albums, favourites and view rotations: `~/.local/share/nixe-photos/`. `export.rs` writes
  smaller/converted copies; deleting trashes with an Undo toast (`library::untrash`). User themes:
  `~/.config/nixe-photos/themes/*.toml`.
- Checks: `cargo clippy -- -D warnings`, `cargo test`. Every path honours the XDG
  variables, so `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `XDG_CACHE_HOME` can point at a
  scratch directory to try the app without touching real data (with
  `NIXE_PHOTOS_STANDALONE=1` beside a running copy).
