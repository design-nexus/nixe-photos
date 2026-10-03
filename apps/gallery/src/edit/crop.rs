//! The geometry of the crop box: which handle is under the pointer, and what a drag
//! does to the box, optionally keeping an aspect ratio. Works in whatever unit the
//! caller uses (pixels of the preview), with `bounds` as the picture's rectangle.

use super::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    /// The inside of the box: drag to move all of it.
    Move,
}

/// The handle at `(px, py)`: a corner or edge within `tol` of the box's outline, or
/// the inside. `None` when the point is outside the box.
pub fn hit_test(r: Rect, px: f64, py: f64, tol: f64) -> Option<Handle> {
    let in_x = px >= r.x - tol && px <= r.right() + tol;
    let in_y = py >= r.y - tol && py <= r.bottom() + tol;
    if !in_x || !in_y {
        return None;
    }
    let left = (px - r.x).abs() <= tol;
    let right = (px - r.right()).abs() <= tol;
    let top = (py - r.y).abs() <= tol;
    let bottom = (py - r.bottom()).abs() <= tol;
    Some(match (left, right, top, bottom) {
        (true, _, true, _) => Handle::TopLeft,
        (_, true, true, _) => Handle::TopRight,
        (true, _, _, true) => Handle::BottomLeft,
        (_, true, _, true) => Handle::BottomRight,
        (_, _, true, _) => Handle::Top,
        (_, _, _, true) => Handle::Bottom,
        (true, ..) => Handle::Left,
        (_, true, ..) => Handle::Right,
        _ => Handle::Move,
    })
}

/// The box after dragging `handle` by `(dx, dy)` from `r`. It stays inside `bounds`
/// and at least `min` on its smaller side. `ratio` (width / height) locks the shape.
pub fn drag(r: Rect, handle: Handle, dx: f64, dy: f64, bounds: Rect, ratio: Option<f64>, min: f64) -> Rect {
    if handle == Handle::Move {
        return Rect::new(
            (r.x + dx).clamp(bounds.x, (bounds.right() - r.w).max(bounds.x)),
            (r.y + dy).clamp(bounds.y, (bounds.bottom() - r.h).max(bounds.y)),
            r.w,
            r.h,
        );
    }
    match ratio {
        None => drag_free(r, handle, dx, dy, bounds, min),
        Some(ratio) => drag_locked(r, handle, dx, dy, bounds, ratio, min),
    }
}

fn drag_free(r: Rect, handle: Handle, dx: f64, dy: f64, bounds: Rect, min: f64) -> Rect {
    use Handle::*;
    let (mut l, mut t, mut rt, mut b) = (r.x, r.y, r.right(), r.bottom());
    if matches!(handle, TopLeft | Left | BottomLeft) {
        l = (r.x + dx).clamp(bounds.x, r.right() - min);
    }
    if matches!(handle, TopRight | Right | BottomRight) {
        rt = (r.right() + dx).clamp(r.x + min, bounds.right());
    }
    if matches!(handle, TopLeft | Top | TopRight) {
        t = (r.y + dy).clamp(bounds.y, r.bottom() - min);
    }
    if matches!(handle, BottomLeft | Bottom | BottomRight) {
        b = (r.bottom() + dy).clamp(r.y + min, bounds.bottom());
    }
    Rect::new(l, t, rt - l, b - t)
}

fn drag_locked(r: Rect, handle: Handle, dx: f64, dy: f64, bounds: Rect, ratio: f64, min: f64) -> Rect {
    use Handle::*;
    // The smallest width such that both sides are at least `min`.
    let min_w = min.max(min * ratio);
    match handle {
        TopLeft | TopRight | BottomLeft | BottomRight => {
            let left = matches!(handle, TopLeft | BottomLeft);
            let up = matches!(handle, TopLeft | TopRight);
            // The corner opposite the one being dragged stays put.
            let (ax, ay) = (if left { r.right() } else { r.x }, if up { r.bottom() } else { r.y });
            let (cx, cy) = (if left { r.x } else { r.right() } + dx, if up { r.y } else { r.bottom() } + dy);
            let want_w = (cx - ax).abs().max((cy - ay).abs() * ratio);
            let room_w = if left { ax - bounds.x } else { bounds.right() - ax };
            let room_h = if up { ay - bounds.y } else { bounds.bottom() - ay };
            let w = want_w.min(room_w).min(room_h * ratio).max(min_w);
            let h = w / ratio;
            Rect::new(if left { ax - w } else { ax }, if up { ay - h } else { ay }, w, h)
        }
        Top | Bottom => {
            let h_edge = if handle == Top { r.bottom() - (r.y + dy) } else { r.bottom() + dy - r.y };
            let room_h = if handle == Top { r.bottom() - bounds.y } else { bounds.bottom() - r.y };
            let h = h_edge.min(room_h).min(bounds.w / ratio).max(min_w / ratio);
            let w = h * ratio;
            let cx = r.x + r.w / 2.0;
            let x = (cx - w / 2.0).clamp(bounds.x, (bounds.right() - w).max(bounds.x));
            Rect::new(x, if handle == Top { r.bottom() - h } else { r.y }, w, h)
        }
        Left | Right => {
            let w_edge = if handle == Left { r.right() - (r.x + dx) } else { r.right() + dx - r.x };
            let room_w = if handle == Left { r.right() - bounds.x } else { bounds.right() - r.x };
            let w = w_edge.min(room_w).min(bounds.h * ratio).max(min_w);
            let h = w / ratio;
            let cy = r.y + r.h / 2.0;
            let y = (cy - h / 2.0).clamp(bounds.y, (bounds.bottom() - h).max(bounds.y));
            Rect::new(if handle == Left { r.right() - w } else { r.x }, y, w, h)
        }
        Move => r,
    }
}

