//! Hold to peek and fade while clicking. Pure: the loop
//! feeds it the time, the keys and the pointer each frame and emits only when the result
//! changes. Hidden is the Show or Hide Prompter toggle and wins over everything.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::distance;
use super::placement::Point;

/// The key the user holds to peek. Option is Alt on Windows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PeekKey {
    #[default]
    Control,
    Option,
    Shift,
}

/// What the overlay should show. Peeking and Hidden are opacity 0, Fading is a quarter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Visibility {
    #[default]
    Visible,
    Peeking,
    Fading,
    Hidden,
}

/// Modifier keys held right now. Command is the Windows key on Windows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub control: bool,
    pub option: bool,
    pub shift: bool,
    pub command: bool,
}

impl Modifiers {
    /// True when `key` is held and no other modifier is, so a shortcut never peeks.
    pub fn only(self, key: PeekKey) -> bool {
        let held = match key {
            PeekKey::Control => self.control,
            PeekKey::Option => self.option,
            PeekKey::Shift => self.shift,
        };
        let count = [self.control, self.option, self.shift, self.command]
            .iter()
            .filter(|&&down| down)
            .count();
        held && count == 1
    }
}

/// One frame's input, in the loop's units.
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub modifiers: Modifiers,
    pub primary_down: bool,
    pub pointer: Point,
    /// `RETURN_DISTANCE` scaled into loop units.
    pub return_distance: f64,
}

/// After the key is released the bubble stays away this long, so the user can click.
pub const GRACE: Duration = Duration::from_millis(1200);
/// Moving the pointer this far after release brings the bubble back at once.
pub const RETURN_DISTANCE: f64 = 120.0;

#[derive(Debug, Default)]
pub struct Peek {
    key: PeekKey,
    fade: bool,
    hidden: bool,
    holding: bool,
    /// When and where the key was released. Cleared once the grace ends or the pointer moves on.
    released: Option<(Instant, Point)>,
}

impl Peek {
    pub fn set_key(&mut self, key: PeekKey) {
        self.key = key;
    }

