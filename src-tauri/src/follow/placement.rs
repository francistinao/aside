//! Where the bubble goes: corner offsets, edge flip with hysteresis, clamping.
//! Coordinates are top-down with the origin at the top left, in whatever unit the
//! platform uses (points on macOS, physical pixels on Windows).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

impl Corner {
    fn is_right(self) -> bool {
        matches!(self, Corner::TopRight | Corner::BottomRight)
    }

    fn is_bottom(self) -> bool {
        matches!(self, Corner::BottomLeft | Corner::BottomRight)
    }

    fn with_side(right: bool, bottom: bool) -> Corner {
        match (right, bottom) {
            (true, true) => Corner::BottomRight,
            (false, true) => Corner::BottomLeft,
            (true, false) => Corner::TopRight,
            (false, false) => Corner::TopLeft,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }
}

/// Which axes are currently flipped away from the chosen corner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlipState {
    horizontal: bool,
    vertical: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceParams {
    pub distance: f64,
    pub bubble: Size,
    pub edge_margin: f64,
    pub flip_hysteresis: f64,
    /// Off: never flip, only clamp.
    pub flip: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// Top left corner of the window.
    pub target: Point,
    /// The corner in effect after flips. Its sharp corner points at the pointer.
    pub corner: Corner,
}

/// The arrow's body hangs down and to the right of its tip. Clear it below the pointer.
const ARROW_CLEARANCE: f64 = 4.0;

fn target_x(pointer: Point, right: bool, params: &PlaceParams) -> f64 {
    if right {
        pointer.x + params.distance
    } else {
        pointer.x - params.distance - params.bubble.w
    }
}

fn target_y(pointer: Point, bottom: bool, params: &PlaceParams) -> f64 {
    if bottom {
        pointer.y + params.distance + ARROW_CLEARANCE
    } else {
        pointer.y - params.distance - params.bubble.h
    }
}

/// Decides one axis: flip when the chosen side overflows, flip back only once the
/// chosen side fits with `hysteresis` to spare.
fn resolve_axis(flipped: &mut bool, start: f64, extent: f64, low: f64, high: f64, hysteresis: f64) {
    let overflows = start < low || start + extent > high;
    let fits_with_room = start >= low + hysteresis && start + extent <= high - hysteresis;
    if !*flipped && overflows {
        *flipped = true;
    } else if *flipped && fits_with_room {
        *flipped = false;
    }
}

pub fn place(
    pointer: Point,
    corner: Corner,
    params: &PlaceParams,
    work_area: Rect,
    flip: &mut FlipState,
) -> Placement {
    let hysteresis = params.flip_hysteresis;

    if params.flip {
        resolve_axis(
            &mut flip.horizontal,
            target_x(pointer, corner.is_right(), params),
            params.bubble.w,
            work_area.x,
            work_area.right(),
            hysteresis,
        );
        resolve_axis(
            &mut flip.vertical,
            target_y(pointer, corner.is_bottom(), params),
            params.bubble.h,
            work_area.y,
            work_area.bottom(),
            hysteresis,
        );
    } else {
        *flip = FlipState::default();
    }

    let right = corner.is_right() != flip.horizontal;
    let bottom = corner.is_bottom() != flip.vertical;
    let effective = Corner::with_side(right, bottom);

    let m = params.edge_margin;
    let max_x = work_area.right() - m - params.bubble.w;
    let max_y = work_area.bottom() - m - params.bubble.h;
    let target = Point {
        x: target_x(pointer, right, params).min(max_x).max(work_area.x + m),
        y: target_y(pointer, bottom, params).min(max_y).max(work_area.y + m),
    };

    Placement { target, corner: effective }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect { x: 0.0, y: 0.0, w: 1440.0, h: 900.0 };

    fn params() -> PlaceParams {
        PlaceParams {
            distance: 20.0,
            bubble: Size { w: 380.0, h: 140.0 },
            edge_margin: 8.0,
            flip_hysteresis: 24.0,
            flip: true,
        }
    }

    fn at(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    fn place_once(pointer: Point, corner: Corner) -> Placement {
        place(pointer, corner, &params(), AREA, &mut FlipState::default())
    }

    #[test]
    fn places_every_corner_around_a_pointer_in_the_middle() {
        let p = at(700.0, 450.0);
        let cases = [
            (Corner::BottomRight, at(720.0, 474.0)),
            (Corner::BottomLeft, at(300.0, 474.0)),
            (Corner::TopRight, at(720.0, 290.0)),
            (Corner::TopLeft, at(300.0, 290.0)),
        ];
        for (corner, expected) in cases {
            let placed = place_once(p, corner);
            assert_eq!(placed.target, expected, "{corner:?}");
            assert_eq!(placed.corner, corner, "no flip expected for {corner:?}");
        }
    }

    #[test]
    fn with_flip_off_the_corner_stays_and_the_bubble_is_clamped() {
        let no_flip = PlaceParams { flip: false, ..params() };
        let mut flip = FlipState { horizontal: true, vertical: true };
        let placed = place(at(1300.0, 450.0), Corner::BottomRight, &no_flip, AREA, &mut flip);
        assert_eq!(placed.corner, Corner::BottomRight);
        assert_eq!(placed.target, at(AREA.w - 8.0 - 380.0, 474.0));
        assert_eq!(flip, FlipState::default(), "a stale flip is cleared");
    }

    #[test]
    fn flips_at_each_edge() {
        let right = place_once(at(1300.0, 450.0), Corner::BottomRight);
        assert_eq!(right.corner, Corner::BottomLeft);
        assert_eq!(right.target, at(900.0, 474.0));

        let left = place_once(at(100.0, 450.0), Corner::BottomLeft);
        assert_eq!(left.corner, Corner::BottomRight);
        assert_eq!(left.target, at(120.0, 474.0));

        let bottom = place_once(at(700.0, 850.0), Corner::BottomRight);
        assert_eq!(bottom.corner, Corner::TopRight);
        assert_eq!(bottom.target, at(720.0, 690.0));

        let top = place_once(at(700.0, 50.0), Corner::TopRight);
        assert_eq!(top.corner, Corner::BottomRight);
        assert_eq!(top.target, at(720.0, 74.0));

        let both = place_once(at(1300.0, 850.0), Corner::BottomRight);
        assert_eq!(both.corner, Corner::TopLeft);
        assert_eq!(both.target, at(900.0, 690.0));
    }

    #[test]
    fn flips_back_only_with_room_to_spare() {
        let mut flip = FlipState::default();
        let p = params();
        // 1070 + 380 overflows 1440: flip.
        let flipped = place(at(1050.0, 450.0), Corner::BottomRight, &p, AREA, &mut flip);
        assert_eq!(flipped.corner, Corner::BottomLeft);
        // 1059 + 380 = 1439 fits by 1 pt. Not enough room: stay flipped.
        let close = place(at(1039.0, 450.0), Corner::BottomRight, &p, AREA, &mut flip);
        assert_eq!(close.corner, Corner::BottomLeft);
        // 1035 + 380 = 1415 fits by 25 pt: flip back.
        let back = place(at(1015.0, 450.0), Corner::BottomRight, &p, AREA, &mut flip);
        assert_eq!(back.corner, Corner::BottomRight);
        assert_eq!(back.target, at(1035.0, 474.0));
    }

    #[test]
    fn jitter_at_the_edge_does_not_flicker() {
        let mut flip = FlipState::default();
        let p = params();
        let mut corners = Vec::new();
        for frame in 0..60 {
            let x = if frame % 2 == 0 { 1045.0 } else { 1035.0 };
            corners.push(place(at(x, 450.0), Corner::BottomRight, &p, AREA, &mut flip).corner);
        }
        assert!(corners.iter().all(|c| *c == Corner::BottomLeft), "corner flickered: {corners:?}");
    }

    #[test]
    fn clamps_inside_the_work_area_with_a_margin() {
        let p = params();
        let tiny = Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 };
        let placed = place(at(200.0, 150.0), Corner::BottomRight, &p, tiny, &mut FlipState::default());
        assert_eq!(placed.target, at(8.0, 8.0));
        assert!(placed.target.x >= tiny.x + 8.0 && placed.target.x + 380.0 <= tiny.right() - 8.0);
        assert!(placed.target.y >= tiny.y + 8.0 && placed.target.y + 140.0 <= tiny.bottom() - 8.0);

        // Pointer in the Dock below the work area: flipped up, still overflowing, clamped.
        let with_dock = Rect { x: 0.0, y: 0.0, w: 1440.0, h: 855.0 };
        let dock = place(at(700.0, 880.0), Corner::BottomRight, &p, with_dock, &mut FlipState::default());
        assert_eq!(dock.corner, Corner::TopRight);
        assert_eq!(dock.target, at(720.0, 707.0));
    }

    #[test]
    fn rect_contains_its_inside_and_not_its_far_edges() {
        assert!(AREA.contains(at(0.0, 0.0)));
        assert!(AREA.contains(at(1439.9, 899.9)));
        assert!(!AREA.contains(at(1440.0, 450.0)));
        assert!(!AREA.contains(at(-1.0, 450.0)));
    }
}
