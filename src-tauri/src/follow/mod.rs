//! The follow loop: cursor polling, spring, placement.
//! One plain thread owns the whole loop. It never sends per-frame data over IPC.

pub mod peek;
pub mod placement;
pub mod platform;
pub mod spring;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, WebviewWindow};

use peek::{Input, Peek, PeekKey, RETURN_DISTANCE, Visibility};
use placement::{Corner, FlipState, PlaceParams, Point, Size, place};
use platform::Platform;
use spring::{cap_speed, clamp_dt, is_settled, spring_step};

use crate::error::Result;
use crate::lock::lock;

/// Sent to the prompter only when the effective corner changes after an edge flip.
pub const CORNER_EVENT: &str = "prompter://corner";
/// Sent to the prompter only when peek, fade or hide changes what it should show.
pub const VISIBILITY_EVENT: &str = "prompter://visibility";

/// The follow feel presets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Feel {
    Snappy,
    #[default]
    Smooth,
    Floaty,
}

impl Feel {
    /// Spring stiffness in 1/s.
    pub fn omega(self) -> f64 {
        match self {
            Feel::Snappy => 28.0,
            Feel::Smooth => 16.0,
            Feel::Floaty => 9.0,
        }
    }
}

/// The distance presets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Distance {
    Close,
    #[default]
    Normal,
    Far,
}

impl Distance {
    /// Gap between the pointer and the bubble, in logical points.
    pub fn points(self) -> f64 {
        match self {
            Distance::Close => 12.0,
            Distance::Normal => 20.0,
            Distance::Far => 36.0,
        }
    }
}

/// Every follow parameter in one place. Units are logical points and seconds.
/// Peek and fade parameters arrive with the peek feature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FollowConfig {
    pub omega: f64,
    pub max_speed: f64,
    pub dead_zone_target: f64,
    pub settle_distance: f64,
    pub settle_speed: f64,
    pub distance: f64,
    pub edge_margin: f64,
    pub flip_hysteresis: f64,
    /// Off: the bubble keeps its corner and is only clamped inside the work area.
    pub flip_at_edges: bool,
    pub active_hz: u32,
    pub idle_hz: u32,
}

impl FollowConfig {
    pub fn new(feel: Feel, distance: Distance) -> Self {
        Self {
            omega: feel.omega(),
            max_speed: 6000.0,
            dead_zone_target: 2.0,
            settle_distance: 0.5,
            settle_speed: 5.0,
            distance: distance.points(),
            edge_margin: 8.0,
            flip_hysteresis: 24.0,
            flip_at_edges: true,
            active_hz: 120,
            idle_hz: 20,
        }
    }
}

impl Default for FollowConfig {
    fn default() -> Self {
        Self::new(Feel::Smooth, Distance::Normal)
    }
}

/// Debug readout for the Library window.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct FollowStats {
    pub hz: u32,
    pub settled: bool,
}

#[derive(Clone, Copy, Serialize)]
struct CornerPayload {
    corner: Corner,
}

#[derive(Clone, Copy, Serialize)]
struct VisibilityPayload {
    state: Visibility,
}

struct Shared {
    active: AtomicBool,
    hz: AtomicU32,
    settled: AtomicBool,
    /// Feel, distance and flip from the settings. Read once per frame.
    config: Mutex<FollowConfig>,
    peek_key: Mutex<PeekKey>,
    fade: AtomicBool,
    /// The Show or Hide Prompter toggle.
    hidden: AtomicBool,
    /// Set when the overlay asks for its state: the next frame emits corner and visibility.
    resend: AtomicBool,
    /// The corner the user chose. The loop flips away from it at screen edges.
    corner: Mutex<Corner>,
    /// Logical window size, kept in step with the overlay's layout.
    bubble: Mutex<Size>,
}

/// Controls the follow thread from the main thread.
pub struct FollowHandle {
    shared: Arc<Shared>,
    thread: Thread,
}

