//! `motion/tilt-card.tsx` — a 3D tilt plus a cursor-tracking glare.
//!
//! Rotation is two `SPRING_MOUSE` axes; the glare origin is the raw pointer
//! ratio, so it sits under the cursor rather than trailing it. Skipped on touch
//! and under reduced motion.

use crate::prelude::*;
use crate::rect::{use_element_rect, ElementRect};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

fn value(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// Props for [`TiltCard`].
#[derive(Props, Clone, PartialEq)]
pub struct TiltCardProps {
    pub children: Element,
    /// Maximum rotation in degrees at the edges. Defaults to `12`.
    #[props(default = 12.0)]
    pub max: f64,
    /// Render the radial glare. Defaults to `true`.
    #[props(default = true)]
    pub glare: bool,
    #[props(default)]
    pub class: Option<String>,
}

/// A card that tilts toward the cursor.
#[component]
pub fn TiltCard(props: TiltCardProps) -> Element {
    let TiltCardProps {
        children,
        max,
        glare,
        class,
    } = props;
    let reduce = use_reduced_motion();
    let can_hover = use_hover_capable();
    let enabled = !reduce() && can_hover();

    let mut rotate_x = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let mut rotate_y = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let mut glare_x = use_signal(|| 50.0f64);
    let mut glare_y = use_signal(|| 50.0f64);
    let rect: ElementRect = use_element_rect();

    let style = format!(
        "transform:perspective(1000px) rotateX({}deg) rotateY({}deg);transform-style:preserve-3d",
        value(rotate_x),
        value(rotate_y)
    );
    let class = cn!(
        "relative overflow-hidden rounded-2xl will-change-transform",
        class
    );

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
                if bounds.width == 0.0 || bounds.height == 0.0 {
                    return;
                }
                let point = event.client_coordinates();
                let px = (point.x - bounds.left) / bounds.width;
                let py = (point.y - bounds.top) / bounds.height;
                let _ = rotate_y.animate_to(((px - 0.5) * max) as f32, AnimationConfig::spring(SPRING_MOUSE));
                let _ = rotate_x.animate_to(((0.5 - py) * max) as f32, AnimationConfig::spring(SPRING_MOUSE));
                glare_x.set(px * 100.0);
                glare_y.set(py * 100.0);
            },
            onmouseleave: move |_| {
                let _ = rotate_x.animate_to(0.0, AnimationConfig::spring(SPRING_MOUSE));
                let _ = rotate_y.animate_to(0.0, AnimationConfig::spring(SPRING_MOUSE));
            },
            {children}
            if glare && enabled {
                div {
                    aria_hidden: "true",
                    class: "pointer-events-none absolute inset-0 opacity-15",
                    style: "background:radial-gradient(circle at {glare_x()}% {glare_y()}%, var(--foreground), transparent 50%)",
                }
            }
        }
    }
}
