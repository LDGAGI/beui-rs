//! Motion tokens — a byte-for-byte port of `lib/ease.ts`.
//!
//! Two kinds of value live here:
//!
//! - **Springs.** `stiffness` / `damping` / `mass` are copied verbatim into
//!   [`dioxus_motion::prelude::Spring`], which integrates the same
//!   mass-spring-damper ODE as Motion, so the curves match without a
//!   conversion. `SPRING_MOUSE` carries no `type` field in the source (it is
//!   never passed as an animation) and is kept as the same raw numbers.
//! - **Easing.** Motion's `easeOut` / `easeInOut` / `easeDrawer` are custom
//!   cubic-béziers, not the CSS keywords of the same name. [`cubic_bezier`] is
//!   the port of `motion-utils/dist/es/easing/cubic-bezier.mjs` from motion
//!   12.43.0 — bisection to a `1e-7` x-tolerance, at most 12 iterations — so a
//!   sampled curve is identical rather than merely close.

use dioxus_motion::prelude::{AnimationConfig, AnimationMode, Duration, Spring, Tween};

/// `EASE_OUT` — `[0.16, 1, 0.3, 1]`.
pub const EASE_OUT: [f64; 4] = [0.16, 1.0, 0.3, 1.0];
/// `EASE_IN_OUT` — `[0.77, 0, 0.175, 1]`.
pub const EASE_IN_OUT: [f64; 4] = [0.77, 0.0, 0.175, 1.0];
/// `EASE_DRAWER` — `[0.32, 0.72, 0, 1]`.
pub const EASE_DRAWER: [f64; 4] = [0.32, 0.72, 0.0, 1.0];

/// CSS string form of [`EASE_OUT`], for inline `transition` styles.
pub const EASE_OUT_CSS: &str = "cubic-bezier(0.16, 1, 0.3, 1)";

/// Press feedback on buttons and other tappable surfaces.
pub const SPRING_PRESS: Spring = Spring {
    stiffness: 500.0,
    damping: 30.0,
    mass: 0.6,
};

/// Content swaps — label/icon slots trading places inside a control.
pub const SPRING_SWAP: Spring = Spring {
    stiffness: 460.0,
    damping: 30.0,
    mass: 0.55,
};

/// Overlay panel entrances — modals and sheets summoned by pointer.
pub const SPRING_PANEL: Spring = Spring {
    stiffness: 420.0,
    damping: 40.0,
    mass: 0.5,
};

/// Shared-layout glides — pills, indicators and panels morphing between positions.
pub const SPRING_LAYOUT: Spring = Spring {
    stiffness: 360.0,
    damping: 32.0,
    mass: 0.6,
};

/// Cursor-follow physics for decorative mouse tracking (magnetic, tilt, dock).
pub const SPRING_MOUSE: Spring = Spring {
    stiffness: 200.0,
    damping: 15.0,
    mass: 0.3,
};

/// Dragged handles and fills (sliders) — critically damped, so the value follows
/// the pointer and never rebounds off an end.
pub const SPRING_GLIDE: Spring = Spring {
    stiffness: 700.0,
    damping: 50.0,
    mass: 0.5,
};

/// `subdivisionPrecision` in motion's `cubic-bezier.mjs`.
const SUBDIVISION_PRECISION: f64 = 0.0000001;
/// `subdivisionMaxIterations` in motion's `cubic-bezier.mjs`.
const SUBDIVISION_MAX_ITERATIONS: i32 = 12;

/// `calcBezier(t, a1, a2)` — the Bernstein form, exactly 0 at `t = 0` and 1 at `t = 1`.
fn calc_bezier(t: f64, a1: f64, a2: f64) -> f64 {
    (((1.0 - 3.0 * a2 + 3.0 * a1) * t + (3.0 * a2 - 6.0 * a1)) * t + 3.0 * a1) * t
}

/// `binarySubdivide(...)` — solve `calcBezier(t, mX1, mX2) = x` for `t`.
fn binary_subdivide(
    x: f64,
    mut lower_bound: f64,
    mut upper_bound: f64,
    m_x1: f64,
    m_x2: f64,
) -> f64 {
    let mut current_x;
    let mut current_t;
    let mut i = 0;
    loop {
        current_t = lower_bound + (upper_bound - lower_bound) / 2.0;
        current_x = calc_bezier(current_t, m_x1, m_x2) - x;
        if current_x > 0.0 {
            upper_bound = current_t;
        } else {
            lower_bound = current_t;
        }
        i += 1;
        if !(current_x.abs() > SUBDIVISION_PRECISION && i < SUBDIVISION_MAX_ITERATIONS) {
            break;
        }
    }
    current_t
}

/// `cubicBezier(mX1, mY1, mX2, mY2)`: the easing curve as `x -> y`, `x` in `[0, 1]`.
///
/// A linear gradient (`mX1 == mY1 && mX2 == mY2`) is the identity, matching
/// motion's `noop` shortcut. Out-of-range `x` saturates the way motion's
/// bisection bracket does.
pub fn cubic_bezier_value(m_x1: f64, m_y1: f64, m_x2: f64, m_y2: f64, x: f64) -> f64 {
    if m_x1 == m_y1 && m_x2 == m_y2 {
        return x;
    }
    if x == 0.0 || x == 1.0 {
        return x;
    }
    calc_bezier(binary_subdivide(x, 0.0, 1.0, m_x1, m_x2), m_y1, m_y2)
}

/// [`cubic_bezier_value`] bound to one curve, for repeated sampling.
pub fn cubic_bezier(m_x1: f64, m_y1: f64, m_x2: f64, m_y2: f64) -> impl Fn(f64) -> f64 {
    move |x| cubic_bezier_value(m_x1, m_y1, m_x2, m_y2, x)
}

