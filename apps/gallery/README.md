# Nexus Photos

A fast, keyboard-first photo gallery for Omarchy, built with GTK4 and Rust. It follows the
Omarchy theme live, groups photos on a timeline, and shows EXIF details in a side drawer.

## Features

- **Timeline:** group by years, months or days, or show everything in one grid. A slider sets the thumbnail size.
- **Sections:** all photos, favorites, recently added, and photos, screenshots and videos by type.
- **Albums:** create, rename and delete albums, and add one photo or a whole selection to them.
- **Search:** filter the current view by file name, camera, lens, date or format.
- **Standalone viewer:** `nexus-photos-viewer FILE` opens one photo in a light window with Open in gallery, Edit in gallery, Rotate and Reveal buttons. Settings → Opening photos makes a double-click use it.
- **Viewer:** full-size view with zoom, pan, 90° rotation (display only), and next/previous with the arrow keys.
- **Editing:** rotate, flip, crop (free or fixed ratio), resize, brightness, contrast, saturation, Mono and Sepia filters, and markup (pen, highlighter, arrow, rectangle, ellipse, text and blur). Edits can be undone and redone, copied to the clipboard, and saved as a copy or over the original.
- **Photo info:** file details, camera and lens settings, and actions in a drawer.
- **Themes:** follows the current Omarchy theme as it changes, or pick one of 15 bundled themes or your own.
- **Narrow windows:** the sidebar collapses to icons when the window is narrow, so it fits tiled at half a screen.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/design-nexus/nexus-gallery/main/install.sh | bash
```

This builds Nexus Photos with Cargo (installing Rust and GTK 4 with pacman if needed) and
installs the binary, desktop entry and icon under `~/.local`. From a clone, run `./install.sh`.

Thumbnails for videos need `ffmpegthumbnailer` or `ffmpeg`.

## Usage

```bash
nexus-photos                      # open the library (~/Pictures)
nexus-photos photo.jpg            # open a file in the viewer, with its folder as the library
nexus-photos --edit photo.jpg     # open a file straight in the editor
nexus-photos --section settings   # open a page: all_photos, favorites, recent, photos, screenshots, videos, settings
```

Launching it again focuses the open window. The photo info panel (<kbd>I</kbd>) is closed until you open it, and it remembers. Photos and videos are looked for in `~/Pictures`
and its subfolders.

## Keyboard shortcuts

| Keys | Action |
| :--- | :--- |
| <kbd>←</kbd> <kbd>→</kbd> <kbd>↑</kbd> <kbd>↓</kbd> | Move between photos |
| <kbd>Space</kbd> or <kbd>Enter</kbd> | Open the viewer |
| <kbd>Esc</kbd> | Close the viewer, clear the search, or deselect |
| <kbd>F</kbd> or <kbd>.</kbd> | Favorite |
| <kbd>E</kbd> | Edit the photo |
| <kbd>R</kbd> | Rotate the view 90° (not saved) |
| <kbd>I</kbd> | Show or hide the photo info |
| <kbd>Ctrl</kbd>+<kbd>A</kbd> | Select every photo |
| <kbd>Ctrl</kbd>+<kbd>F</kbd> or <kbd>Ctrl</kbd>+<kbd>K</kbd> | Search |
| <kbd>Ctrl</kbd>+<kbd>B</kbd> | Collapse or expand the sidebar |
| <kbd>Ctrl</kbd>+<kbd>O</kbd> | Reveal in the file manager |
| <kbd>+</kbd> <kbd>-</kbd> <kbd>0</kbd> <kbd>1</kbd> | Viewer: zoom in, zoom out, fit, 100% |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd> / <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> | Editor: undo / redo |
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | Editor: save |
| <kbd>Ctrl</kbd>+<kbd>Q</kbd> or <kbd>Ctrl</kbd>+<kbd>W</kbd> | Close |

Click selects, <kbd>Ctrl</kbd>+click adds to the selection, <kbd>Shift</kbd>+click selects a range, and a double click opens the viewer. Right-click a photo for a menu: open, edit, rotate, favorite, add to an album, copy path, reveal, and Delete (click twice), which moves the photo to the trash and removes it from the library.

## Editing

Press <kbd>E</kbd> on a photo (or use the pencil in the viewer) to open the editor. JPG, PNG and WebP can be edited.

- **Adjust:** rotate and flip, crop with a box you drag, resize by pixels or percent, and light and colour sliders.
- **Markup:** pick a tool, a colour and a size, then drag on the picture. Click with the Text tool to type. Blur hides what is underneath. Once there is markup, rotate, crop and resize are locked until it is removed.
- **Saving:** *Save a copy* writes `name edited.ext` next to the original and leaves it untouched. *Replace original* writes over the file after you click twice, and keeps the original in `~/.local/share/nexus-photos/backups/`. Settings picks which one the Save button does.
- JPEG is saved at the quality set in Settings; PNG is lossless; WebP is saved lossless, so it can be larger.
- Saved pictures don't keep camera EXIF data.

## Settings and files

Open the Settings page (bottom of the sidebar) to choose a theme, turn Glow and Reduce motion on or off, set how the editor saves, and rescan the library.

| What | Where |
| :--- | :--- |
| Preferences | `~/.config/nexus-photos/settings.toml` |
| Your themes | `~/.config/nexus-photos/themes/<name>.toml` |
| Albums and favorites | `~/.local/share/nexus-photos/` |
| Backups of replaced originals | `~/.local/share/nexus-photos/backups/` |
| Thumbnail cache | `~/.cache/nexus-photos/thumbnails/` |

A theme file has a `name` and the colours `bg`, `surface`, `text`, `dim_text`, `accent`, `border`, `muted`,
`highlight`, `danger`, `glow` and `shadow` as `#rrggbb` values, plus `light = true` for a light theme.

## Build

```bash
cargo build --release
cargo test
```

Requires Rust and GTK 4 development files.
