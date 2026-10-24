//! `motion/hold-action-button.tsx` — hold the button to complete, with a liquid
//! fill rising behind the label.
//!
//! The fill and the looping wave are pure CSS: the fill is a `transform`
//! transition whose timing flips between `linear` over the hold duration and an
//! `EASE_OUT` return, and the wave is a 1.1s linear keyframe loop switched on
//! while active. That is what the source's `transition={{duration:
//! holdDuration/1000, ease: "linear"}}` and `repeat: Infinity` describe.
//!
//! The pointer hold uses `beui_core::platform`'s pointer capture. A captured
//! pointer gets no boundary events, so leaving the button is measured in
//! `pointermove`; a touch gets no leave reading at all, which is why the
//! pointer-type check is there.

use crate::prelude::*;
use crate::rect::{use_element_rect, ElementRect};
use beui_core::platform::{capture_pointer, release_pointer};
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::PointerInteraction;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, Time, TimeProvider};

/// `@keyframes` for the wave, travelling in the fill's direction.
const WAVE_KEYFRAMES: &str = "@keyframes beui-hold-wave-y{to{transform:translateY(-50%)}}\
     @keyframes beui-hold-wave-x{to{transform:translateX(-50%)}}";

/// The hold direction (which way the fill travels).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HoldActionType {
    #[default]
    Vertical,
    Horizontal,
}

/// Props for [`HoldActionButton`].
#[derive(Props, Clone, PartialEq)]
pub struct HoldActionButtonProps {
    pub children: Element,
    #[props(default)]
    pub r#type: HoldActionType,
    #[props(default = "Keep holding".to_string())]
    pub holding_label: String,
    #[props(default = "Done".to_string())]
    pub complete_label: String,
    /// Hold duration in milliseconds. Defaults to `1600`.
    #[props(default = 1600.0)]
    pub hold_duration: f64,
    #[props(default)]
    pub on_hold_complete: Option<EventHandler<()>>,
    #[props(default)]
    pub fill_class: Option<String>,
    #[props(default)]
    pub label_class: Option<String>,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub aria_label: Option<String>,
}

