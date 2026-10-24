//! The rest of the button family: `magnetic.tsx`, `button/magnetic.tsx`,
//! `button/metallic.tsx` and `button/stateful.tsx`.
//!
//! - [`Magnetic`] pulls a child toward the cursor on `SPRING_MOUSE`; it is
//!   skipped on touch (phantom hover) and under reduced motion.
//! - [`MetallicButton`] is a chrome rim that drifts on an 8 s loop with a
//!   reflection that crosses on hover.
//! - [`StatefulButton`] swaps its icon and label per state through presence
//!   boundaries.

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::ease::{ease_in_out, SPRING_MOUSE, SPRING_SWAP};
use beui_core::hooks::{use_hover_capable, use_reduced_motion};
use beui_core::platform::Rect;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

use crate::button::{Button, ButtonSize, ButtonVariant};
use crate::icons::{IconCheck, IconLoader2, IconX};
use crate::motion::{
    animate_style, css, number, spring, track, tween, use_loop_phase, use_style_motion, Ease,
    MotionStyle,
};

// --- magnetic ---------------------------------------------------------------

/// Cursor-follow wrapper: the child is pulled toward the pointer by `strength`
/// of its own half-size, on `SPRING_MOUSE`.
#[component]
pub fn Magnetic(
    #[props(default = 0.35)] strength: f64,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let can_hover = use_hover_capable()();
    let enabled = !reduce && can_hover;
    let mut offset = use_style_motion(motion_style! { x: 0.0, y: 0.0 });
    let mut rect = use_signal(|| None::<Rect>);

    rsx! {
        div {
            class: cn!("inline-block", class),
            style: css(&offset),
            onmounted: move |data| {
                spawn(async move {
                    if let Some(measured) = beui_core::platform::client_rect(&data).await {
                        rect.set(Some(measured));
                    }
                });
            },
            onmousemove: move |event| {
                if !enabled {
                    return;
                }
                let Some(measured) = *rect.peek() else {
                    return;
                };
                let point = event.client_coordinates();
                let x = (point.x - measured.left - measured.width / 2.0) * strength;
                let y = (point.y - measured.top - measured.height / 2.0) * strength;
                animate_style(&mut offset, motion_style! { x: x, y: y }, spring(SPRING_MOUSE));
            },
            onmouseleave: move |_| {
                animate_style(&mut offset, motion_style! { x: 0.0, y: 0.0 }, spring(SPRING_MOUSE));
            },
            {children}
        }
    }
}

/// A [`Button`] inside a [`Magnetic`].
#[component]
pub fn MagneticButton(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default = 0.25)] strength: f64,
    #[props(default)] magnetic_class: Option<String>,
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    rsx! {
        Magnetic { strength, class: magnetic_class,
            Button { variant, size, class, disabled, aria_label, onclick, {children} }
        }
    }
}

// --- metallic ---------------------------------------------------------------

const RIM_GRADIENT: &str =
    "linear-gradient(105deg, #111 0%, #737373 14%, #fafafa 26%, #525252 38%, #0a0a0a 50%, #a3a3a3 64%, #fff 75%, #404040 87%, #111 100%)";
const SHIMMER_GRADIENT: &str =
    "linear-gradient(90deg, transparent, rgba(255,255,255,0.5) 48%, transparent)";

/// Chrome button: a metallic rim that drifts on an 8 s loop, a reflection that
/// crosses the face on hover, and the button's own press spring underneath.
#[component]
pub fn MetallicButton(
    #[props(default)] size: ButtonSize,
    #[props(default)] paused: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let still = paused || reduce;
    let mut hovered = use_signal(|| false);

    // Rim drift: x = 0% → 13% → 0% over 8 s, `EASE_IN_OUT`.
    let phase = use_loop_phase(8000);
    let rim_x = if still {
        0.0
    } else {
        track(&[0.0, 13.0, 0.0], number(&phase), ease_in_out)
    };
    let mut shimmer = use_style_motion(motion_style! { x: 0.0 });
    let hovered_now = hovered();
    use_effect(use_reactive!(|hovered_now| {
        if !still {
            let target = if hovered_now { 310.0 } else { 0.0 };
            animate_style(
                &mut shimmer,
                MotionStyle::default().x(target),
                tween(2400, Ease::InOut),
            );
        }
    }));

    let shimmer_css = css(&shimmer);

    rsx! {
        Button {
            variant: ButtonVariant::Ghost,
            size,
            class: cn!(
                "group relative isolate overflow-hidden border-0 bg-transparent text-foreground",
                "hover:bg-transparent hover:text-foreground",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                "shadow-[0_8px_22px_rgba(0,0,0,0.16)]",
                if size == ButtonSize::Icon { "rounded-full" } else { "" },
                class,
            ),
            disabled,
            aria_label,
            onclick,
            on_hover_start: move |_| hovered.set(true),
            on_hover_end: move |_| hovered.set(false),
            span {
                "aria-hidden": "true",
                class: "pointer-events-none absolute inset-y-0 left-[-18%] z-0 w-[136%] rounded-[inherit]",
                style: "background-image:{RIM_GRADIENT};transform:translateX({rim_x}%)",
            }
            span {
                "aria-hidden": "true",
                class: "pointer-events-none absolute inset-y-0 left-[-58%] z-[1] w-[52%] -skew-x-12 opacity-50 blur-[3px] mix-blend-screen",
                style: "background-image:{SHIMMER_GRADIENT};{shimmer_css}",
            }
            span { "aria-hidden": "true", class: "pointer-events-none absolute inset-[2px] z-[2] rounded-[inherit] bg-background transition-colors group-hover:bg-muted/40" }
            span { "aria-hidden": "true", class: "pointer-events-none absolute inset-[2px] z-[3] rounded-[inherit] shadow-[inset_0_1px_0_rgba(255,255,255,0.28),inset_0_-1px_0_rgba(0,0,0,0.16)]" }
            span { class: "relative z-10 inline-flex items-center justify-center gap-2", {children} }
        }
    }
}

