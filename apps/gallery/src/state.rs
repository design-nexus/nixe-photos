//! What the window is showing: the section, the search, the timeline groups and the
//! selection. Plain data, no widgets, so the rules can be tested.

use crate::grid::Span;
use crate::library::Library;
use crate::models::PhotoItem;
use std::collections::{HashMap, HashSet};

pub const LIBRARY_SECTIONS: &[(&str, &str, &str)] = &[
    ("all_photos", "All photos", "image-x-generic-symbolic"),
    ("favorites", "Favorites", "starred-symbolic"),
    ("recent", "Recently added", "document-open-recent-symbolic"),
];

pub const MEDIA_SECTIONS: &[(&str, &str, &str)] = &[
    ("photos", "Photos", "camera-photo-symbolic"),
    ("screenshots", "Screenshots", "video-display-symbolic"),
    ("videos", "Videos", "video-x-generic-symbolic"),
];

#[derive(Default)]
pub struct State {
    pub lib: Library,
    pub section: String,
    pub query: String,
    pub view_mode: String,
    /// Indices into `lib.photos`, in the order the grid shows them.
    pub shown: Vec<usize>,
    pub spans: Vec<Span>,
    /// Selected photo ids, oldest pick first.
    pub selected: Vec<String>,
    /// The photo the inspector and lightbox are about.
    pub cursor: Option<String>,
    pub rotation: HashMap<String, u32>,
    index: HashMap<String, usize>,
}

impl State {
    pub fn new(lib: Library, view_mode: String) -> State {
        State { lib, view_mode, section: "all_photos".into(), ..Default::default() }
    }

    pub fn photo(&self, shown_idx: usize) -> Option<&PhotoItem> {
        self.shown.get(shown_idx).and_then(|&i| self.lib.photos.get(i))
    }

    pub fn cursor_index(&self) -> Option<usize> {
        self.cursor.as_ref().and_then(|id| self.index.get(id).copied())
    }

    pub fn cursor_photo(&self) -> Option<&PhotoItem> {
        self.cursor_index().and_then(|i| self.photo(i))
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }

    pub fn selected_indices(&self) -> HashSet<usize> {
        self.selected.iter().filter_map(|id| self.index.get(id).copied()).collect()
    }

    /// The photos an action applies to: the selection, or just the cursor.
    pub fn target_ids(&self) -> Vec<String> {
        if !self.selected.is_empty() { self.selected.clone() } else { self.cursor.iter().cloned().collect() }
    }

    pub fn album_id(&self) -> Option<&str> {
        self.lib.album(&self.section).map(|a| a.id.as_str())
    }

    pub fn section_title(&self) -> String {
        if let Some(a) = self.lib.album(&self.section) {
            return a.name.clone();
        }
        LIBRARY_SECTIONS
            .iter()
            .chain(MEDIA_SECTIONS)
            .find(|(id, _, _)| *id == self.section)
            .map(|(_, t, _)| t.to_string())
            .unwrap_or_else(|| "All photos".into())
    }

    pub fn count(&self, section: &str) -> usize {
        let photos = &self.lib.photos;
        match section {
            "favorites" => photos.iter().filter(|p| p.is_favorite).count(),
            "photos" => photos.iter().filter(|p| p.media_type == "photos").count(),
            "screenshots" => photos.iter().filter(|p| p.media_type == "screenshots").count(),
            "videos" => photos.iter().filter(|p| p.media_type == "videos").count(),
            "all_photos" | "recent" => photos.len(),
            id => self.lib.album(id).map(|a| photos.iter().filter(|p| a.photo_ids.contains(&p.id)).count()).unwrap_or(0),
        }
    }

