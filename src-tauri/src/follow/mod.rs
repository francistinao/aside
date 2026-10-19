//! The follow loop: cursor polling, spring, placement. See docs/CURSOR-UX.md.
//! One plain thread owns the whole loop. It never sends per-frame data over IPC.

pub mod placement;
pub mod platform;
pub mod spring;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, WebviewWindow};

use placement::{Corner, FlipState, PlaceParams, Point, Size, place};
use platform::Platform;
use spring::{cap_speed, clamp_dt, is_settled, spring_step};

use crate::error::Result;

/// Sent to the prompter only when the effective corner changes after an edge flip.
pub const STYLE_EVENT: &str = "prompter://style";

/// The follow feel presets from docs/CURSOR-UX.md.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feel {
    Snappy,
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

/// The distance presets from docs/CURSOR-UX.md.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distance {
    Close,
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
struct StylePayload {
    corner: Corner,
}

#[derive(Default)]
struct Shared {
    active: AtomicBool,
    hz: AtomicU32,
    settled: AtomicBool,
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
}

/// Starts the follow thread for `window`. Call on the main thread. The thread starts parked.
pub fn spawn(
    app: &AppHandle,
    window: &WebviewWindow,
    config: FollowConfig,
    corner: Corner,
    bubble: Size,
) -> Result<FollowHandle> {
    let platform = Platform::new(app, window)?;
    let shared = Arc::new(Shared::default());
    let loop_shared = Arc::clone(&shared);
    let loop_app = app.clone();
    let label = window.label().to_string();
    let join = thread::Builder::new()
        .name("follow".into())
        .spawn(move || run(loop_app, label, platform, loop_shared, config, corner, bubble))?;
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

fn run(
    app: AppHandle,
    label: String,
    mut platform: Platform,
    shared: Arc<Shared>,
    config: FollowConfig,
    corner: Corner,
    bubble: Size,
) {
    let active_period = Duration::from_secs_f64(1.0 / f64::from(config.active_hz));
    let idle_period = Duration::from_secs_f64(1.0 / f64::from(config.idle_hz));
    let mut state = Bubble::default();
    let mut flip = FlipState::default();
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
            thread::park();
            last_frame = Instant::now();
            continue;
        }

        let now = Instant::now();
        let dt = clamp_dt(now.duration_since(last_frame).as_secs_f64());
        last_frame = now;

        let Some(sample) = platform.sample() else {
            thread::sleep(idle_period);
            continue;
        };
        let scale = sample.unit_scale;
        let params = PlaceParams {
            distance: config.distance * scale,
            bubble: Size {
                w: bubble.w * scale,
                h: bubble.h * scale,
            },
            edge_margin: config.edge_margin * scale,
            flip_hysteresis: config.flip_hysteresis * scale,
        };
        let placement = place(sample.pointer, corner, &params, sample.work_area, &mut flip);
        if effective_corner != Some(placement.corner) {
            effective_corner = Some(placement.corner);
            let payload = StylePayload {
                corner: placement.corner,
            };
            let _ = app.emit_to(label.as_str(), STYLE_EVENT, payload);
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
