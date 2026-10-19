//! The follow loop: cursor polling, spring, placement. See docs/CURSOR-UX.md.

pub mod placement;
pub mod spring;

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