// --- stateful ---------------------------------------------------------------

/// `ButtonState`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonState {
    #[default]
    Idle,
    Loading,
    Success,
    Error,
}

impl ButtonState {
    fn key(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Loading => "loading",
            Self::Success => "success",
            Self::Error => "error",
        }
    }
}

fn icon_style(opacity: f32, width: f32, scale: f32, blur: f32) -> MotionStyle {
    let mut style = MotionStyle {
        opacity,
        scale,
        ..Default::default()
    };
    style.add_css_property("width", width);
    style.add_css_property("filter", format!("blur({blur}px)"));
    style
}

fn icon_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(150, Ease::Out),
            tween(150, Ease::Out),
        );
    }
    PresenceConfig::with_transitions(
        icon_style(0.0, 0.0, 0.7, 6.0),
        icon_style(1.0, 24.0, 1.0, 0.0),
        icon_style(0.0, 0.0, 0.7, 6.0),
        spring(SPRING_SWAP),
        tween(160, Ease::Out),
    )
}

fn text_style(opacity: f32, y: f32, blur: f32) -> MotionStyle {
    let mut style = MotionStyle {
        opacity,
        y,
        ..Default::default()
    };
    style.add_css_property("filter", format!("blur({blur}px)"));
    style
}

fn text_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(150, Ease::Out),
            tween(150, Ease::Out),
        );
    }
    PresenceConfig::with_transitions(
        text_style(0.0, 14.0, 6.0),
        text_style(1.0, 0.0, 0.0),
        text_style(0.0, -14.0, 6.0),
        spring(SPRING_SWAP),
        tween(160, Ease::Out),
    )
}

/// A label slot: either a state's string or the caller's children.
#[derive(Clone, PartialEq)]
enum LabelSlot {
    Text(String),
    Idle,
}

impl LabelSlot {
    fn key(&self) -> String {
        match self {
            Self::Text(key) => key.clone(),
            Self::Idle => "idle-label".to_string(),
        }
    }
}

/// A button whose icon and label roll when its state changes.
///
/// Difference from the source: the label rolls as one line (fade + slide +
/// blur) rather than per-letter with a stagger, and the label slot's width does
/// not animate between strings.
#[component]
pub fn StatefulButton(
    #[props(default)] state: ButtonState,
    #[props(default = "Loading".to_string())] loading_text: String,
    #[props(default = "Done".to_string())] success_text: String,
    #[props(default = "Try again".to_string())] error_text: String,
    #[props(default)] icon: Option<Element>,
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let is_busy = state == ButtonState::Loading;
    let text = match state {
        ButtonState::Loading => loading_text,
        ButtonState::Success => success_text,
        ButtonState::Error => error_text,
        ButtonState::Idle => String::new(),
    };

    // One keyed child per state, so a state change rolls the icon.
    let icon_key = match state {
        ButtonState::Idle => icon.as_ref().map(|_| "idle-icon".to_string()),
        _ => Some(format!("{}-icon", state.key())),
    };
    let icons: Vec<String> = icon_key.into_iter().collect();
    let labels: Vec<LabelSlot> = if state == ButtonState::Idle {
        vec![LabelSlot::Idle]
    } else {
        vec![LabelSlot::Text(format!("{}-{}", state.key(), text))]
    };

    rsx! {
        Button {
            variant,
            size,
            class,
            disabled: disabled || is_busy,
            onclick,
            span {
                "aria-live": "polite",
                class: "relative inline-flex items-center justify-center overflow-hidden",
                AnimatePresence { initial: false,
                    for key in icons {
                        StatefulIconSlot { key: "{key}", state, icon: icon.clone() }
                    }
                }
                AnimatePresence { initial: false,
                    for slot in labels {
                        StatefulLabelSlot { key: "{slot.key()}", slot, text: text.clone(), children: children.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn StatefulIconSlot(state: ButtonState, icon: Option<Element>) -> Element {
    let reduce = use_reduced_motion()();
    let gate = use_gate();
    let style = use_presence_style(icon_config(reduce)).expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        span {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "inline-grid shrink-0 place-items-center overflow-hidden",
            match state {
                ButtonState::Loading => rsx! {
                    span { class: "inline-flex animate-spin", IconLoader2 { class: "h-4 w-4" } }
                },
                ButtonState::Success => rsx! { IconCheck { class: "h-4 w-4" } },
                ButtonState::Error => rsx! { IconX { class: "h-4 w-4" } },
                ButtonState::Idle => rsx! { {icon} },
            }
        }
    }
}

#[component]
fn StatefulLabelSlot(slot: LabelSlot, text: String, children: Element) -> Element {
    let reduce = use_reduced_motion()();
    let gate = use_gate();
    let style = use_presence_style(text_config(reduce)).expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        span {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "inline-block whitespace-nowrap",
            match slot {
                LabelSlot::Text(_) => rsx! { "{text}" },
                LabelSlot::Idle => rsx! { {children} },
            }
        }
    }
}