impl FollowHandle {
    /// An inactive loop parks its thread and leaves the window alone.
    pub fn set_active(&self, active: bool) {
        self.shared.active.store(active, Ordering::SeqCst);
        if active {
            self.thread.unpark();
        }
    }

    pub fn stats(&self) -> FollowStats {
        FollowStats {
            hz: self.shared.hz.load(Ordering::Relaxed),
            settled: self.shared.settled.load(Ordering::Relaxed),
        }
    }

    /// Applies a new feel, distance or flip setting from the next frame on.
    pub fn set_config(&self, config: FollowConfig) {
        *lock(&self.shared.config) = config;
    }

    pub fn set_peek_key(&self, key: PeekKey) {
        *lock(&self.shared.peek_key) = key;
    }

    pub fn set_fade(&self, fade: bool) {
        self.shared.fade.store(fade, Ordering::SeqCst);
    }

    /// Hidden keeps following at opacity 0; the window itself stays shown.
    pub fn set_hidden(&self, hidden: bool) {
        self.shared.hidden.store(hidden, Ordering::SeqCst);
    }

    /// Emits the current corner and visibility again on the next frame.
    pub fn resend(&self) {
        self.shared.resend.store(true, Ordering::SeqCst);
    }

    pub fn set_corner(&self, corner: Corner) {
        *lock(&self.shared.corner) = corner;
    }

    pub fn set_bubble(&self, bubble: Size) {
        *lock(&self.shared.bubble) = bubble;
    }
}

/// Starts the follow thread for `window`. Call on the main thread. The thread starts parked.
pub fn spawn(
    app: &AppHandle,
    window: &WebviewWindow,
    config: FollowConfig,
    corner: Corner,
    bubble: Size,
    peek_key: PeekKey,
    fade: bool,
) -> Result<FollowHandle> {
    let platform = Platform::new(app, window)?;
    let shared = Arc::new(Shared {
        active: AtomicBool::new(false),
        hz: AtomicU32::new(0),
        settled: AtomicBool::new(false),
        config: Mutex::new(config),
        peek_key: Mutex::new(peek_key),
        fade: AtomicBool::new(fade),
        hidden: AtomicBool::new(false),
        resend: AtomicBool::new(false),
        corner: Mutex::new(corner),
        bubble: Mutex::new(bubble),
    });
    let loop_shared = Arc::clone(&shared);
    let loop_app = app.clone();
    let label = window.label().to_string();
    let join = thread::Builder::new()
        .name("follow".into())
        .spawn(move || run(loop_app, label, platform, loop_shared))?;
    Ok(FollowHandle {
        shared,
        thread: join.thread().clone(),
    })
}

