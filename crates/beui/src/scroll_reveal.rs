//! `motion/scroll-reveal.tsx` — a one-shot (or repeating) reveal as the element
//! enters its scroll root.
//!
//! One `EASE_OUT` tween drives opacity, the slide and the blur together, which
//! is what the source's three transition entries do with the same duration and
//! easing.

use crate::prelude::*;
use crate::scroll::{use_in_view, InViewAmount, InViewOptions};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationManager, MotionHandle};

fn value(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// Props for [`ScrollReveal`].
#[derive(Props, Clone, PartialEq)]
pub struct ScrollRevealProps {
    pub children: Element,
    /// Slide distance in px before reveal. Defaults to `16`.
    #[props(default = 16.0)]
    pub y: f64,
    /// Enter blur in px. Defaults to `8`.
    #[props(default = 8.0)]
    pub blur: f64,
    /// Reveal duration in seconds. Defaults to `0.6`.
    #[props(default = 0.6)]
    pub duration: f64,
    #[props(default)]
    pub delay: f64,
    /// Reveal once (default) or every time the element enters view.
    #[props(default = true)]
    pub once: bool,
    /// Portion of the element that must be visible. Defaults to `0.3`.
    #[props(default = InViewAmount::Fraction(0.3))]
    pub amount: InViewAmount,
    /// CSS selector for the scroll root. `None` is the viewport.
    #[props(default)]
    pub root: Option<String>,
    #[props(default)]
    pub class: Option<String>,
}

/// Reveal its children when they scroll into view.
#[component]
pub fn ScrollReveal(props: ScrollRevealProps) -> Element {
    let ScrollRevealProps {
        children,
        y,
        blur,
        duration,
        delay,
        once,
        amount,
        root,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let view = use_in_view(InViewOptions { once, amount, root });
    let mut progress = use_motion(0.0f32).expect("0.0 is a finite animation value");

    // The signal is read *inside* the effect: capturing the render-time bool
    // would subscribe the effect to nothing, so it would run once at mount
    // (hidden) and never again when the element scrolled into view.
    let in_view = view.signal();
    use_effect(move || {
        let target = if in_view() { 1.0 } else { 0.0 };
        let config = beui_core::ease::tween_ease_out((duration * 1000.0).round() as u64)
            .with_delay(dioxus_motion::Duration::from_millis(
                (delay * 1000.0).round() as u64,
            ));
        let _ = progress.animate_to(target, config);
    });

    let p = value(progress);
    let style = if reduce() {
        format!("opacity:{p}")
    } else {
        format!(
            "opacity:{p};transform:translateY({}px);filter:blur({}px)",
            y * f64::from(1.0 - p),
            blur * f64::from(1.0 - p)
        )
    };
    let class = cn!(class);

    rsx! {
        div {
            class,
            style,
            "data-beui-inview": view.attr(),
            {children}
        }
    }
}
