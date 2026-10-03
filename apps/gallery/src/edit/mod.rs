//! Photo editing: what the user has asked for (`EditState`), the maths for cropping,
//! the image pipeline that applies it, and the drawing of markup. No widgets in here.

pub mod crop;
pub mod marks;
pub mod pipeline;
pub mod save;

/// A rectangle. In an `EditState` it is normalised: 0.0 to 1.0 of the image's width and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const FULL: Rect = Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };

    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }

    /// The same rectangle scaled by `sx`, `sy` (normalised to pixels and back).
    pub fn scaled(&self, sx: f64, sy: f64) -> Rect {
        Rect::new(self.x * sx, self.y * sy, self.w * sx, self.h * sy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    None,
    Mono,
    Sepia,
}

/// An RGB colour, each channel 0.0 to 1.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colour {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Colour {
    pub const fn rgb(r: f64, g: f64, b: f64) -> Colour {
        Colour { r, g, b }
    }

    /// Parse `#rrggbb`.
    pub fn from_hex(hex: &str) -> Option<Colour> {
        let h = hex.strip_prefix('#')?;
        if h.len() != 6 {
            return None;
        }
        let v = u32::from_str_radix(h, 16).ok()?;
        Some(Colour::rgb(((v >> 16) & 0xff) as f64 / 255.0, ((v >> 8) & 0xff) as f64 / 255.0, (v & 0xff) as f64 / 255.0))
    }

    /// Black or white, whichever reads better on this colour.
    pub fn contrast(&self) -> Colour {
        if 0.299 * self.r + 0.587 * self.g + 0.114 * self.b > 0.55 {
            Colour::rgb(0.0, 0.0, 0.0)
        } else {
            Colour::rgb(1.0, 1.0, 1.0)
        }
    }
}

/// One piece of markup. Points are normalised to the (cropped) image; sizes are a
/// fraction of its shorter side, so the preview and the full-size result match.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    Stroke {
        points: Vec<(f64, f64)>,
        colour: Colour,
        size: f64,
        highlighter: bool,
    },
    Arrow {
        from: (f64, f64),
        to: (f64, f64),
        colour: Colour,
        size: f64,
    },
    Rect {
        rect: Rect,
        colour: Colour,
        size: f64,
    },
    Ellipse {
        rect: Rect,
        colour: Colour,
        size: f64,
    },
    /// `size` is the text height.
    Text {
        at: (f64, f64),
        text: String,
        colour: Colour,
        size: f64,
    },
    /// Hides what is underneath behind big blocks. `size` is the block size.
    Pixelate {
        rect: Rect,
        size: f64,
    },
}

/// Everything the user has asked for, applied to the original in a fixed order:
/// flips, rotation, crop, resize, adjustments, then markup.
#[derive(Clone, Debug, PartialEq)]
pub struct EditState {
    /// Clockwise degrees: 0, 90, 180 or 270.
    pub rotation: u32,
    /// Flips happen before the rotation (in the original's own frame).
    pub flip_h: bool,
    pub flip_v: bool,
    /// Normalised to the flipped and rotated image.
    pub crop: Option<Rect>,
    /// Output size in pixels; `None` keeps the size after cropping.
    pub resize: Option<(u32, u32)>,
    /// -100 to 100 each; 0 changes nothing.
    pub brightness: i32,
    pub contrast: i32,
    pub saturation: i32,
    pub filter: Filter,
    pub marks: Vec<Mark>,
}

impl Default for EditState {
    fn default() -> Self {
        EditState {
            rotation: 0,
            flip_h: false,
            flip_v: false,
            crop: None,
            resize: None,
            brightness: 0,
            contrast: 0,
            saturation: 0,
            filter: Filter::None,
            marks: Vec::new(),
        }
    }
}

impl EditState {
    /// Turn what is shown 90° clockwise (`quarter_turns` may be negative).
    pub fn rotate(&mut self, quarter_turns: i32) {
        self.rotation = (self.rotation as i32 + 90 * quarter_turns).rem_euclid(360) as u32;
        self.geometry_changed();
    }

    /// Mirror what is shown, left to right. When the picture is on its side that is the
    /// original's vertical axis.
    pub fn flip_shown_horizontally(&mut self) {
        if self.rotation.is_multiple_of(180) {
            self.flip_h = !self.flip_h;
        } else {
            self.flip_v = !self.flip_v;
        }
        self.geometry_changed();
    }

    pub fn flip_shown_vertically(&mut self) {
        if self.rotation.is_multiple_of(180) {
            self.flip_v = !self.flip_v;
        } else {
            self.flip_h = !self.flip_h;
        }
        self.geometry_changed();
    }

    /// A rotation or flip makes an old crop and size meaningless.
    fn geometry_changed(&mut self) {
        self.crop = None;
        self.resize = None;
    }

    pub fn set_crop(&mut self, crop: Option<Rect>) {
        self.crop = crop.filter(|c| c.w < 0.9999 || c.h < 0.9999 || c.x > 0.0001 || c.y > 0.0001);
        self.resize = None;
    }

