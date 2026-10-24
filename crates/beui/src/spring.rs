//! Motion's time-defined springs, converted to the physics form.
//!
//! Motion lets a spring be described by a `duration` (seconds) plus a `bounce`
//! instead of `stiffness`/`damping`/`mass`. `dioxus-motion` takes only the
//! physics form, so these convert — a port of
//! `motion-dom/src/animation/generators/spring.ts` at motion 12.43.0
//! (`safeMin = 0.001`, `minDuration = 0.01`, `maxDuration = 10`,
//! `minDamping = 0.05`, `maxDamping = 1`, mass 1).
//!
//! `findSpring` solves `envelope(ω) = safeMin` for the undamped frequency by
//! Newton iteration. With zero velocity that has a closed form, used here:
//!
//! - underdamped (`ζ < 1`): `(ζ/√(1-ζ²))·e^(-ζωd) = safeMin`, so
//!   `ω = ln(ζ / (safeMin·√(1-ζ²))) / (ζ·d)`;
//! - critically damped (`ζ = 1`): `e^(-ωd)·(ωd + 1) = safeMin`, solved by
//!   bisection over `ωd` (the left side is strictly decreasing from 1 to 0).
//!
//! The `visualDuration` fast path is Motion's other branch:
//! `ω = 2π / (visualDuration·1.2)`, `stiffness = ω²`,
//! `damping = 2ζ√stiffness`.

use dioxus_motion::prelude::Spring;

/// `safeMin` — the normalized distance at which `findSpring` calls it settled.
const SAFE_MIN: f64 = 0.001;
const MIN_DURATION: f64 = 0.01;
const MAX_DURATION: f64 = 10.0;
const MIN_DAMPING: f64 = 0.05;
const MAX_DAMPING: f64 = 1.0;

fn clamp(lo: f64, hi: f64, value: f64) -> f64 {
    value.max(lo).min(hi)
}

/// `dampingRatio = clamp(0.05, 1, 1 - bounce)`.
fn damping_ratio(bounce: f64) -> f64 {
    clamp(MIN_DAMPING, MAX_DAMPING, 1.0 - bounce)
}

/// `findSpring`'s `envelope`, at zero velocity. Zero is the root. Only used
/// by the tests now that the root is solved in closed form.
#[cfg(test)]
fn envelope(undamped_freq: f64, ratio: f64, duration: f64) -> f64 {
    if ratio < 1.0 {
        let root = (1.0 - ratio * ratio).sqrt();
        SAFE_MIN - (ratio / root) * (-ratio * undamped_freq * duration).exp()
    } else {
        let x = undamped_freq * duration;
        -SAFE_MIN + (-x).exp() * (x + 1.0)
    }
}

fn undamped_frequency(duration: f64, ratio: f64) -> f64 {
    if ratio < 1.0 {
        let root = (1.0 - ratio * ratio).sqrt();
        (ratio / (SAFE_MIN * root)).ln() / (ratio * duration)
    } else {
        // `e^(-x)(x + 1) = safeMin`, x = ωd.
        let (mut lo, mut hi): (f64, f64) = (0.0, 200.0);
        for _ in 0..100 {
            let mid = 0.5 * (lo + hi);
            if (-mid).exp() * (mid + 1.0) > SAFE_MIN {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (0.5 * (lo + hi)) / duration
    }
}

/// A `{ duration, bounce }` spring, converted to `{ stiffness, damping, mass }`.
///
/// `duration` is in seconds, as Motion's `duration` option is.
pub fn spring_from_duration(duration: f64, bounce: f64) -> Spring {
    let ratio = damping_ratio(bounce);
    let duration = clamp(MIN_DURATION, MAX_DURATION, duration);
    let frequency = undamped_frequency(duration, ratio);
    if !frequency.is_finite() || frequency <= 0.0 {
        // `findSpring` falls back to the defaults when the root does not converge.
        return Spring {
            stiffness: 100.0,
            damping: 10.0,
            mass: 1.0,
        };
    }
    let stiffness = frequency * frequency;
    Spring {
        stiffness: stiffness as f32,
        damping: (ratio * 2.0 * stiffness.sqrt()) as f32,
        mass: 1.0,
    }
}

/// A `{ visualDuration, bounce }` spring (Motion's fast path).
///
/// `visual_duration` is in seconds.
pub fn spring_from_visual_duration(visual_duration: f64, bounce: f64) -> Spring {
    let ratio = damping_ratio(bounce);
    let root = (2.0 * std::f64::consts::PI) / (visual_duration * 1.2);
    let stiffness = root * root;
    Spring {
        stiffness: stiffness as f32,
        damping: (2.0 * ratio * stiffness.sqrt()) as f32,
        mass: 1.0,
    }
}

/// `spring({ duration, bounce })` — the same thing in a const-friendly call.
pub fn duration_spring(duration: f64, bounce: f64) -> Spring {
    spring_from_duration(duration, bounce)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The closed form lands on a root of `envelope`.
    #[test]
    fn closed_form_solves_the_envelope() {
        for duration in [0.2, 0.42, 0.46, 0.55, 0.58, 1.2] {
            for bounce in [0.0, 0.06, 0.15, 0.26, 0.28, 0.32, 0.38] {
                let ratio = damping_ratio(bounce);
                let frequency = undamped_frequency(duration, ratio);
                let residual = envelope(frequency, ratio, duration);
                assert!(
                    residual.abs() < 1e-6,
                    "duration {duration} bounce {bounce}: residual {residual}"
                );
            }
        }
    }

    /// Overdamped springs (`bounce <= 0`) keep a positive frequency.
    #[test]
    fn non_positive_bounce_is_critically_damped() {
        let spring = spring_from_duration(0.5, 0.0);
        assert!(spring.stiffness > 0.0);
        assert!(spring.damping > 0.0);
        // ζ = 1 exactly.
        let ratio = spring.damping as f64 / (2.0 * (spring.stiffness as f64).sqrt());
        assert!((ratio - 1.0).abs() < 1e-4, "ratio {ratio}");
    }

    /// The known bouncy-accordion numbers, so a change is visible.
    #[test]
    fn bouncy_accordion_row_spring() {
        let spring = spring_from_duration(0.55, 0.38);
        assert!(
            (spring.stiffness - 382.8).abs() < 1.0,
            "stiffness {}",
            spring.stiffness
        );
        assert!(
            (spring.damping - 24.26).abs() < 0.2,
            "damping {}",
            spring.damping
        );
        assert_eq!(spring.mass, 1.0);
    }

    /// The `visualDuration` fast path matches Motion's arithmetic.
    #[test]
    fn visual_duration_fast_path() {
        let spring = spring_from_visual_duration(0.3, 0.15);
        let root = 2.0 * std::f64::consts::PI / (0.3 * 1.2);
        assert!((spring.stiffness as f64 - root * root).abs() < 1e-3);
        assert!((spring.damping as f64 - 2.0 * 0.85 * root).abs() < 1e-3);
    }
}