    pub fn set_fade(&mut self, fade: bool) {
        self.fade = fade;
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    pub fn step(&mut self, now: Instant, input: &Input) -> Visibility {
        if input.modifiers.only(self.key) {
            // No timeout while held: a long peek keeps the bubble away until release.
            self.holding = true;
            self.released = None;
        } else if self.holding {
            self.holding = false;
            self.released = Some((now, input.pointer));
        }
        if let Some((at, origin)) = self.released {
            let moved_on = distance(origin, input.pointer) > input.return_distance;
            if moved_on || now.duration_since(at) >= GRACE {
                self.released = None;
            }
        }
        if self.hidden {
            Visibility::Hidden
        } else if self.holding || self.released.is_some() {
            Visibility::Peeking
        } else if self.fade && input.primary_down {
            Visibility::Fading
        } else {
            Visibility::Visible
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTROL: Modifiers = Modifiers { control: true, option: false, shift: false, command: false };
    const NONE: Modifiers = Modifiers { control: false, option: false, shift: false, command: false };

    fn input(modifiers: Modifiers, x: f64, primary_down: bool) -> Input {
        Input {
            modifiers,
            primary_down,
            pointer: Point { x, y: 0.0 },
            return_distance: RETURN_DISTANCE,
        }
    }

    fn after(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    #[test]
    fn holding_the_key_peeks_without_timeout_and_the_grace_holds_after_release() {
        let start = Instant::now();
        let mut peek = Peek::default();
        assert_eq!(peek.step(start, &input(CONTROL, 0.0, false)), Visibility::Peeking);
        assert_eq!(peek.step(after(start, 15_000), &input(CONTROL, 0.0, false)), Visibility::Peeking);
        let released = after(start, 15_000);
        assert_eq!(peek.step(released, &input(NONE, 0.0, false)), Visibility::Peeking);
        assert_eq!(peek.step(after(released, 1_000), &input(NONE, 0.0, false)), Visibility::Peeking);
        assert_eq!(peek.step(after(released, 1_250), &input(NONE, 0.0, false)), Visibility::Visible);
    }

    #[test]
    fn moving_the_pointer_after_release_brings_the_bubble_back() {
        let start = Instant::now();
        let mut peek = Peek::default();
        peek.step(start, &input(CONTROL, 0.0, false));
        assert_eq!(peek.step(after(start, 100), &input(NONE, 0.0, false)), Visibility::Peeking);
        assert_eq!(peek.step(after(start, 200), &input(NONE, 100.0, false)), Visibility::Peeking);
        assert_eq!(peek.step(after(start, 300), &input(NONE, 130.0, false)), Visibility::Visible);
    }

    #[test]
    fn another_modifier_held_with_the_key_does_not_peek() {
        let mut peek = Peek::default();
        let both = Modifiers { control: true, option: true, ..NONE };
        assert_eq!(peek.step(Instant::now(), &input(both, 0.0, false)), Visibility::Visible);
        let mut shift = Peek::default();
        shift.set_key(PeekKey::Shift);
        let only_shift = Modifiers { shift: true, ..NONE };
        assert_eq!(shift.step(Instant::now(), &input(only_shift, 0.0, false)), Visibility::Peeking);
        let mut other_key = Peek::default();
        other_key.set_key(PeekKey::Shift);
        assert_eq!(other_key.step(Instant::now(), &input(CONTROL, 0.0, false)), Visibility::Visible);
    }

    #[test]
    fn fade_while_clicking_needs_the_setting_and_loses_to_peek() {
        let now = Instant::now();
        let mut peek = Peek::default();
        assert_eq!(peek.step(now, &input(NONE, 0.0, true)), Visibility::Visible);
        peek.set_fade(true);
        assert_eq!(peek.step(now, &input(NONE, 0.0, true)), Visibility::Fading);
        assert_eq!(peek.step(now, &input(CONTROL, 0.0, true)), Visibility::Peeking);
        // Released: the grace keeps the bubble away, then the fade applies again.
        assert_eq!(peek.step(after(now, 2_000), &input(NONE, 0.0, true)), Visibility::Peeking);
        assert_eq!(peek.step(after(now, 3_300), &input(NONE, 0.0, true)), Visibility::Fading);
        assert_eq!(peek.step(after(now, 3_300), &input(NONE, 0.0, false)), Visibility::Visible);
    }

    #[test]
    fn hidden_wins_over_everything_until_cleared() {
        let now = Instant::now();
        let mut peek = Peek::default();
        peek.set_hidden(true);
        assert_eq!(peek.step(now, &input(CONTROL, 0.0, true)), Visibility::Hidden);
        peek.set_hidden(false);
        assert_eq!(peek.step(now, &input(CONTROL, 0.0, true)), Visibility::Peeking);
    }

    #[test]
    fn a_key_already_held_on_the_first_frame_peeks_at_once() {
        let mut peek = Peek::default();
        assert_eq!(peek.step(Instant::now(), &input(CONTROL, 0.0, false)), Visibility::Peeking);
    }

    #[test]
    fn pressing_again_within_the_grace_restarts_it_on_the_next_release() {
        let start = Instant::now();
        let mut peek = Peek::default();
        peek.step(start, &input(CONTROL, 0.0, false));
        peek.step(after(start, 100), &input(NONE, 0.0, false));
        assert_eq!(peek.step(after(start, 900), &input(CONTROL, 0.0, false)), Visibility::Peeking);
        let released = after(start, 1_000);
        peek.step(released, &input(NONE, 0.0, false));
        assert_eq!(peek.step(after(released, 1_100), &input(NONE, 0.0, false)), Visibility::Peeking);
        assert_eq!(peek.step(after(released, 1_300), &input(NONE, 0.0, false)), Visibility::Visible);
    }
}
