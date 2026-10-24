//! The `visualDuration + bounce` → spring conversion.
//!
//! Several beUI components (popover's goo, select's unfold, bouncy-accordion,
//! tooltip's arrow) configure springs in motion's *visual* form rather than by
//! physics. `dioxus-motion` only takes `stiffness/damping/mass`, so this is the
//! conversion motion itself performs, copied from
//! `packages/motion-dom/src/animation/generators/spring.ts`:
//!
//! ```js
//! if (options.visualDuration) {
//!   const root = (2 * Math.PI) / (visualDuration * 1.2)
//!   const stiffness = root * root
//!   const damping =
//!     2 * clamp(0.05, 1, 1 - (options.bounce || 0)) * Math.sqrt(stiffness)
//!   springOptions = { mass: 1.0, stiffness, damping }
//! }
//! ```
//!
//! The `* 1.2` is not a typo: motion stretches the angular frequency so that the
//! bulk of the motion lands *before* `visualDuration`, which is what "visual"
//! means. It matters — with plain `2π / duration` every converted spring here
//! would come out roughly 45% too stiff.

use dioxus_motion::prelude::Spring;

/// `clamp(0.05, 1, value)` — motion's `clamp(min, max, value)`.
fn clamp_damping_ratio(value: f64) -> f64 {
    value.clamp(0.05, 1.0)
}

/// Convert motion's `visualDuration` (seconds) + `bounce` (`-1..=1`) into the
/// `stiffness/damping/mass` a [`Spring`] takes. `mass` is always 1.0, as in the
/// source; the conversion ignores a caller-supplied mass for the same reason.
pub fn spring_from_visual_duration(visual_duration: f64, bounce: f64) -> Spring {
    let root = (2.0 * std::f64::consts::PI) / (visual_duration * 1.2);
    let stiffness = root * root;
    let damping = 2.0 * clamp_damping_ratio(1.0 - bounce) * stiffness.sqrt();
    Spring {
        stiffness: stiffness as f32,
        damping: damping as f32,
        mass: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_motions_visual_duration_formula() {
        // popover's GOO_OPEN_SPRING: visualDuration 0.3, bounce 0.15.
        let spring = spring_from_visual_duration(0.3, 0.15);
        let root = (2.0 * std::f64::consts::PI) / (0.3 * 1.2);
        let expected_stiffness = root * root;
        let expected_damping = 2.0 * 0.85 * expected_stiffness.sqrt();
        assert!((f64::from(spring.stiffness) - expected_stiffness).abs() < 1.0e-3);
        assert!((f64::from(spring.damping) - expected_damping).abs() < 1.0e-3);
        assert_eq!(spring.mass, 1.0);
    }

    #[test]
    fn bounce_zero_is_critically_damped() {
        let spring = spring_from_visual_duration(0.4, 0.0);
        // dampingRatio 1 → damping = 2 * sqrt(stiffness * mass).
        let expected = 2.0 * f64::from(spring.stiffness).sqrt();
        assert!((f64::from(spring.damping) - expected).abs() < 1.0e-3);
    }

    #[test]
    fn damping_ratio_is_clamped() {
        // bounce 2 would give a ratio of -1; motion clamps to 0.05.
        let spring = spring_from_visual_duration(0.4, 2.0);
        let expected = 2.0 * 0.05 * f64::from(spring.stiffness).sqrt();
        assert!((f64::from(spring.damping) - expected).abs() < 1.0e-3);
    }
}
