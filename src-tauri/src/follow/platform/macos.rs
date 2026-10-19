//! macOS: `NSEvent.mouseLocation` in points from any thread, screen frames cached from the
//! main thread (NSScreen is main thread only in objc2), moves with `setFrameTopLeftPoint:`.
//! Everything is in points, so mixed display scales place the bubble exactly.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use objc2::MainThreadMarker;
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSScreen, NSWindow};
use objc2_foundation::{NSPoint, NSRect};
use tauri::{AppHandle, WebviewWindow};

use super::Sample;
use crate::error::{Error, Result};
use crate::follow::peek::Modifiers;
use crate::follow::placement::{Point, Rect};
use crate::lock::lock;

/// How often the screen list is re-read. Monitors come and go rarely.
const SCREEN_REFRESH: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug)]
struct Screen {
    frame: Rect,
    work_area: Rect,
    scale: f64,
}

#[derive(Clone, Debug, Default)]
struct Screens {
    /// Height of the primary screen, the reference for flipping AppKit's bottom-up y axis.
    primary_height: f64,
    list: Vec<Screen>,
}

pub struct Platform {
    app: AppHandle,
    /// The prompter's NSWindow pointer, carried as an integer so the thread can post it.
    ns_window: usize,
    screens: Arc<Mutex<Screens>>,
    refreshed: Instant,
}

impl Platform {
    /// Call on the main thread.
    pub fn new(app: &AppHandle, window: &WebviewWindow) -> Result<Self> {
        let mtm = MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        let ns_window = window.ns_window()? as usize;
        Ok(Self {
            app: app.clone(),
            ns_window,
            screens: Arc::new(Mutex::new(read_screens(mtm))),
            refreshed: Instant::now(),
        })
    }

    pub fn sample(&mut self) -> Option<Sample> {
        self.refresh_screens_if_stale();
        let location = NSEvent::mouseLocation();
        // Current device state, like mouseLocation: no event stream, no permission.
        let flags = NSEvent::modifierFlags_class();
        let buttons = NSEvent::pressedMouseButtons();
        let screens = lock(&self.screens);
        let pointer = Point {
            x: location.x,
            y: screens.primary_height - location.y,
        };
        let screen = screens
            .list
            .iter()
            .find(|screen| screen.frame.contains(pointer))
            .or_else(|| screens.list.first())?;
        Some(Sample {
            pointer,
            work_area: screen.work_area,
            unit_scale: 1.0,
            pixels_per_unit: screen.scale,
            modifiers: Modifiers {
                control: flags.contains(NSEventModifierFlags::Control),
                option: flags.contains(NSEventModifierFlags::Option),
                shift: flags.contains(NSEventModifierFlags::Shift),
                command: flags.contains(NSEventModifierFlags::Command),
            },
            // Bit 0 is the primary button, whichever physical button that is.
            primary_down: buttons & 1 != 0,
        })
    }

    pub fn move_to(&self, x: f64, y: f64) {
        let ns_window = self.ns_window;
        let primary_height = lock(&self.screens).primary_height;
        let _ = self.app.run_on_main_thread(move || {
            // SAFETY: the integer is the pointer Tauri returned from `ns_window()` for a window
            // that is never destroyed while the app runs, and this closure runs on the main
            // thread, the only place AppKit allows NSWindow calls.
            let window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
            window.setFrameTopLeftPoint(NSPoint::new(x, primary_height - y));
        });
    }

    fn refresh_screens_if_stale(&mut self) {
        if self.refreshed.elapsed() < SCREEN_REFRESH {
            return;
        }
        self.refreshed = Instant::now();
        let screens = Arc::clone(&self.screens);
        let _ = self.app.run_on_main_thread(move || {
            if let Some(mtm) = MainThreadMarker::new() {
                *lock(&screens) = read_screens(mtm);
            }
        });
    }
}

fn read_screens(mtm: MainThreadMarker) -> Screens {
    let all = NSScreen::screens(mtm);
    let primary_height = all
        .iter()
        .next()
        .map(|screen| screen.frame().size.height)
        .unwrap_or_default();
    let list = all
        .iter()
        .map(|screen| Screen {
            frame: flip(screen.frame(), primary_height),
            work_area: flip(screen.visibleFrame(), primary_height),
            scale: screen.backingScaleFactor(),
        })
        .collect();
    Screens { primary_height, list }
}

/// AppKit rects have their origin at the bottom left. The loop works top-down.
fn flip(rect: NSRect, primary_height: f64) -> Rect {
    Rect {
        x: rect.origin.x,
        y: primary_height - (rect.origin.y + rect.size.height),
        w: rect.size.width,
        h: rect.size.height,
    }
}
