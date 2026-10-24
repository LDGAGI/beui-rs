//! `motion/input.tsx` — the input field.
//!
//! Three animations, all small: the field shakes when an error appears, the
//! success check draws itself, and the error message fades, slides and unblurs
//! in and out (that last one is an `AnimatePresence`, so a cleared message is
//! kept until its exit finishes).

use dioxus::prelude::*;
use dioxus_motion::prelude::Duration;
use dioxus_motion::{AnimationManager, KeyframeAnimation};

use beui_core::cn;
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

use crate::motion::{animate_number, number, tween, use_number_motion, Ease, MotionStyle};

/// `error?: string | boolean`.
#[derive(Clone, PartialEq, Default)]
pub enum InputError {
    #[default]
    None,
    /// A bare truthy error: shake and red border, no message.
    Invalid,
    /// A message, which is also announced and rendered under the field.
    Message(String),
}

impl InputError {
    fn is_error(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn message(&self) -> Option<&str> {
        match self {
            Self::Message(message) => Some(message),
            _ => None,
        }
    }
}

/// The per-slot class overrides from `InputClassNames`.
#[derive(Clone, PartialEq, Default)]
pub struct InputClassNames {
    pub root: Option<String>,
    pub label: Option<String>,
    pub field: Option<String>,
    pub input: Option<String>,
    pub left_icon: Option<String>,
    pub right_icon: Option<String>,
    pub success_icon: Option<String>,
    pub error_message: Option<String>,
}

#[component]
pub fn Input(
    #[props(default)] label: Option<String>,
    #[props(default)] value: Option<String>,
    #[props(default)] default_value: Option<String>,
    #[props(default)] on_change: Option<EventHandler<String>>,
    #[props(default)] error: InputError,
    #[props(default)] reserve_error_line: bool,
    #[props(default)] success: bool,
    #[props(default)] left_icon: Option<Element>,
    #[props(default)] right_icon: Option<Element>,
    #[props(default)] class: Option<String>,
    #[props(default)] class_names: InputClassNames,
    #[props(default)] disabled: bool,
    #[props(default)] id: Option<String>,
    #[props(default)] r#type: Option<String>,
    #[props(default)] placeholder: Option<String>,
) -> Element {
    let auto_id = use_hook(|| format!("beui-input-{}", platform::next_scope_id()));
    let id = id.unwrap_or(auto_id);
    let reduce = use_reduced_motion();

    let controlled = value.is_some();
    let mut internal = use_signal(|| default_value.clone().unwrap_or_default());
    let current = match &value {
        Some(value) => value.clone(),
        None => internal(),
    };

    let mut focused = use_signal(|| false);
    let mut shake = use_number_motion(0.0);

    let has_error = error.is_error();
    let error_message = error.message().map(str::to_string);
    let right_slot = !success;

    // Shake the field when an error appears.
    use_effect(use_reactive!(|error| {
        if reduce() || !error.is_error() {
            return;
        }
        let frames = KeyframeAnimation::new(Duration::from_millis(450))
            .add_keyframe(0.0f32, 0.0, None)
            .expect("valid shake")
            .add_keyframe(-6.0, 1.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(6.0, 2.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(-4.0, 3.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(4.0, 4.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(-2.0, 5.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(0.0, 1.0, None)
            .expect("valid shake");
        let _ = shake.animate_keyframes(frames);
    }));

    let messages: Vec<String> = error_message.clone().into_iter().collect();
    let described_by = error_message.as_ref().map(|_| format!("{id}-error"));
    let has_left_icon = left_icon.is_some();

    rsx! {
        div { class: cn!("flex flex-col gap-1.5", class, class_names.root.clone()),
            if let Some(label) = label {
                label {
                    r#for: id.clone(),
                    class: cn!("px-1 text-sm font-medium text-foreground", class_names.label.clone()),
                    "{label}"
                }
            }
            div {
                "data-state": if has_error { "error" } else if success { "success" } else if focused() { "focused" } else { "idle" },
                style: "transform:translateX({number(&shake)}px)",
                class: cn!(
                    "relative h-11 overflow-hidden rounded-full border transition-colors duration-200",
                    "border-border",
                    if focused() && !has_error { "border-foreground/40 ring-2 ring-ring/40" } else { "" },
                    if has_error { "border-destructive ring-2 ring-destructive/25" } else { "" },
                    if disabled { "opacity-60" } else { "" },
                    class_names.field.clone(),
                ),
                if let Some(left_icon) = left_icon {
                    span {
                        class: cn!(
                            "pointer-events-none absolute left-3 top-1/2 flex -translate-y-1/2 items-center text-muted-foreground [&_svg]:h-4 [&_svg]:w-4",
                            class_names.left_icon.clone(),
                        ),
                        {left_icon}
                    }
                }
                input {
                    id: id.clone(),
                    r#type: r#type.unwrap_or_else(|| "text".to_string()),
                    value: current.clone(),
                    disabled,
                    placeholder,
                    "aria-invalid": has_error.then_some("true"),
                    "aria-describedby": described_by,
                    class: cn!(
                        "peer h-full w-full bg-transparent text-base leading-6 text-foreground caret-foreground outline-none",
                        "placeholder:text-muted-foreground/60",
                        if has_left_icon { "pl-10" } else { "pl-3.5" },
                        if right_slot || success { "pr-10" } else { "pr-3.5" },
                        if disabled { "cursor-not-allowed" } else { "" },
                        class_names.input.clone(),
                    ),
                    oninput: move |event| {
                        let next = event.value();
                        if !controlled {
                            internal.set(next.clone());
                        }
                        if let Some(on_change) = on_change {
                            on_change.call(next);
                        }
                    },
                    onfocus: move |_| focused.set(true),
                    onblur: move |_| focused.set(false),
                }
                if success {
                    InputSuccess { class_names: class_names.success_icon.clone() }
                } else if right_slot {
                    if let Some(right_icon) = right_icon {
                        span {
                            class: cn!(
                                "absolute right-0 top-0 flex h-full items-center text-muted-foreground [&_button]:grid [&_button]:size-11 [&_button]:place-items-center [&_svg]:h-4 [&_svg]:w-4",
                                class_names.right_icon.clone(),
                            ),
                            {right_icon}
                        }
                    }
                }
            }
            div { class: if reserve_error_line { "min-h-4" } else { "contents" },
                AnimatePresence { initial: false,
                    for message in messages {
                        InputErrorMessage {
                            key: "{message}",
                            id: format!("{id}-error"),
                            message,
                            class: class_names.error_message.clone(),
                        }
                    }
                }
            }
        }
    }
}

/// The success check, drawn with a `pathLength` sweep.
#[component]
fn InputSuccess(class_names: Option<String>) -> Element {
    let reduce = use_reduced_motion();
    let mut draw = use_number_motion(if reduce() { 1.0 } else { 0.0 });

    use_effect(move || {
        if !reduce() {
            animate_number(&mut draw, 1.0, tween(350, Ease::Out));
        }
    });

    let offset = 1.0 - number(&draw);

    rsx! {
        svg {
            view_box: "0 0 24 24",
            fill: "none",
            class: cn!(
                "absolute right-3.5 top-1/2 h-5 w-5 -translate-y-1/2 text-(--color-success)",
                class_names,
            ),
            path {
                d: "M5 12.5l4.5 4.5L19 7.5",
                stroke: "currentColor",
                stroke_width: "2.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                path_length: "1",
                style: "stroke-dasharray:1;stroke-dashoffset:{offset}",
            }
        }
    }
}

fn message_style(opacity: f32, y: f32, blur: Option<f32>) -> MotionStyle {
    let mut style = MotionStyle {
        opacity,
        y,
        ..Default::default()
    };
    if let Some(px) = blur {
        style.add_css_property("filter", format!("blur({px}px)"));
    }
    style
}

fn message_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        );
    }
    PresenceConfig::with_transitions(
        message_style(0.0, -4.0, Some(4.0)),
        message_style(1.0, 0.0, Some(0.0)),
        message_style(0.0, -4.0, Some(4.0)),
        tween(200, Ease::Out),
        tween(200, Ease::Out),
    )
}

#[component]
fn InputErrorMessage(id: String, message: String, class: Option<String>) -> Element {
    let reduce = use_reduced_motion();
    let gate = use_gate();
    let style = use_presence_style(message_config(reduce())).expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        p {
            id,
            role: "alert",
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: cn!("px-1 text-xs text-destructive", class),
            "{message}"
        }
    }
}
