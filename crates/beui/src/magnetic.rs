//! `motion/magnetic.tsx` — a decorative cursor-follow pull.
//!
//! `SPRING_MOUSE` on two axes, driven from the pointer's offset from the
//! element's centre. Skipped on touch (phantom hover) and under reduced motion,
//! exactly like the source.

use crate::prelude::*;
use crate::rect::{use_element_rect, ElementRect};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

/// Read a spring's current value reactively, so the component re-renders as it
/// moves. (`MotionHandle::get_value` reads the store untracked.)
fn value(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// Props for [`Magnetic`].
#[derive(Props, Clone, PartialEq)]
pub struct MagneticProps {
    pub children: Element,
    /// Fraction of the pointer offset applied as translation. Defaults to `0.35`.
    #[props(default = 0.35)]
    pub strength: f64,
    #[props(default)]
    pub class: Option<String>,
}

/// A wrapper whose contents lean toward the cursor.
#[component]
pub fn Magnetic(props: MagneticProps) -> Element {
    let MagneticProps {
        children,
        strength,
        class,
    } = props;
    let reduce = use_reduced_motion();
    let can_hover = use_hover_capable();
    let enabled = !reduce() && can_hover();

    let mut x = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let mut y = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let rect: ElementRect = use_element_rect();

    let style = format!(
        "transform:translateX({}px) translateY({}px)",
        value(x),
        value(y)
    );
    let class = cn!("inline-block", class);

    rsx! {
        div {
            class,
            style,
            "data-beui-rect": rect.attr(),
            onmounted: move |data| rect.measure(data.data()),
            onmouseenter: move |_| rect.refresh(),
            onmousemove: move |event: Event<MouseData>| {
                if !enabled {
                    return;
                }
                let Some(bounds) = rect.get() else {
                    return;
                };
                let point = event.client_coordinates();
                let target_x = (point.x - bounds.left - bounds.width / 2.0) * strength;
                let target_y = (point.y - bounds.top - bounds.height / 2.0) * strength;
                let _ = x.animate_to(target_x as f32, AnimationConfig::spring(SPRING_MOUSE));
                let _ = y.animate_to(target_y as f32, AnimationConfig::spring(SPRING_MOUSE));
            },
            onmouseleave: move |_| {
                let _ = x.animate_to(0.0, AnimationConfig::spring(SPRING_MOUSE));
                let _ = y.animate_to(0.0, AnimationConfig::spring(SPRING_MOUSE));
            },
            {children}
        }
    }
}