    /// Work out what is shown from the section, the search and the grouping, and
    /// keep the selection to photos that are still there.
    pub fn recompute(&mut self) {
        let q = self.query.trim().to_lowercase();
        let album = self.lib.album(&self.section).cloned();
        let mut shown: Vec<usize> = self
            .lib
            .photos
            .iter()
            .enumerate()
            .filter(|(_, p)| match (&album, self.section.as_str()) {
                (Some(a), _) => a.photo_ids.contains(&p.id),
                (None, "favorites") => p.is_favorite,
                (None, "photos") => p.media_type == "photos",
                (None, "screenshots") => p.media_type == "screenshots",
                (None, "videos") => p.media_type == "videos",
                _ => true,
            })
            .filter(|(_, p)| q.is_empty() || matches_query(p, &q))
            .map(|(i, _)| i)
            .collect();
        self.spans = self.group(&mut shown);
        self.shown = shown;
        self.index = self.shown.iter().enumerate().map(|(n, &i)| (self.lib.photos[i].id.clone(), n)).collect();
        self.selected.retain(|id| self.index.contains_key(id));
        if self.cursor.as_ref().is_none_or(|id| !self.index.contains_key(id)) {
            self.cursor = self.shown.first().map(|&i| self.lib.photos[i].id.clone());
            self.selected = self.cursor.iter().cloned().collect();
        }
    }

    /// Reorder `shown` so each group is contiguous (first appearance first) and describe the groups.
    fn group(&self, shown: &mut Vec<usize>) -> Vec<Span> {
        if shown.is_empty() {
            return Vec::new();
        }
        if self.view_mode == "all" {
            return vec![Span { title: String::new(), sub: String::new(), start: 0, end: shown.len() }];
        }
        let key = |p: &PhotoItem| match self.view_mode.as_str() {
            "years" => p.year.to_string(),
            "months" => format!("{} {}", month_name(p.month), p.year),
            _ => p.date_display.clone(),
        };
        let mut order: Vec<String> = Vec::new();
        let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
        for &i in shown.iter() {
            let k = key(&self.lib.photos[i]);
            if !buckets.contains_key(&k) {
                order.push(k.clone());
            }
            buckets.entry(k).or_default().push(i);
        }
        let mut spans = Vec::new();
        shown.clear();
        for k in order {
            let items = buckets.remove(&k).unwrap_or_default();
            let start = shown.len();
            shown.extend(items);
            let n = shown.len() - start;
            spans.push(Span {
                title: k,
                sub: format!("{n} {}", if n == 1 { "photo" } else { "photos" }),
                start,
                end: shown.len(),
            });
        }
        spans
    }

    pub fn select_single(&mut self, idx: usize) {
        if let Some(id) = self.photo(idx).map(|p| p.id.clone()) {
            self.selected = vec![id.clone()];
            self.cursor = Some(id);
        }
    }

    pub fn toggle(&mut self, idx: usize) {
        let Some(id) = self.photo(idx).map(|p| p.id.clone()) else { return };
        if let Some(at) = self.selected.iter().position(|s| *s == id) {
            self.selected.remove(at);
            if self.cursor.as_ref() == Some(&id) {
                self.cursor = self.selected.last().cloned();
            }
        } else {
            self.selected.push(id.clone());
            self.cursor = Some(id);
        }
    }

    /// Extend the selection from the cursor to `idx`.
    pub fn select_range(&mut self, idx: usize) {
        let from = self.cursor_index().unwrap_or(0);
        let (lo, hi) = (from.min(idx), from.max(idx));
        for n in lo..=hi {
            if let Some(id) = self.photo(n).map(|p| p.id.clone())
                && !self.selected.contains(&id)
            {
                self.selected.push(id);
            }
        }
        self.cursor = self.photo(idx).map(|p| p.id.clone());
    }

    pub fn select_all(&mut self) {
        self.selected = self.shown.iter().map(|&i| self.lib.photos[i].id.clone()).collect();
    }

    /// Back to just the first photo selected.
    pub fn clear_selection(&mut self) {
        match self.shown.first() {
            Some(_) => self.select_single(0),
            None => {
                self.selected.clear();
                self.cursor = None;
            }
        }
    }

    /// Move the cursor to `idx` (clamped) and select just it.
    pub fn move_to(&mut self, idx: isize) {
        if self.shown.is_empty() {
            return;
        }
        self.select_single(idx.clamp(0, self.shown.len() as isize - 1) as usize);
    }

    /// Step to the next or previous photo, wrapping round (for the lightbox).
    pub fn step_wrapping(&mut self, delta: isize) {
        let n = self.shown.len() as isize;
        if n == 0 {
            return;
        }
        let cur = self.cursor_index().unwrap_or(0) as isize;
        self.select_single((cur + delta).rem_euclid(n) as usize);
    }
}

