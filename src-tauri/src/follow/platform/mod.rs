//! Native pointer reading and window moves, one implementation per OS.

use crate::follow::peek::Modifiers;
use crate::follow::placement::{Point, Rect};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::Platform;
#[cfg(windows)]
pub use windows::Platform;

/// One reading of the pointer and the monitor under it, in the loop's units:
/// points on macOS, physical pixels on Windows.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub pointer: Point,
    pub work_area: Rect,
    /// Multiplies logical config values (distance, margins, bubble size) into loop units.
    pub unit_scale: f64,
    /// Physical pixels per loop unit, for rounding so text stays crisp.
    pub pixels_per_unit: f64,
    /// Modifier keys held right now, for the peek key.
    pub modifiers: Modifiers,
    /// The primary mouse button, for fade while clicking.
    pub primary_down: bool,
}
