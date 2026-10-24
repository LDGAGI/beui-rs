//! `motion/parallax.tsx` — an element drifting against (or with) the scroll.
//!
//! Progress comes from [`use_parallax_progress`] (`useScroll` with the
//! `["start end", "end start"]` offset), mapped to a symmetric drift and
//! optionally softened by `PARALLAX_SPRING`. Reduced motion drops the drift.

use crate::prelude::*;
use crate::scroll::{use_parallax_progress, use_spring_value};
use dioxus_motion::prelude::Spring;

/// `PARALLAX_SPRING` — a soft follow, looser than the UI springs.
const PARALLAX_SPRING: Spring = Spring {
    stiffness: 120.0,
    damping: 30.0,
    mass: 0.6,
};

/// The axis the drift is applied on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParallaxAxis {
    X,
    #[default]
    Y,
}

/// Props for [`Parallax`].
#[derive(Props, Clone, PartialEq)]
pub struct ParallaxProps {
    pub children: Element,
    /// Drift as a fraction of the element's travel. Defaults to `0.3`.
    #[props(default = 0.3)]
    pub speed: f64,
    #[props(default)]
    pub axis: ParallaxAxis,
    /// CSS selector for the scroll container. `None` is the viewport.
    #[props(default)]
    pub container: Option<String>,
    /// Spring-smooth the drift. Defaults to `true`; disabled under reduced motion.
    #[props(default = true)]
    pub spring: bool,
    #[props(default)]
    pub class: Option<String>,
}

/// A parallax wrapper.
#[component]
pub fn Parallax(props: ParallaxProps) -> Element {
    let ParallaxProps {
        children,
        speed,
        axis,
        container,
        spring,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let progress = use_parallax_progress(container);
    let travel = speed * 100.0;
    // Motion's `useTransform(scrollYProgress, [0, 1], [travel, -travel])` is
    // `travel * (1 - 2p)`. A derived signal is how the spring follows it.
    let mut derived = use_signal(|| travel);
    let raw = progress.signal();
    use_effect(move || {
        derived.set(travel * (1.0 - 2.0 * raw()));
    });

    let smoothed = use_spring_value(derived, PARALLAX_SPRING, spring && !reduce());
    let value = if spring && !reduce() {
        f64::from(smoothed)
    } else {
        derived()
    };

    let style = if reduce() {
        String::new()
    } else if axis == ParallaxAxis::X {
        format!("transform:translateX({value}px)")
    } else {
        format!("transform:translateY({value}px)")
    };
    let class = cn!(class);

    rsx! {
        div {
            class,
            style,
            "data-beui-parallax": progress.attr(),
            {children}
        }
    }
}