fn matches_query(p: &PhotoItem, q: &str) -> bool {
    let fields = [
        Some(p.file_name.as_str()),
        p.camera_model.as_deref(),
        p.lens_model.as_deref(),
        Some(p.date_display.as_str()),
        Some(p.format.as_str()),
    ];
    q.split_whitespace().all(|term| fields.iter().flatten().any(|f| f.to_lowercase().contains(term)))
}

fn month_name(m: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    NAMES.get((m as usize).wrapping_sub(1)).copied().unwrap_or("January")
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn photo(id: &str, name: &str, year: i32, month: u32, day: u32) -> PhotoItem {
        PhotoItem {
            id: id.into(),
            file_name: name.into(),
            path: format!("/p/{name}"),
            file_size: 1,
            file_size_formatted: "1 B".into(),
            format: "JPG".into(),
            date_taken: format!("{year}-{month:02}-{day:02}"),
            date_display: format!("{month}/{day}/{year}"),
            year,
            month,
            width: 1,
            height: 1,
            thumbnail_path: String::new(),
            mtime: 0,
            camera_make: None,
            camera_model: Some("X100".into()),
            lens_model: None,
            focal_length: None,
            aperture: None,
            shutter_speed: None,
            iso: None,
            is_favorite: false,
            media_type: "photos".into(),
        }
    }

    fn state(mode: &str) -> State {
        let mut s = State { view_mode: mode.into(), section: "all_photos".into(), ..Default::default() };
        s.lib.photos = vec![photo("a", "a.jpg", 2024, 5, 1), photo("b", "b.jpg", 2023, 1, 2), photo("c", "c.jpg", 2024, 6, 3)];
        s.recompute();
        s
    }

    #[test]
    fn groups_by_year_and_keeps_order_contiguous() {
        let s = state("years");
        assert_eq!(s.spans.len(), 2);
        assert_eq!(s.spans[0].title, "2024");
        assert_eq!((s.spans[0].start, s.spans[0].end), (0, 2));
        assert_eq!(s.photo(2).unwrap().id, "b");
    }

    #[test]
    fn all_mode_has_one_headless_group() {
        let s = state("all");
        assert_eq!(s.spans.len(), 1);
        assert!(s.spans[0].title.is_empty());
    }

    #[test]
    fn query_filters_and_selection_survives() {
        let mut s = state("all");
        s.select_single(1);
        s.query = "c.jpg".into();
        s.recompute();
        assert_eq!(s.shown.len(), 1);
        assert_eq!(s.cursor.as_deref(), Some("c"));
    }

    #[test]
    fn range_selects_between_cursor_and_target() {
        let mut s = state("all");
        s.select_single(0);
        s.select_range(2);
        assert_eq!(s.selected.len(), 3);
        assert_eq!(s.cursor_index(), Some(2));
    }

    #[test]
    fn stepping_wraps() {
        let mut s = state("all");
        s.select_single(2);
        s.step_wrapping(1);
        assert_eq!(s.cursor_index(), Some(0));
        s.step_wrapping(-1);
        assert_eq!(s.cursor_index(), Some(2));
    }
}

#[cfg(test)]
mod favorites_flow {
    use super::*;

    #[test]
    fn a_favourited_photo_shows_in_the_favorites_section() {
        let dir = std::env::temp_dir().join(format!("nexus-photos-fav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // SAFETY: only this test touches the environment, before anything reads it.
        unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };
        let mut s = State::new(Library::default(), "all".into());
        let make = |id: &str| {
            let mut p = tests::photo(id, &format!("{id}.jpg"), 2024, 5, 1);
            p.is_favorite = false;
            p
        };
        s.lib.set_photos(vec![make("a"), make("b"), make("c")]);
        s.recompute();
        assert_eq!(s.count("favorites"), 0);
        assert!(s.lib.toggle_favorite("b").unwrap());
        assert_eq!(s.count("favorites"), 1);
        s.section = "favorites".into();
        s.recompute();
        assert_eq!(s.shown.len(), 1, "the favourite is listed");
        assert_eq!(s.photo(0).unwrap().id, "b");
        // Deleting a photo takes it out of the library and its favourites.
        s.lib.remove_photos(&["b".to_string()]).unwrap();
        s.recompute();
        assert_eq!(s.lib.photos.len(), 2);
        assert_eq!(s.count("favorites"), 0);
        std::fs::remove_dir_all(&dir).ok();
    }
}
