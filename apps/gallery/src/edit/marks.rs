//! Drawing markup with cairo, for the live preview and for the saved picture alike.
//! Everything is placed by normalised coordinates, so both come out the same.

use super::{Colour, Mark, Rect};
use gtk::cairo::{self, Context, Format, ImageSurface};
use gtk::pango;
use image::RgbaImage;
use std::f64::consts::PI;

/// A highlighter stroke is this many times as wide as a pen's, and see-through.
const HIGHLIGHTER_WIDTH: f64 = 4.0;
const HIGHLIGHTER_ALPHA: f64 = 0.4;

/// Draw `marks` onto a `w`×`h` canvas. Blur marks are not drawn here: they change
/// pixels, so the pipeline applies them first.
pub fn draw_marks(cr: &Context, w: f64, h: f64, marks: &[Mark]) {
    let unit = w.min(h);
    for mark in marks {
        let _ = cr.save();
        draw_one(cr, w, h, unit, mark);
        let _ = cr.restore();
    }
}

fn set_colour(cr: &Context, c: Colour, alpha: f64) {
    cr.set_source_rgba(c.r, c.g, c.b, alpha);
}

fn draw_one(cr: &Context, w: f64, h: f64, unit: f64, mark: &Mark) {
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    match mark {
        Mark::Stroke { points, colour, size, highlighter } => {
            let Some(first) = points.first() else { return };
            if *highlighter {
                set_colour(cr, *colour, HIGHLIGHTER_ALPHA);
                cr.set_line_width(size * unit * HIGHLIGHTER_WIDTH);
                cr.set_line_cap(cairo::LineCap::Butt);
            } else {
                set_colour(cr, *colour, 1.0);
                cr.set_line_width(size * unit);
            }
            cr.move_to(first.0 * w, first.1 * h);
            if points.len() == 1 {
                // A click makes a dot: a zero-length line with round caps.
                cr.line_to(first.0 * w, first.1 * h);
            }
            for p in &points[1..] {
                cr.line_to(p.0 * w, p.1 * h);
            }
            let _ = cr.stroke();
        }
        Mark::Arrow { from, to, colour, size } => {
            let width = size * unit;
            let (fx, fy, tx, ty) = (from.0 * w, from.1 * h, to.0 * w, to.1 * h);
            let angle = (ty - fy).atan2(tx - fx);
            let head = width * 4.0 + unit * 0.01;
            // The shaft stops at the head's base so it doesn't poke through the tip.
            let (bx, by) = (tx - head * 0.8 * angle.cos(), ty - head * 0.8 * angle.sin());
            set_colour(cr, *colour, 1.0);
            cr.set_line_width(width);
            cr.move_to(fx, fy);
            cr.line_to(bx, by);
            let _ = cr.stroke();
            let wing = 25f64.to_radians();
            cr.move_to(tx, ty);
            cr.line_to(tx - head * (angle - wing).cos(), ty - head * (angle - wing).sin());
            cr.line_to(tx - head * (angle + wing).cos(), ty - head * (angle + wing).sin());
            cr.close_path();
            let _ = cr.fill();
        }
        Mark::Rect { rect, colour, size } => {
            let r = rect.scaled(w, h);
            set_colour(cr, *colour, 1.0);
            cr.set_line_width(size * unit);
            cr.rectangle(r.x, r.y, r.w, r.h);
            let _ = cr.stroke();
        }
        Mark::Ellipse { rect, colour, size } => {
            let r = rect.scaled(w, h);
            if r.w < 0.5 || r.h < 0.5 {
                return;
            }
            set_colour(cr, *colour, 1.0);
            // Scale only while building the path, so the line keeps an even width.
            cr.translate(r.x + r.w / 2.0, r.y + r.h / 2.0);
            cr.scale(r.w / 2.0, r.h / 2.0);
            cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * PI);
            cr.identity_matrix();
            cr.set_line_width(size * unit);
            let _ = cr.stroke();
        }
        Mark::Text { at, text, colour, size } => {
            if text.is_empty() {
                return;
            }
            let px = (size * unit).max(4.0);
            let layout = pangocairo::functions::create_layout(cr);
            let mut font = pango::FontDescription::new();
            font.set_family("Sans");
            font.set_weight(pango::Weight::Bold);
            font.set_absolute_size(px * pango::SCALE as f64);
            layout.set_font_description(Some(&font));
            layout.set_text(text);
            cr.move_to(at.0 * w, at.1 * h);
            pangocairo::functions::layout_path(cr, &layout);
            // A thin outline in the opposite shade keeps the text readable on any picture.
            set_colour(cr, colour.contrast(), 0.6);
            cr.set_line_width(px * 0.14);
            let _ = cr.stroke_preserve();
            set_colour(cr, *colour, 1.0);
            let _ = cr.fill();
        }
        Mark::Pixelate { .. } => {}
    }
}

