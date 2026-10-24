//! `motion/animated-number.tsx` — a count-up to `value` on an `EASE_OUT` tween.
//!
//! The tween runs from wherever the value currently is to the new one, so a
//! `value` change mid-flight continues from the visible number rather than
//! snapping back. `startOnView` waits for the element to be 60% visible, and
//! reduced motion jumps straight to the target.

use crate::prelude::*;
use crate::scroll::{use_in_view, InViewAmount, InViewOptions};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationManager, MotionHandle};

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// `Math.round(n).toLocaleString()` for the `en-US` grouping the default
/// formatter uses (Rust has no locale-aware integer formatter in `core`).
pub fn locale_integer(value: f64) -> String {
    let rounded = value.round();
    if !rounded.is_finite() {
        return rounded.to_string();
    }
    let negative = rounded < 0.0;
    let digits = format!("{}", rounded.abs() as u64);
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(character);
    }
    if negative {
        out.insert(0, '-');
    }
    out
}

/// Props for [`AnimatedNumber`].
#[derive(Props, Clone, PartialEq)]
pub struct AnimatedNumberProps {
    pub value: f64,
    /// Count duration in seconds. Defaults to `1.2`.
    #[props(default = 1.2)]
    pub duration: f64,
    /// Custom formatter. Defaults to `Math.round(n).toLocaleString()`.
    #[props(default)]
    pub format: Option<Callback<f64, String>>,
    #[props(default)]
    pub class: Option<String>,
    /// Wait until the element is in view. Defaults to `true`.
    #[props(default = true)]
    pub start_on_view: bool,
}

/// A number that counts up to its value.
#[component]
pub fn AnimatedNumber(props: AnimatedNumberProps) -> Element {
    let AnimatedNumberProps {
        value,
        duration,
        format,
        class,
        start_on_view,
    } = props;

    let reduce = use_reduced_motion();
    let view = use_in_view(InViewOptions {
        once: true,
        amount: InViewAmount::Fraction(0.6),
        root: None,
    });
    let mut motion = use_motion(0.0f32).expect("0.0 is a finite animation value");

    let in_view = view.signal();
    use_effect(use_reactive(
        (&value, &duration, &start_on_view),
        move |(value, duration, start_on_view)| {
            if start_on_view && !in_view() {
                return;
            }
            let config = if reduce() {
                beui_core::ease::tween_ease_out(0)
            } else {
                beui_core::ease::tween_ease_out((duration * 1000.0).round() as u64)
            };
            let _ = motion.animate_to(value as f32, config);
        },
    ));

    let display = f64::from(current(motion));
    let text = match format {
        Some(format) => format.call(display),
        None => locale_integer(display),
    };
    let class = cn!("tabular-nums", class);

    rsx! {
        span { class, "data-beui-inview": view.attr(), "{text}" }
    }
}
