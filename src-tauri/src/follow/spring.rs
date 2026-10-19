//! Critically damped spring math. Pure functions, no windows.

use super::FollowConfig;

/// Longest frame the spring will integrate. A stall never teleports the bubble.
pub const MAX_DT: f64 = 1.0 / 30.0;

/// One axis of a critically damped spring.
/// omega: stiffness in 1/s. Higher means snappier.
pub fn spring_step(x: f64, v: f64, target: f64, omega: f64, dt: f64) -> (f64, f64) {
    let delta = x - target;
    let temp = (v + omega * delta) * dt;
    let decay = (-omega * dt).exp();
    let new_v = (v - omega * temp) * decay;
    let new_x = target + (delta + temp) * decay;
    (new_x, new_v)
}

/// Clamps a frame duration into the range the spring accepts.
pub fn clamp_dt(dt: f64) -> f64 {
    dt.clamp(0.0, MAX_DT)
}

/// Caps the length of the velocity vector so a big jump glides instead of whipping.
pub fn cap_speed(vx: f64, vy: f64, max_speed: f64) -> (f64, f64) {
    let speed = (vx * vx + vy * vy).sqrt();
    if speed <= max_speed || speed == 0.0 {
        (vx, vy)
    } else {
        let scale = max_speed / speed;
        (vx * scale, vy * scale)
    }
}

/// The dead zone: the target barely moved and the bubble has arrived and slowed down.
/// A settled bubble snaps to the target and the loop stops moving the window.
pub fn is_settled(target_moved: f64, distance_to_target: f64, speed: f64, cfg: &FollowConfig) -> bool {
    target_moved < cfg.dead_zone_target
        && distance_to_target <= cfg.settle_distance
        && speed < cfg.settle_speed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::follow::{Distance, Feel};

    /// Steps the spring at a fixed rate until it is within 5% of the way and returns the time taken.
    fn time_to_95_percent(omega: f64, dt: f64) -> f64 {
        let (mut x, mut v): (f64, f64) = (0.0, 0.0);
        let target = 100.0;
        let mut t = 0.0;
        while (target - x).abs() > 5.0 {
            (x, v) = spring_step(x, v, target, omega, dt);
            t += dt;
            assert!(t < 5.0, "spring never arrived");
        }
        t
    }

    #[test]
    fn reaches_95_percent_within_the_preset_times() {
        for (feel, expected) in [(Feel::Snappy, 0.17), (Feel::Smooth, 0.30), (Feel::Floaty, 0.53)] {
            let t = time_to_95_percent(feel.omega(), 1.0 / 120.0);
            assert!(t <= expected + 0.01, "{feel:?} took {t:.3} s, expected about {expected} s");
            assert!(t >= expected - 0.02, "{feel:?} arrived suspiciously early at {t:.3} s");
        }
    }

    #[test]
    fn never_overshoots_for_any_frame_time() {
        for dt_ms in 1..=33 {
            let dt = dt_ms as f64 / 1000.0;
            for omega in [28.0, 16.0, 9.0] {
                let (mut x, mut v): (f64, f64) = (0.0, 0.0);
                let mut previous = x;
                for _ in 0..2000 {
                    (x, v) = spring_step(x, v, 100.0, omega, dt);
                    assert!(x <= 100.0 + 1e-9, "overshoot at dt {dt_ms} ms, omega {omega}: x = {x}");
                    assert!(x >= previous - 1e-9, "went backwards at dt {dt_ms} ms, omega {omega}");
                    previous = x;
                }
                assert!((100.0 - x).abs() < 1e-3, "did not converge at dt {dt_ms} ms, omega {omega}");
            }
        }
    }

    #[test]
    fn one_long_step_matches_two_short_steps() {
        for omega in [28.0, 16.0, 9.0] {
            let (x0, v0) = (10.0, 250.0);
            let (long_x, long_v) = spring_step(x0, v0, 300.0, omega, 0.032);
            let (mid_x, mid_v) = spring_step(x0, v0, 300.0, omega, 0.016);
            let (short_x, short_v) = spring_step(mid_x, mid_v, 300.0, omega, 0.016);
            assert!((long_x - short_x).abs() < 0.5, "x differs by {}", (long_x - short_x).abs());
            assert!((long_v - short_v).abs() < 0.5, "v differs by {}", (long_v - short_v).abs());
        }
    }

    #[test]
    fn clamps_frame_time_to_a_thirtieth() {
        assert_eq!(clamp_dt(0.5), MAX_DT);
        assert_eq!(clamp_dt(-1.0), 0.0);
        assert_eq!(clamp_dt(0.008), 0.008);
    }

    #[test]
    fn caps_speed_and_keeps_direction() {
        let (vx, vy) = cap_speed(6000.0, 8000.0, 6000.0);
        let speed = (vx * vx + vy * vy).sqrt();
        assert!((speed - 6000.0).abs() < 1e-9);
        assert!((vx / vy - 0.75).abs() < 1e-9);
        assert_eq!(cap_speed(100.0, -200.0, 6000.0), (100.0, -200.0));
        assert_eq!(cap_speed(0.0, 0.0, 6000.0), (0.0, 0.0));
    }

    #[test]
    fn settles_only_inside_the_dead_zone() {
        let cfg = FollowConfig::new(Feel::Smooth, Distance::Normal);
        assert!(is_settled(0.0, 0.1, 1.0, &cfg));
        assert!(!is_settled(3.0, 0.1, 1.0, &cfg), "target still moving");
        assert!(!is_settled(0.0, 2.0, 1.0, &cfg), "not there yet");
        assert!(!is_settled(0.0, 0.1, 20.0, &cfg), "still fast");
    }
}