/// `EASE_OUT` sampled at `x`.
pub fn ease_out(x: f64) -> f64 {
    cubic_bezier_value(EASE_OUT[0], EASE_OUT[1], EASE_OUT[2], EASE_OUT[3], x)
}

/// `EASE_IN_OUT` sampled at `x`.
pub fn ease_in_out(x: f64) -> f64 {
    cubic_bezier_value(
        EASE_IN_OUT[0],
        EASE_IN_OUT[1],
        EASE_IN_OUT[2],
        EASE_IN_OUT[3],
        x,
    )
}

/// `EASE_DRAWER` sampled at `x`.
pub fn ease_drawer(x: f64) -> f64 {
    cubic_bezier_value(
        EASE_DRAWER[0],
        EASE_DRAWER[1],
        EASE_DRAWER[2],
        EASE_DRAWER[3],
        x,
    )
}

// `dioxus-motion` tweens take an `easer`-shaped function, `f(t, b, c, d) = b + c * f(t / d)`.

/// [`EASE_OUT`] in the `easer` shape.
pub fn ease_out_easer(t: f32, b: f32, c: f32, d: f32) -> f32 {
    b + c * ease_out((t / d) as f64) as f32
}

/// [`EASE_IN_OUT`] in the `easer` shape.
pub fn ease_in_out_easer(t: f32, b: f32, c: f32, d: f32) -> f32 {
    b + c * ease_in_out((t / d) as f64) as f32
}

/// [`EASE_DRAWER`] in the `easer` shape.
pub fn ease_drawer_easer(t: f32, b: f32, c: f32, d: f32) -> f32 {
    b + c * ease_drawer((t / d) as f64) as f32
}

/// A tween on [`EASE_OUT`] — the source's `transition={{ duration, ease: EASE_OUT }}`.
pub fn tween_ease_out(duration_ms: u64) -> AnimationConfig {
    AnimationConfig::new(AnimationMode::Tween(
        Tween::new(Duration::from_millis(duration_ms)).with_easing(ease_out_easer),
    ))
}

/// A tween on [`EASE_IN_OUT`].
pub fn tween_ease_in_out(duration_ms: u64) -> AnimationConfig {
    AnimationConfig::new(AnimationMode::Tween(
        Tween::new(Duration::from_millis(duration_ms)).with_easing(ease_in_out_easer),
    ))
}

/// A tween on [`EASE_DRAWER`].
pub fn tween_ease_drawer(duration_ms: u64) -> AnimationConfig {
    AnimationConfig::new(AnimationMode::Tween(
        Tween::new(Duration::from_millis(duration_ms)).with_easing(ease_drawer_easer),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Endpoints and monotonicity of the three tokens.
    #[test]
    fn easing_endpoints() {
        for ease in [ease_out, ease_in_out, ease_drawer] {
            assert_eq!(ease(0.0), 0.0);
            assert_eq!(ease(1.0), 1.0);
            let mut previous = 0.0;
            for step in 1..=100 {
                let y = ease(step as f64 / 100.0);
                assert!(y >= previous - 1e-12, "not monotonic at {step}");
                previous = y;
            }
        }
    }

    /// `EASE_OUT` front-loads its travel: at a quarter of the duration it has
    /// already covered four fifths of the distance. The four values are motion's
    /// own samples from `tests/fixtures/ease-samples.txt`.
    #[test]
    fn ease_out_is_front_loaded() {
        for (x, expected) in [
            (0.10, 0.494_441_545_582_958_5),
            (0.25, 0.825_475_045_232_451_5),
            (0.50, 0.971_782_436_143_257_7),
            (0.90, 0.999_878_267_015_446_9),
        ] {
            assert!(
                (ease_out(x) - expected).abs() < 1e-15,
                "ease_out({x}) = {}",
                ease_out(x)
            );
        }
    }

    /// The linear shortcut in motion's `cubicBezier` survives the port.
    #[test]
    fn linear_gradient_is_identity() {
        for x in [0.0, 0.13, 0.5, 0.99, 1.0] {
            assert_eq!(cubic_bezier_value(0.5, 0.5, 0.5, 0.5, x), x);
        }
    }

    /// Spring tokens keep the source numbers exactly.
    #[test]
    fn spring_tokens_are_verbatim() {
        assert_eq!(
            (
                SPRING_PRESS.stiffness,
                SPRING_PRESS.damping,
                SPRING_PRESS.mass
            ),
            (500.0, 30.0, 0.6)
        );
        assert_eq!(
            (SPRING_SWAP.stiffness, SPRING_SWAP.damping, SPRING_SWAP.mass),
            (460.0, 30.0, 0.55)
        );
        assert_eq!(
            (
                SPRING_PANEL.stiffness,
                SPRING_PANEL.damping,
                SPRING_PANEL.mass
            ),
            (420.0, 40.0, 0.5)
        );
        assert_eq!(
            (
                SPRING_LAYOUT.stiffness,
                SPRING_LAYOUT.damping,
                SPRING_LAYOUT.mass
            ),
            (360.0, 32.0, 0.6)
        );
        assert_eq!(
            (
                SPRING_MOUSE.stiffness,
                SPRING_MOUSE.damping,
                SPRING_MOUSE.mass
            ),
            (200.0, 15.0, 0.3)
        );
        assert_eq!(
            (
                SPRING_GLIDE.stiffness,
                SPRING_GLIDE.damping,
                SPRING_GLIDE.mass
            ),
            (700.0, 50.0, 0.5)
        );
    }
}