#[derive(Clone, Copy, Default)]
struct Bubble {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

impl Bubble {
    fn at(point: Point) -> Self {
        Self {
            x: point.x,
            y: point.y,
            vx: 0.0,
            vy: 0.0,
        }
    }
}

fn distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn run(app: AppHandle, label: String, mut platform: Platform, shared: Arc<Shared>) {
    let mut state = Bubble::default();
    let mut flip = FlipState::default();
    let mut peek = Peek::default();
    let mut last_visibility = None;
    let mut chosen_corner = *lock(&shared.corner);
    let mut effective_corner = None;
    let mut last_target = Point::default();
    let mut last_move = None;
    let mut last_frame = Instant::now();
    let mut fresh = true;
    let mut frames = 0u32;
    let mut second = Instant::now();

    loop {
        if !shared.active.load(Ordering::SeqCst) {
            shared.hz.store(0, Ordering::Relaxed);
            fresh = true;
            last_move = None;
            // The next session starts visible even if the last one stopped mid peek.
            last_visibility = None;
            thread::park();
            last_frame = Instant::now();
            continue;
        }

        let now = Instant::now();
        let dt = clamp_dt(now.duration_since(last_frame).as_secs_f64());
        last_frame = now;
        if shared.resend.swap(false, Ordering::SeqCst) {
            effective_corner = None;
            last_visibility = None;
        }
        let config = *lock(&shared.config);
        let active_period = Duration::from_secs_f64(1.0 / f64::from(config.active_hz));
        let idle_period = Duration::from_secs_f64(1.0 / f64::from(config.idle_hz));

        let Some(sample) = platform.sample() else {
            thread::sleep(idle_period);
            continue;
        };
        let scale = sample.unit_scale;
        peek.set_key(*lock(&shared.peek_key));
        peek.set_fade(shared.fade.load(Ordering::SeqCst));
        peek.set_hidden(shared.hidden.load(Ordering::SeqCst));
        let visibility = peek.step(
            now,
            &Input {
                modifiers: sample.modifiers,
                primary_down: sample.primary_down,
                pointer: sample.pointer,
                return_distance: RETURN_DISTANCE * scale,
            },
        );
        if last_visibility != Some(visibility) {
            last_visibility = Some(visibility);
            let payload = VisibilityPayload { state: visibility };
            let _ = app.emit_to(label.as_str(), VISIBILITY_EVENT, payload);
        }

        let corner = *lock(&shared.corner);
        let bubble = *lock(&shared.bubble);
        if corner != chosen_corner {
            chosen_corner = corner;
            flip = FlipState::default();
        }
        let params = PlaceParams {
            distance: config.distance * scale,
            bubble: Size {
                w: bubble.w * scale,
                h: bubble.h * scale,
            },
            edge_margin: config.edge_margin * scale,
            flip_hysteresis: config.flip_hysteresis * scale,
            flip: config.flip_at_edges,
        };
        let placement = place(sample.pointer, corner, &params, sample.work_area, &mut flip);
        if effective_corner != Some(placement.corner) {
            effective_corner = Some(placement.corner);
            let payload = CornerPayload {
                corner: placement.corner,
            };
            let _ = app.emit_to(label.as_str(), CORNER_EVENT, payload);
        }

        let target = placement.target;
        if fresh {
            // Start on the target. The bubble never flies in from somewhere else.
            state = Bubble::at(target);
            last_target = target;
            fresh = false;
        }
        let target_moved = distance(target, last_target);
        last_target = target;

        let (x, vx) = spring_step(state.x, state.vx, target.x, config.omega, dt);
        let (y, vy) = spring_step(state.y, state.vy, target.y, config.omega, dt);
        let (vx, vy) = cap_speed(vx, vy, config.max_speed * scale);
        state = Bubble { x, y, vx, vy };

        let scaled = FollowConfig {
            dead_zone_target: config.dead_zone_target * scale,
            settle_distance: config.settle_distance * scale,
            settle_speed: config.settle_speed * scale,
            ..config
        };
        let remaining = distance(Point { x, y }, target);
        let speed = (vx * vx + vy * vy).sqrt();
        let settled = is_settled(target_moved, remaining, speed, &scaled);
        if settled {
            state = Bubble::at(target);
        }

        // Round to whole physical pixels and skip the move when nothing changed.
        let ppu = sample.pixels_per_unit;
        let rounded = ((state.x * ppu).round() as i64, (state.y * ppu).round() as i64);
        if last_move != Some(rounded) {
            platform.move_to(rounded.0 as f64 / ppu, rounded.1 as f64 / ppu);
            last_move = Some(rounded);
        }

        shared.settled.store(settled, Ordering::Relaxed);
        frames += 1;
        if second.elapsed() >= Duration::from_secs(1) {
            shared.hz.store(frames, Ordering::Relaxed);
            frames = 0;
            second = Instant::now();
        }

        let period = if settled { idle_period } else { active_period };
        let spent = now.elapsed();
        if spent < period {
            thread::sleep(period - spent);
        }
    }
}