/// Copy `img` (straight RGBA) into a cairo surface (premultiplied), for drawing on or painting from.
pub fn surface_from_image(img: &RgbaImage) -> Option<ImageSurface> {
    let (w, h) = img.dimensions();
    let mut surface = ImageSurface::create(Format::ARgb32, w as i32, h as i32).ok()?;
    let stride = surface.stride() as usize;
    {
        let mut data = surface.data().ok()?;
        for (y, row) in img.rows().enumerate() {
            for (x, p) in row.enumerate() {
                data[y * stride + x * 4..][..4].copy_from_slice(&premultiply(p.0).to_ne_bytes());
            }
        }
    }
    Some(surface)
}

/// Copy a surface made by `surface_from_image` back into `img`.
fn copy_surface_into(surface: &mut ImageSurface, img: &mut RgbaImage) {
    surface.flush();
    let stride = surface.stride() as usize;
    if let Ok(data) = surface.data() {
        for (y, row) in img.rows_mut().enumerate() {
            for (x, p) in row.enumerate() {
                let bytes: [u8; 4] = data[y * stride + x * 4..][..4].try_into().unwrap();
                p.0 = unpremultiply(u32::from_ne_bytes(bytes));
            }
        }
    }
}

/// Draw `marks` onto `img` through a cairo surface.
pub fn draw_on_image(img: &mut RgbaImage, marks: &[Mark]) {
    if marks.iter().all(|m| matches!(m, Mark::Pixelate { .. })) {
        return;
    }
    let (w, h) = img.dimensions();
    let Some(mut surface) = surface_from_image(img) else { return };
    {
        let Ok(cr) = Context::new(&surface) else { return };
        draw_marks(&cr, w as f64, h as f64, marks);
    }
    copy_surface_into(&mut surface, img);
}

/// Straight RGBA to cairo's premultiplied `0xAARRGGBB`.
fn premultiply([r, g, b, a]: [u8; 4]) -> u32 {
    let m = |c: u8| (c as u32 * a as u32 + 127) / 255;
    ((a as u32) << 24) | (m(r) << 16) | (m(g) << 8) | m(b)
}

fn unpremultiply(px: u32) -> [u8; 4] {
    let a = (px >> 24) & 0xff;
    let (r, g, b) = ((px >> 16) & 0xff, (px >> 8) & 0xff, px & 0xff);
    if a == 0 {
        return [0, 0, 0, 0];
    }
    let un = |c: u32| ((c * 255 + a / 2) / a).min(255) as u8;
    [un(r), un(g), un(b), a as u8]
}