/// A hold-to-confirm button.
#[component]
pub fn HoldActionButton(props: HoldActionButtonProps) -> Element {
    let HoldActionButtonProps {
        children,
        r#type,
        holding_label,
        complete_label,
        hold_duration,
        on_hold_complete,
        fill_class,
        label_class,
        class,
        disabled,
        aria_label,
    } = props;

    let reduce = use_reduced_motion();
    let rect: ElementRect = use_element_rect();
    use_effect(move || rect.refresh());

    let mut holding = use_signal(|| false);
    let mut completed = use_signal(|| false);
    let mut completed_flag = use_signal(|| false);
    let mut generation = use_signal(|| 0u32);
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");
    let reduce_value = reduce();

    let mut start_hold = move || {
        if disabled || holding() {
            return;
        }
        completed_flag.set(false);
        completed.set(false);
        holding.set(true);
        let run = generation() + 1;
        generation.set(run);
        let holding = holding;
        let mut completed = completed;
        let mut completed_flag = completed_flag;
        let generation = generation;
        spawn(async move {
            if Time::delay(dioxus_motion::Duration::from_millis(
                hold_duration.round() as u64
            ))
            .await
            .is_err()
            {
                return;
            }
            if generation() == run && holding() && !completed_flag() {
                completed_flag.set(true);
                completed.set(true);
                if let Some(handler) = on_hold_complete {
                    handler.call(());
                }
            }
        });
    };

    let mut cancel_hold = move || {
        holding.set(false);
        completed.set(false);
    };

    let active = holding() || completed();
    let (active_transform, idle_transform) = match r#type {
        HoldActionType::Horizontal => ("translateX(0%)", "translateX(-100%)"),
        HoldActionType::Vertical => ("translateY(0%)", "translateY(115%)"),
    };
    let fill_style = if reduce_value {
        format!(
            "opacity:{};transform:none;transition:opacity 150ms ease",
            if active { 1.0 } else { 0.0 }
        )
    } else {
        let easing = if active { "linear" } else { EASE_OUT_CSS };
        let duration = if active { hold_duration } else { 240.0 };
        format!(
            "opacity:1;transform:{};transition:transform {}ms {easing}",
            if active {
                active_transform
            } else {
                idle_transform
            },
            duration.round()
        )
    };
    let label_transition = if reduce_value { 0 } else { 120 };

    let button_class = cn!(
        "relative inline-grid h-16 min-w-72 touch-none place-items-center overflow-hidden rounded-[var(--hold-radius)] bg-primary px-8 text-primary-foreground",
        "[--hold-radius:22px]",
        TOUCH_GESTURE_CLASS,
        "outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:pointer-events-none disabled:opacity-50",
        class,
    );
    let fill_class = cn!(
        "absolute inset-0 bg-sky-400 will-change-[opacity,transform]",
        fill_class,
    );
    let label_class = cn!(
        "relative z-10 grid place-items-center text-base font-medium tracking-[-0.01em]",
        label_class,
    );
    let scale_value = f64::from(*scale.current().read());
    let label = aria_label.unwrap_or_else(|| "Hold to confirm".to_string());
    let base_style = format!(
        "opacity:{};transition:opacity {label_transition}ms {EASE_OUT_CSS}",
        if active { 0 } else { 1 }
    );
    let holding_style = format!(
        "opacity:{};transition:opacity {label_transition}ms {EASE_OUT_CSS}",
        if holding() && !completed() { 1 } else { 0 }
    );
    let complete_style = format!(
        "opacity:{};transition:opacity {label_transition}ms {EASE_OUT_CSS}",
        if completed() { 1 } else { 0 }
    );

    rsx! {
        style { dangerous_inner_html: WAVE_KEYFRAMES }
        button {
            r#type: "button",
            disabled,
            aria_label: label,
            "data-beui-rect": rect.attr(),
            onmounted: move |data| rect.measure(data.data()),
            class: button_class,
            style: "transform:scale({scale_value})",
            onpointerdown: move |event: Event<PointerData>| {
                if event.trigger_button() != Some(MouseButton::Primary) {
                    return;
                }
                if !reduce_value {
                    let _ = scale.animate_to(0.98, AnimationConfig::spring(SPRING_PRESS));
                }
                start_hold();
                let _ = capture_pointer(&event);
            },
            onpointermove: move |event: Event<PointerData>| {
                if !holding() {
                    return;
                }
                let Some(bounds) = rect.get() else {
                    return;
                };
                let point = event.client_coordinates();
                let outside = point.x < bounds.left
                    || point.x > bounds.left + bounds.width
                    || point.y < bounds.top
                    || point.y > bounds.top + bounds.height;
                if outside {
                    cancel_hold();
                }
            },
            onpointerup: move |event: Event<PointerData>| {
                let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
                let _ = release_pointer(&event);
                cancel_hold();
            },
            onpointercancel: move |_| cancel_hold(),
            onpointerleave: move |event: Event<PointerData>| {
                let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
                if event.pointer_type() != "touch" {
                    cancel_hold();
                }
            },
            oncontextmenu: move |event: Event<MouseData>| event.prevent_default(),
            onkeydown: move |event: Event<KeyboardData>| {
                let activate = event.key() == Key::Enter
                    || event.key() == Key::Character(" ".to_string());
                if activate && !event.is_auto_repeating() {
                    event.prevent_default();
                    start_hold();
                }
            },
            onkeyup: move |event: Event<KeyboardData>| {
                if event.key() == Key::Character(" ".to_string()) || event.key() == Key::Enter {
                    event.prevent_default();
                    cancel_hold();
                }
            },
            span {
                aria_hidden: "true",
                class: "pointer-events-none absolute inset-0 [clip-path:inset(0_round_var(--hold-radius))]",
                span { aria_hidden: "true", class: fill_class, style: fill_style,
                    if !reduce_value {
                        match r#type {
                            HoldActionType::Horizontal => rsx! {
                                svg {
                                    view_box: "0 0 240 24",
                                    preserve_aspect_ratio: "none",
                                    "aria-hidden": "true",
                                    class: "absolute -top-5 left-0 h-6 w-[200%] text-sky-400",
                                    style: if active {
                                        "animation:beui-hold-wave-x 1.1s linear infinite"
                                    } else {
                                        "animation:none"
                                    },
                                    path {
                                        d: "M0 12C20 2 40 2 60 12s40 10 60 0 40-10 60 0 40 10 60 0v12H0Z",
                                        fill: "currentColor",
                                    }
                                }
                            },
                            HoldActionType::Vertical => rsx! {
                                svg {
                                    view_box: "0 0 24 240",
                                    preserve_aspect_ratio: "none",
                                    "aria-hidden": "true",
                                    class: "absolute -right-5 top-0 h-[200%] w-6 text-sky-400",
                                    style: if active {
                                        "animation:beui-hold-wave-y 1.1s linear infinite"
                                    } else {
                                        "animation:none"
                                    },
                                    path {
                                        d: "M0 0h12C2 20 2 40 12 60s10 40 0 60-10 40 0 60 10 40 0 60H0Z",
                                        fill: "currentColor",
                                    }
                                }
                            },
                        }
                    }
                }
            }
            span { class: label_class,
                span { class: "col-start-1 row-start-1", style: base_style, {children} }
                span {
                    aria_hidden: if holding() && !completed() { "false" } else { "true" },
                    class: "col-start-1 row-start-1",
                    style: holding_style,
                    "{holding_label}"
                }
                span {
                    aria_hidden: if completed() { "false" } else { "true" },
                    class: "col-start-1 row-start-1",
                    style: complete_style,
                    "{complete_label}"
                }
            }
        }
    }
}