/// The largest box of `ratio` (width / height) that fits in `r`, with the same centre.
pub fn fit_ratio(r: Rect, ratio: f64) -> Rect {
    let (w, h) = if r.w / r.h > ratio { (r.h * ratio, r.h) } else { (r.w, r.w / ratio) };
    Rect::new(r.x + (r.w - w) / 2.0, r.y + (r.h - h) / 2.0, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: Rect = Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 };

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn finds_corners_edges_and_inside() {
        let r = Rect::new(100.0, 100.0, 200.0, 100.0);
        assert_eq!(hit_test(r, 101.0, 99.0, 8.0), Some(Handle::TopLeft));
        assert_eq!(hit_test(r, 299.0, 201.0, 8.0), Some(Handle::BottomRight));
        assert_eq!(hit_test(r, 200.0, 98.0, 8.0), Some(Handle::Top));
        assert_eq!(hit_test(r, 303.0, 150.0, 8.0), Some(Handle::Right));
        assert_eq!(hit_test(r, 200.0, 150.0, 8.0), Some(Handle::Move));
        assert_eq!(hit_test(r, 50.0, 50.0, 8.0), None);
    }

    #[test]
    fn moving_stays_inside_the_picture() {
        let r = Rect::new(100.0, 100.0, 200.0, 100.0);
        let m = drag(r, Handle::Move, 500.0, -500.0, B, None, 10.0);
        assert_eq!((m.x, m.y, m.w, m.h), (200.0, 0.0, 200.0, 100.0));
    }

    #[test]
    fn free_drag_moves_only_its_edges() {
        let r = Rect::new(100.0, 100.0, 200.0, 100.0);
        let m = drag(r, Handle::TopLeft, -30.0, 20.0, B, None, 10.0);
        assert_eq!((m.x, m.y, m.right(), m.bottom()), (70.0, 120.0, 300.0, 200.0));
        let m = drag(r, Handle::Right, 500.0, 50.0, B, None, 10.0);
        assert_eq!((m.x, m.y, m.right(), m.bottom()), (100.0, 100.0, 400.0, 200.0));
    }

    #[test]
    fn free_drag_keeps_a_minimum_size() {
        let r = Rect::new(100.0, 100.0, 200.0, 100.0);
        let m = drag(r, Handle::Left, 500.0, 0.0, B, None, 20.0);
        assert!(close(m.w, 20.0) && close(m.right(), 300.0));
    }

    #[test]
    fn locked_corner_keeps_the_ratio_and_stays_inside() {
        let r = Rect::new(100.0, 100.0, 160.0, 90.0);
        let ratio = 16.0 / 9.0;
        let m = drag(r, Handle::BottomRight, 1000.0, 1000.0, B, Some(ratio), 10.0);
        assert!(close(m.w / m.h, ratio));
        assert!(m.right() <= B.right() + 1e-6 && m.bottom() <= B.bottom() + 1e-6);
        assert!(close(m.x, 100.0) && close(m.y, 100.0));
        let m = drag(r, Handle::TopLeft, -20.0, -5.0, B, Some(ratio), 10.0);
        assert!(close(m.w / m.h, ratio));
        assert!(close(m.right(), 260.0) && close(m.bottom(), 190.0));
    }

    #[test]
    fn locked_edge_resizes_about_the_centre() {
        let r = Rect::new(100.0, 100.0, 100.0, 100.0);
        let m = drag(r, Handle::Bottom, 0.0, 50.0, B, Some(1.0), 10.0);
        assert!(close(m.w, 150.0) && close(m.h, 150.0));
        assert!(close(m.x + m.w / 2.0, 150.0));
        let m = drag(r, Handle::Left, -40.0, 0.0, B, Some(1.0), 10.0);
        assert!(close(m.w, 140.0) && close(m.right(), 200.0) && close(m.h, 140.0));
    }

    #[test]
    fn locked_drag_never_leaves_the_picture() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        let m = drag(r, Handle::Right, 10_000.0, 0.0, B, Some(1.0), 10.0);
        assert!(m.right() <= B.right() + 1e-6 && m.bottom() <= B.bottom() + 1e-6);
        assert!(close(m.w, m.h));
    }

    #[test]
    fn fits_a_ratio_inside_a_box() {
        let r = fit_ratio(Rect::new(0.0, 0.0, 400.0, 300.0), 1.0);
        assert_eq!((r.x, r.y, r.w, r.h), (50.0, 0.0, 300.0, 300.0));
        let r = fit_ratio(Rect::new(0.0, 0.0, 300.0, 400.0), 2.0);
        assert!(close(r.w / r.h, 2.0) && close(r.w, 300.0));
    }
}
