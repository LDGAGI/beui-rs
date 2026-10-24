//! Animation helpers the ported components share.
//!
//! `dioxus-motion` is imperative where `motion/react` is declarative: a
//! component holds a [`MotionHandle`], writes a target with `animate_to`, and
//! reads the current value while rendering (the value lives in a store, so
//! reading it subscribes the component to the next frame). These helpers keep
//! that boilerplate in one place and keep the springs and easings coming from
//! `beui-core::ease`, so a component file reads like the source it was ported
//! from:
//!
//! ```ignore
//! let mut scale = use_style_motion(motion_style! { scale: 1.0 });
//! // whileTap={{ scale: 0.92 }} on the element's own spring
//! onpointerdown: move |_| animate_style(&mut scale, motion_style! { scale: 0.92 }, spring(SPRING_PRESS)),
//! ```

use beui_core::ease::{ease_drawer_easer, ease_in_out_easer, ease_out_easer};
use dioxus::prelude::*;
pub use dioxus_motion::animations::style::MotionStyle;
use dioxus_motion::prelude::{
    use_motion, AnimationConfig, AnimationMode, Duration, LoopMode, MotionHandle, Tween,
};
use dioxus_motion::AnimationManager;

pub use beui_core::presence::motion_style;

/// The three beUI easing curves, for [`tween`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ease {
    /// `EASE_OUT` — `[0.16, 1, 0.3, 1]`.
    Out,
    /// `EASE_IN_OUT` — `[0.77, 0, 0.175, 1]`.
    InOut,
    /// `EASE_DRAWER` — `[0.32, 0.72, 0, 1]`.
    Drawer,
}

/// `transition={{ duration, ease }}` on one of the three beUI easing tokens.
///
/// The curve runs through `beui-core::ease`'s port of motion's own
/// `cubicBezier`, so a sampled tween matches the source rather than merely
/// resembling it.
pub fn tween(duration_ms: u64, ease: Ease) -> AnimationConfig {
    let easing = match ease {
        Ease::Out => ease_out_easer,
        Ease::InOut => ease_in_out_easer,
        Ease::Drawer => ease_drawer_easer,
    };
    AnimationConfig::new(AnimationMode::Tween(
        Tween::new(Duration::from_millis(duration_ms)).with_easing(easing),
    ))
}

/// `{ type: "spring", stiffness, damping, mass }` from a `beui-core::ease` token.
pub fn spring(token: dioxus_motion::prelude::Spring) -> AnimationConfig {
    AnimationConfig::spring(token)
}

/// A linear tween, for the loops that must not be shaped by a curve.
pub fn tween_linear(duration_ms: u64) -> AnimationConfig {
    AnimationConfig::new(AnimationMode::Tween(Tween::new(Duration::from_millis(
        duration_ms,
    ))))
}

/// A looping `0 → 1` phase, for animations that repeat.
///
/// Motion's `repeat: Infinity` keyframes become a single linear loop whose
/// value is the cycle phase; the caller evaluates its keyframe tracks against
/// [`track`] each render, which keeps the curve exact (a tween between two
/// keyframes would not be) and needs no completion callback. The loop restarts
/// at `0` every `duration_ms`, and every track that matters is continuous there.
pub fn use_loop_phase(duration_ms: u64) -> MotionHandle<f32> {
    let mut phase = use_number_motion(0.0);
    use_effect(use_reactive!(|duration_ms| {
        let config = tween_linear(duration_ms.max(1)).with_loop(LoopMode::Infinite);
        let _ = phase.animate_to(1.0, config);
    }));
    phase
}

