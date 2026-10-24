//! `motion/expanding-arrow-button.tsx` — a CTA whose accent capsule opens into
//! a row of dotted chevrons on hover or focus.
//!
//! The accent's width animates between `52px` and `calc(100% - 12px)` — a
//! spring drives a `0→1` progress and the width is
//! `calc(52px + (100% - 64px) · p)` so the same curve covers both endpoints.
//! The label fades out as the chevron row fades in with a per-chevron stagger.
//! Skipped on touch (phantom hover) and under reduced motion.

use crate::prelude::*;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

/// `ARROW_OPACITY`.
const ARROW_OPACITY: [f64; 5] = [1.0, 0.78, 0.54, 0.32, 0.16];

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// The dotted chevron glyph (five graded dots).
#[component]
fn DottedChevron(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            view_box: "0 0 20 28",
            fill: "none",
            "aria-hidden": "true",
            class,
            circle { cx: "4", cy: "4", r: "2", fill: "currentColor" }
            circle { cx: "10", cy: "9", r: "2", fill: "currentColor" }
            circle { cx: "16", cy: "14", r: "2", fill: "currentColor" }
            circle { cx: "10", cy: "19", r: "2", fill: "currentColor" }
            circle { cx: "4", cy: "24", r: "2", fill: "currentColor" }
        }
    }
}

/// Props for [`ExpandingArrowButton`].
#[derive(Props, Clone, PartialEq)]
pub struct ExpandingArrowButtonProps {
    /// The label shown while collapsed.
    pub children: Element,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub accent_class: Option<String>,
    #[props(default)]
    pub label_class: Option<String>,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub on_click: Option<EventHandler<()>>,
}

/// A CTA that opens its accent capsule on hover or focus.
#[component]
pub fn ExpandingArrowButton(props: ExpandingArrowButtonProps) -> Element {
    let ExpandingArrowButtonProps {
        children,
        class,
        accent_class,
        label_class,
        disabled,
        on_click,
    } = props;

    let reduce = use_reduced_motion();
    let can_hover = use_hover_capable();
    let mut hovered = use_signal(|| false);
    let mut focused = use_signal(|| false);
    let active = !disabled && ((can_hover() && hovered()) || focused());
    let reduce_value = reduce();

    let mut accent = use_motion(if active { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    let mut collapsed = use_motion(if active { 0.0f32 } else { 1.0 }).expect("0/1 are finite");
    let mut label = use_motion(if active { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");

    use_effect(use_reactive((&active,), move |(active,)| {
        let target = if active { 1.0 } else { 0.0 };
        let instant = || beui_core::ease::tween_ease_out(0);
        let _ = accent.animate_to(
            target,
            if reduce_value {
                instant()
            } else {
                AnimationConfig::spring(SPRING_LAYOUT)
            },
        );
        let _ = collapsed.animate_to(
            if active { 0.0 } else { 1.0 },
            if reduce_value {
                instant()
            } else {
                beui_core::ease::tween_ease_out(100)
            },
        );
        let _ = label.animate_to(
            target,
            if reduce_value {
                instant()
            } else {
                beui_core::ease::tween_ease_out(120)
            },
        );
    }));

    let accent_p = f64::from(current(accent).clamp(0.0, 1.0));
    let collapsed_opacity = f64::from(current(collapsed).clamp(0.0, 1.0));
    let label_p = f64::from(current(label).clamp(0.0, 1.0));
    let scale_value = f64::from(current(scale));

    let accent_style = format!("width:calc(52px + (100% - 64px) * {accent_p});border-radius:16px");
    let label_style = format!(
        "opacity:{label_p};transform:translateX({}px)",
        if active && !reduce_value {
            6.0 * label_p
        } else {
            0.0
        }
    );

    let button_class = cn!(
        "relative inline-flex h-16 min-w-72 items-center overflow-hidden rounded-[22px] bg-neutral-950 p-1.5 text-white select-none",
        "outline-none focus-visible:ring-2 focus-visible:ring-lime-300 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:pointer-events-none disabled:opacity-50",
        class,
    );
    let accent_class = cn!(
        "absolute inset-y-1.5 left-1.5 z-10 overflow-hidden bg-lime-300 text-neutral-950",
        accent_class,
    );
    let label_class = cn!(
        "relative z-0 ml-[76px] mr-5 whitespace-nowrap text-lg font-medium tracking-[-0.02em]",
        label_class,
    );

    rsx! {
        button {
            r#type: "button",
            disabled,
            class: button_class,
            style: "transform:scale({scale_value})",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            onfocus: move |_| focused.set(true),
            onblur: move |_| focused.set(false),
            onpointerdown: move |_| {
                if !reduce_value && !disabled {
                    let _ = scale.animate_to(0.97, AnimationConfig::spring(SPRING_PRESS));
                }
            },
            onpointerup: move |_| {
                let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
            },
            onclick: move |_| {
                if let Some(handler) = on_click {
                    handler.call(());
                }
            },
            span { aria_hidden: "true", class: accent_class, style: accent_style,
                span {
                    class: "absolute inset-0 grid place-items-center",
                    style: "opacity:{collapsed_opacity}",
                    DottedChevron { class: Some("h-7 w-5".to_string()) }
                }
                span { class: "absolute inset-0 flex items-center justify-around px-3",
                    for (index, opacity) in ARROW_OPACITY.iter().enumerate() {
                        ChevronCell {
                            key: "{index}",
                            index,
                            opacity: *opacity,
                            active,
                            reduce: reduce_value,
                        }
                    }
                }
            }
            span { class: label_class, style: label_style, {children} }
        }
    }
}

/// One chevron in the opened row (its own stagger).
#[component]
fn ChevronCell(index: usize, opacity: f64, active: bool, reduce: bool) -> Element {
    let mut progress = use_motion(if active { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    use_effect(use_reactive((&active,), move |(active,)| {
        let target = if active { 1.0 } else { 0.0 };
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            beui_core::ease::tween_ease_out(180).with_delay(dioxus_motion::Duration::from_millis(
                if active {
                    ((0.04 + index as f64 * 0.025) * 1000.0).round() as u64
                } else {
                    0
                },
            ))
        };
        let _ = progress.animate_to(target, config);
    }));
    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = format!(
        "opacity:{p};transform:translateX({}px);color:rgb(10 10 10 / {opacity})",
        -6.0 * (1.0 - p)
    );
    rsx! {
        span {
            aria_hidden: "true",
            class: "inline-grid place-items-center will-change-[opacity,transform]",
            style,
            DottedChevron { class: Some("h-7 w-5".to_string()) }
        }
    }
}