/// Replace `rect` (normalised) of `img` with blocks of its average colour, `size` of the shorter side wide.
pub fn pixelate(img: &mut RgbaImage, rect: Rect, size: f64) {
    let (w, h) = img.dimensions();
    let x0 = (rect.x * w as f64).round().clamp(0.0, w as f64) as u32;
    let y0 = (rect.y * h as f64).round().clamp(0.0, h as f64) as u32;
    let x1 = (rect.right() * w as f64).round().clamp(0.0, w as f64) as u32;
    let y1 = (rect.bottom() * h as f64).round().clamp(0.0, h as f64) as u32;
    let block = ((size * w.min(h) as f64).round() as u32).max(2);
    let mut by = y0;
    while by < y1 {
        let bh = block.min(y1 - by);
        let mut bx = x0;
        while bx < x1 {
            let bw = block.min(x1 - bx);
            let mut sum = [0u64; 4];
            for y in by..by + bh {
                for x in bx..bx + bw {
                    for (s, c) in sum.iter_mut().zip(img.get_pixel(x, y).0) {
                        *s += c as u64;
                    }
                }
            }
            let n = (bw * bh) as u64;
            let avg = [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, (sum[3] / n) as u8];
            for y in by..by + bh {
                for x in bx..bx + bw {
                    img.get_pixel_mut(x, y).0 = avg;
                }
            }
            bx += bw;
        }
        by += bh;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    const RED: Colour = Colour::rgb(1.0, 0.0, 0.0);

    fn blank(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba([10, 20, 30, 255]))
    }

    #[test]
    fn premultiplying_round_trips() {
        for px in [[200, 100, 50, 255], [200, 100, 50, 128], [0, 0, 0, 0], [255, 255, 255, 255]] {
            let back = unpremultiply(premultiply(px));
            for i in 0..4 {
                assert!((back[i] as i32 - px[i] as i32).abs() <= 2, "{px:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn no_marks_leave_the_picture_alone() {
        let mut img = blank(30, 20);
        let before = img.clone();
        draw_on_image(&mut img, &[]);
        assert_eq!(img, before);
    }

    #[test]
    fn a_rectangle_is_drawn_where_asked_and_nowhere_else() {
        let mut img = blank(100, 100);
        let marks = [Mark::Rect { rect: Rect::new(0.2, 0.2, 0.6, 0.6), colour: RED, size: 0.04 }];
        draw_on_image(&mut img, &marks);
        assert_eq!(img.get_pixel(20, 50).0, [255, 0, 0, 255], "on the left edge");
        assert_eq!(img.get_pixel(50, 50).0, [10, 20, 30, 255], "inside is untouched");
        assert_eq!(img.get_pixel(2, 2).0, [10, 20, 30, 255], "outside is untouched");
    }

    #[test]
    fn the_highlighter_lets_the_picture_show_through() {
        let mut img = blank(100, 100);
        let marks = [Mark::Stroke { points: vec![(0.1, 0.5), (0.9, 0.5)], colour: RED, size: 0.02, highlighter: true }];
        draw_on_image(&mut img, &marks);
        let p = img.get_pixel(50, 50).0;
        assert!(p[0] > 60 && p[0] < 200, "partly red: {p:?}");
        assert!(p[2] > 5, "some of the picture remains: {p:?}");
    }

    #[test]
    fn arrows_ellipses_and_text_all_draw() {
        let mut img = blank(200, 200);
        let marks = [
            Mark::Arrow { from: (0.1, 0.1), to: (0.9, 0.1), colour: RED, size: 0.02 },
            Mark::Ellipse { rect: Rect::new(0.2, 0.3, 0.6, 0.3), colour: RED, size: 0.02 },
            Mark::Text { at: (0.1, 0.7), text: "Hi".into(), colour: RED, size: 0.15 },
        ];
        draw_on_image(&mut img, &marks);
        let changed =
            |x0: u32, y0: u32, x1: u32, y1: u32| (y0..y1).any(|y| (x0..x1).any(|x| img.get_pixel(x, y).0 != [10, 20, 30, 255]));
        assert!(changed(20, 15, 180, 30), "arrow");
        assert!(changed(40, 55, 160, 125), "ellipse");
        assert!(changed(15, 135, 100, 190), "text");
    }

    #[test]
    fn pixelate_makes_flat_blocks_inside_the_rectangle_only() {
        let mut img = RgbaImage::from_fn(40, 40, |x, y| Rgba([(x * 6) as u8, (y * 6) as u8, 0, 255]));
        let before = img.clone();
        pixelate(&mut img, Rect::new(0.0, 0.0, 0.5, 0.5), 0.25);
        assert_eq!(img.get_pixel(0, 0), img.get_pixel(9, 9), "one 10 px block");
        assert_ne!(img.get_pixel(0, 0), img.get_pixel(10, 0), "next block differs");
        assert_eq!(img.get_pixel(30, 30), before.get_pixel(30, 30), "outside untouched");
    }
}