/// Evaluate a keyframe track at `phase` (clamped to `0..=1`), applying `ease`
/// within each segment.
///
/// This is motion's keyframe interpolation for a list of evenly spaced values
/// with one easing function: `[a, b, c]` is `a → b → c` across equal thirds,
/// each third eased. beUI's loaders express their repeats this way.
pub fn track(values: &[f32], phase: f32, ease: fn(f64) -> f64) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return values[0];
    }
    let phase = phase.clamp(0.0, 1.0);
    let segments = (values.len() - 1) as f32;
    let scaled = phase * segments;
    let index = (scaled.floor() as usize).min(values.len() - 2);
    let local = (scaled - index as f32).clamp(0.0, 1.0);
    let eased = ease(f64::from(local)) as f32;
    values[index] + (values[index + 1] - values[index]) * eased
}

/// [`track`] with a per-keyframe delay expressed as a phase offset.
///
/// motion's `delay` on a repeated keyframe animation shifts the whole track;
/// the looped phase cannot carry a delay without pausing every cycle, so the
/// caller adds the offset and wraps.
pub fn track_offset(values: &[f32], phase: f32, offset: f32, ease: fn(f64) -> f64) -> f32 {
    let wrapped = (phase + offset).rem_euclid(1.0);
    track(values, wrapped, ease)
}

/// A keyframe track with explicit times — motion's `times` option.
///
/// `times` must be non-decreasing, start at `0` and end at `1`; a `phase`
/// outside the range clamps to the first or last value.
pub fn track_at(values: &[f32], times: &[f32], phase: f32, ease: fn(f64) -> f64) -> f32 {
    if values.is_empty() || times.len() != values.len() {
        return values.first().copied().unwrap_or(0.0);
    }
    if values.len() == 1 || phase <= times[0] {
        return values[0];
    }
    if phase >= *times.last().expect("non-empty") {
        return *values.last().expect("non-empty");
    }
    let index = times
        .windows(2)
        .position(|pair| phase <= pair[1])
        .unwrap_or(times.len() - 2);
    let span = times[index + 1] - times[index];
    let local = if span > 0.0 {
        (phase - times[index]) / span
    } else {
        0.0
    };
    let eased = ease(f64::from(local.clamp(0.0, 1.0))) as f32;
    values[index] + (values[index + 1] - values[index]) * eased
}

/// The current CSS declaration list of a style handle.
///
/// A frame writes to the store, the render reads it, and the renderer applies
/// the `style` attribute natively on every platform — the path
/// `beui-core::presence` uses, and the reason animation works on the WebView
/// renderers too.
pub fn css(handle: &MotionHandle<MotionStyle>) -> String {
    handle.get_value().to_css()
}

/// [`use_motion`] for a [`MotionStyle`], whose initial value is always finite.
pub fn use_style_motion(initial: MotionStyle) -> MotionHandle<MotionStyle> {
    use_motion(initial).expect("MotionStyle initial values are finite")
}

/// [`use_motion`] for an `f32`, which is only used with finite literals.
pub fn use_number_motion(initial: f32) -> MotionHandle<f32> {
    use_motion(initial).expect("number motion initial values are finite")
}

/// `animate(handle, target, config)`, ignoring a rejected configuration.
///
/// A rejection only happens for a non-finite value or a spring between
/// incompatible shapes; both are programmer errors the source would throw on,
/// and there is no error channel back into `rsx!` here.
pub fn animate_style(
    handle: &mut MotionHandle<MotionStyle>,
    target: MotionStyle,
    config: AnimationConfig,
) {
    let _ = handle.animate_to(target, config);
}

/// `animate(handle, target, config)` for a plain number.
pub fn animate_number(handle: &mut MotionHandle<f32>, target: f32, config: AnimationConfig) {
    let _ = handle.animate_to(target, config);
}

/// `transform: scale(x)` for a number motion, always emitted.
///
/// The `transform` declaration is never omitted: the web interpreter restores
/// inline properties a new `style` value does not mention, so writing `""` at
/// the end of an animation would leave the last frame's transform pinned — the
/// same constraint `beui-core::flip` works under. `scale(1)` is the identity.
pub fn scale_style(scale: f32) -> String {
    format!("transform:scale({scale})")
}

/// The current value of a number motion.
pub fn number(handle: &MotionHandle<f32>) -> f32 {
    handle.get_value()
}