    pub fn has_adjustments(&self) -> bool {
        self.brightness != 0 || self.contrast != 0 || self.saturation != 0 || self.filter != Filter::None
    }

    /// Anything that moves pixels around. Markup is placed on the result, so these stay fixed while it exists.
    pub fn has_geometry(&self) -> bool {
        self.rotation != 0 || self.flip_h || self.flip_v || self.crop.is_some() || self.resize.is_some()
    }

    pub fn is_edited(&self) -> bool {
        self.has_geometry() || self.has_adjustments() || !self.marks.is_empty()
    }

    /// Size in pixels after rotating, cropping and resizing a `w`×`h` original.
    pub fn output_size(&self, w: u32, h: u32) -> (u32, u32) {
        let (w, h) = if self.rotation % 180 == 90 { (h, w) } else { (w, h) };
        let (w, h) = match self.crop {
            Some(c) => (((w as f64 * c.w).round() as u32).max(1), ((h as f64 * c.h).round() as u32).max(1)),
            None => (w, h),
        };
        self.resize.unwrap_or((w, h))
    }
}

/// Undo and redo: a snapshot after each finished action.
pub struct History {
    states: Vec<EditState>,
    at: usize,
}

impl Default for History {
    fn default() -> Self {
        History { states: vec![EditState::default()], at: 0 }
    }
}

impl History {
    pub fn new(start: EditState) -> History {
        History { states: vec![start], at: 0 }
    }

    pub fn current(&self) -> &EditState {
        &self.states[self.at]
    }

    /// Record a new state (dropping anything that could have been redone). Does nothing if it is unchanged.
    pub fn push(&mut self, state: EditState) {
        if state == *self.current() {
            return;
        }
        self.states.truncate(self.at + 1);
        self.states.push(state);
        self.at += 1;
    }

    pub fn can_undo(&self) -> bool {
        self.at > 0
    }

    pub fn can_redo(&self) -> bool {
        self.at + 1 < self.states.len()
    }

    pub fn undo(&mut self) -> &EditState {
        self.at = self.at.saturating_sub(1);
        self.current()
    }

    pub fn redo(&mut self) -> &EditState {
        if self.can_redo() {
            self.at += 1;
        }
        self.current()
    }

    /// Back to the start, as one undoable step.
    pub fn reset(&mut self) {
        let first = self.states[0].clone();
        self.push(first);
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn rotation_wraps_and_clears_crop() {
        let mut s = EditState::default();
        s.set_crop(Some(Rect::new(0.1, 0.1, 0.5, 0.5)));
        s.rotate(-1);
        assert_eq!(s.rotation, 270);
        assert!(s.crop.is_none());
        s.rotate(2);
        assert_eq!(s.rotation, 90);
    }

    #[test]
    fn flips_follow_what_is_shown() {
        let mut s = EditState::default();
        s.flip_shown_horizontally();
        assert!(s.flip_h && !s.flip_v);
        let mut t = EditState::default();
        t.rotate(1);
        t.flip_shown_horizontally();
        assert!(t.flip_v && !t.flip_h);
    }

    #[test]
    fn output_size_follows_rotation_crop_and_resize() {
        let mut s = EditState::default();
        assert_eq!(s.output_size(400, 200), (400, 200));
        s.rotate(1);
        assert_eq!(s.output_size(400, 200), (200, 400));
        s.set_crop(Some(Rect::new(0.0, 0.0, 0.5, 0.25)));
        assert_eq!(s.output_size(400, 200), (100, 100));
        s.resize = Some((50, 50));
        assert_eq!(s.output_size(400, 200), (50, 50));
    }

    #[test]
    fn a_full_crop_is_no_crop() {
        let mut s = EditState::default();
        s.set_crop(Some(Rect::FULL));
        assert!(s.crop.is_none());
    }

    #[test]
    fn history_undoes_redoes_and_drops_the_future() {
        let mut h = History::default();
        let mut a = EditState::default();
        a.brightness = 10;
        h.push(a.clone());
        let mut b = a.clone();
        b.contrast = 20;
        h.push(b);
        assert_eq!(h.undo().contrast, 0);
        assert!(h.can_redo());
        assert_eq!(h.redo().contrast, 20);
        h.undo();
        let mut c = a.clone();
        c.saturation = 5;
        h.push(c);
        assert!(!h.can_redo());
        h.push(h.current().clone());
        assert_eq!(h.states.len(), 3);
    }

    #[test]
    fn reset_is_one_step() {
        let mut h = History::default();
        let mut a = EditState::default();
        a.brightness = 10;
        h.push(a);
        h.reset();
        assert!(!h.current().is_edited());
        assert_eq!(h.undo().brightness, 10);
    }

    #[test]
    fn parses_hex_colours() {
        assert_eq!(Colour::from_hex("#ff0000"), Some(Colour::rgb(1.0, 0.0, 0.0)));
        assert_eq!(Colour::from_hex("ff0000"), None);
        assert_eq!(Colour::from_hex("#fff"), None);
    }
}
