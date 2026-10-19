//! Windows: `GetCursorPos` in physical pixels, the monitor under the pointer and its DPI.
//! Moves go through Tauri's `set_position`, which calls `SetWindowPos` without activating.
//! The loop works in physical virtual-screen pixels, which stay continuous across monitors.

use tauri::{AppHandle, PhysicalPosition, WebviewWindow};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use super::Sample;
use crate::error::Result;
use crate::follow::placement::{Point, Rect};

const BASE_DPI: u32 = 96;

pub struct Platform {
    window: WebviewWindow,
}

impl Platform {
    pub fn new(_app: &AppHandle, window: &WebviewWindow) -> Result<Self> {
        Ok(Self {
            window: window.clone(),
        })
    }

    pub fn sample(&mut self) -> Option<Sample> {
        let mut point = POINT::default();
        // SAFETY: `point` is a valid out pointer for the duration of the call.
        unsafe { GetCursorPos(&mut point) }.ok()?;
        // SAFETY: no preconditions. A point outside every monitor maps to the nearest one.
        let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: `info` is a valid out struct with its size filled in.
        if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            return None;
        }
        let (mut dpi_x, mut dpi_y) = (BASE_DPI, BASE_DPI);
        // SAFETY: valid out pointers. If the call fails the 96 dpi defaults stand.
        let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
        Some(Sample {
            pointer: Point {
                x: f64::from(point.x),
                y: f64::from(point.y),
            },
            work_area: rect(info.rcWork),
            unit_scale: f64::from(dpi_x) / f64::from(BASE_DPI),
            pixels_per_unit: 1.0,
        })
    }

    pub fn move_to(&self, x: f64, y: f64) {
        let _ = self.window.set_position(PhysicalPosition::new(x as i32, y as i32));
    }
}

fn rect(r: RECT) -> Rect {
    Rect {
        x: f64::from(r.left),
        y: f64::from(r.top),
        w: f64::from(r.right - r.left),
        h: f64::from(r.bottom - r.top),
    }
}
