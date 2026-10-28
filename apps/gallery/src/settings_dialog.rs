//! Settings as a card over the window: the groups listed on the left, the
//! chosen group's options on the right, and a search across all of them.

use gtk::prelude::*;
use nyx_ui::settings_card::{self, Spec};

pub use nyx_ui::settings_card::{close, escape, fit, focus_search, is_open};

/// Each settings group's icon and one-line summary in the dialog's list.
const GROUPS: &[(&str, &str, &str)] = &[
    ("Photos window", "applications-graphics-symbolic", "Theme, accent, motion"),
    ("Library", "folder-pictures-symbolic", "Folders, rescans"),
    ("Gallery and viewer", "view-grid-symbolic", "Thumbnail layout, slideshows"),
    ("Opening photos", "document-open-symbolic", "Built-in viewer or Photo Viewer"),
    ("Editing", "document-edit-symbolic", "Saving edits"),
    ("Keyboard shortcuts", "input-keyboard-symbolic", "Every key"),
];

const SPEC: Spec = Spec { groups: GROUPS, search_text: None };

/// Show the card over `overlay`, holding the settings page `page` (whose
/// groups are the children of `body`). It is rebuilt each time it opens.
pub fn open(overlay: &gtk::Overlay, window: &gtk::ApplicationWindow, page: &gtk::ScrolledWindow, body: &gtk::Box) {
    settings_card::open_page(overlay, window, SPEC, page.upcast_ref(), body);
}
