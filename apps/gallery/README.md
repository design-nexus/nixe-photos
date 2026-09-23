# Nexus Photos

**Nexus Photos** is a fast, keyboard-first desktop photo gallery application crafted for **Omarchy Linux**. It fuses the spatial, layered minimalism of the **Strata file manager** with the timeline navigation and metadata inspection features of **macOS Photos**.

Built natively with **Rust** and **QML / QtQuick (Quickshell)**, Nexus Photos runs with zero web runtime overhead, instantly adapting to your active Omarchy theme in real time.

---

## Features

- **Spatial Strata Aesthetic & Dynamic Theming**:
  - Live synchronization with Omarchy’s current color palette (`~/.local/state/omarchy/current/theme/colors.toml`).
  - Seamless theme changes across Light, Dark, Tokyo Night, Catppuccin, Gruvbox, Nord, and custom themes without restarting the app.
  - Three-pane layout: Navigation Sidebar, Fluid Photo Grid, and Collapsible EXIF Inspector Drawer.

- **macOS Photos Style Timeline & Granular Views**:
  - Group by **Days**, **Months**, **Years**, or view flat **All Photos**.
  - High-contrast count badges on timeline section headers and album items.
  - Seamless arrow key grid navigation (<kbd>←</kbd>, <kbd>→</kbd>, <kbd>↑</kbd>, <kbd>↓</kbd>) across dates, rows, and groups.
  - Smooth zoom slider controlling thumbnail grid columns (from dense contact sheets to large previews).
  - Fast background scanning and cached thumbnail generation (`~/.cache/nexus-photos/thumbnails/`).

- **Lightbox & Deep Inspection**:
  - Double-click or press <kbd>Enter</kbd> to enter full-window Lightbox modal.
  - Interactive bottom filmstrip scrubber for continuous browsing.
  - 90° lossless rotation (<kbd>R</kbd>).
  - Clean EXIF optical badge display (Camera, Lens, ISO, Focal Length, Aperture, Shutter Speed) that automatically hides when camera metadata is not present.

- **Albums & Command Palette**:
  - Custom user albums saved persistently (`~/.local/share/nexus-photos/albums.json`).
  - Create, rename, and delete custom albums directly from the sidebar.
  - Assign single photos or multi-selected batches to albums with one click.
  - **Command Palette (<kbd>Ctrl+K</kbd>)**: Instant fuzzy search across filenames, camera gear, capture dates, and formats.

- **Deep Omarchy / Strata Integration**:
  - Single-key Reveal in Strata (<kbd>Ctrl+O</kbd>): instantly opens the active photo or current folder in the Strata file manager with focus.
  - Wayland-native window management via Quickshell.

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>K</kbd> | Open Command Palette / Fuzzy Search modal |
| <kbd>Enter</kbd> | Open selected photo in Lightbox viewer |
| <kbd>Esc</kbd> | Close Modal / Close Lightbox / Deselect all |
| <kbd>I</kbd> | Toggle EXIF Inspector Drawer |
| <kbd>F</kbd> or <kbd>.</kbd> | Toggle Favorite status |
| <kbd>R</kbd> | Rotate photo clockwise (90°) |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Reveal selected photo in Strata file manager |
| <kbd>Ctrl</kbd> + <kbd>A</kbd> | Select all visible photos |
| <kbd>←</kbd> / <kbd>→</kbd> | Previous / Next photo (in Grid or Lightbox) |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Navigate grid rows across dates |
| <kbd>Ctrl</kbd> + Click | Multi-select individual photos |
| <kbd>Shift</kbd> + Click | Range-select photo batches |

---

## Architecture

Nexus Photos follows a decoupled, asynchronous client-daemon architecture:
```
┌──────────────────────────────────────────────┐
│        Quickshell / QtQuick QML UI          │
│   (Spatial Layout, Dynamic Theming, Grid,    │
│    Lightbox, Inspector, Command Palette)     │
└──────────────────────┬───────────────────────┘
                       │ JSON-Lines IPC (stdin/stdout)
┌──────────────────────▼───────────────────────┐
│              Rust Core Daemon                │
│   (Rayon parallel scanner, EXIF extractor,   │
│    fast thumbnail generator, album storage,  │
│    Strata file manager launcher)             │
└──────────────────────────────────────────────┘
```

---

## Installation

### One-Line Install (Recommended)

Run the following command in your terminal to automatically clone, build, and install Nexus Photos along with desktop icons and application shortcuts:

```bash
curl -fsSL https://raw.githubusercontent.com/design-nexus/nexus-gallery/main/install.sh | bash
```

### Prerequisites
- `rust` and `cargo`
- `git`
- `quickshell` (Wayland QtQuick shell, available on Omarchy/Arch: `omarchy-pkg-install quickshell` or AUR `quickshell-git`)

### Local Installation
If you have cloned the repository locally:
```bash
./install.sh
```

Or build and install manually:
```bash
cargo build --release
mkdir -p ~/.local/bin ~/.local/share/nexus-photos/ui ~/.local/share/icons/hicolor/scalable/apps ~/.local/share/applications
install -m 755 target/release/nexus-photos ~/.local/bin/nexus-photos
cp -r ui/* ~/.local/share/nexus-photos/ui/
cp assets/nexus-photos.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.nexus.photos.svg
cp assets/nexus-photos.svg ~/.local/share/icons/hicolor/scalable/apps/nexus-photos.svg
cp nexus-photos.desktop ~/.local/share/applications/nexus-photos.desktop
update-desktop-database ~/.local/share/applications || true
```

---

## Usage

Launch Nexus Photos from your terminal:
```bash
nexus-photos
```
Or launch **Nexus Photos** directly from your desktop application launcher (e.g. Walker, Rofi, Fuzzel).
