//! `motion/otp-input.tsx` — the one-time-code field.
//!
//! The digits live in a fixed-length slot array (so clearing a middle slot
//! leaves a hole rather than collapsing the rest), and the hidden input owns the
//! keyboard, paste and autofill while the slots stay presentational. The motion
//! is the per-digit roll, the blinking caret, the success check that draws
//! itself, and the error shake.
//!
//! Differences from the source: there is no separate paste handler (a paste
//! arrives through the input's `input` event and fills from the first slot
//! rather than the active one), and a click on an earlier filled slot clamps to
//! the first empty slot instead of selecting the one under the pointer.

use dioxus::prelude::*;
use dioxus_motion::prelude::Duration;
use dioxus_motion::{AnimationManager, KeyframeAnimation};
use keyboard_types::{Key, Modifiers};

use beui_core::cn;
use beui_core::ease::SPRING_PRESS;
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

use crate::motion::{
    animate_number, number, spring, track, tween, use_loop_phase, use_number_motion, Ease,
    MotionStyle,
};

/// `OTPStatus`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum OTPStatus {
    #[default]
    Idle,
    Error,
    Success,
}

/// The check has one identity; the key must be a formatted string in `rsx!`.
const CHECK_KEY: &str = "check";

fn sanitize(raw: &str, length: usize) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_digit())
        .take(length)
        .collect()
}

fn to_slots(raw: &str, length: usize) -> Vec<String> {
    let digits: Vec<char> = sanitize(raw, length).chars().collect();
    (0..length)
        .map(|i| digits.get(i).map(|c| c.to_string()).unwrap_or_default())
        .collect()
}

/// The single insertion path: one digit overwrites the active slot and advances;
/// a multi-digit chunk fills forward from `from`.
fn insert_slots(
    slots: Signal<Vec<String>>,
    active: Signal<usize>,
    raw: &str,
    from: usize,
    length: usize,
    on_change: Option<EventHandler<String>>,
    on_complete: Option<EventHandler<String>>,
) {
    let digits = sanitize(raw, length);
    if digits.is_empty() {
        return;
    }
    let mut next = slots.peek().clone();
    let mut i = from;
    for ch in digits.chars() {
        if i >= length {
            break;
        }
        next[i] = ch.to_string();
        i += 1;
    }
    commit_slots(slots, next, on_change, on_complete);
    let mut active = active;
    active.set(i.min(length - 1));
}

/// Commit a slot array: report the string, and complete only on empty → full.
fn commit_slots(
    slots: Signal<Vec<String>>,
    next: Vec<String>,
    on_change: Option<EventHandler<String>>,
    on_complete: Option<EventHandler<String>>,
) {
    let mut slots = slots;
    let was_complete = slots.peek().iter().all(|c| !c.is_empty());
    let now_complete = next.iter().all(|c| !c.is_empty());
    slots.set(next.clone());
    let string = next.join("");
    if let Some(on_change) = on_change {
        on_change.call(string.clone());
    }
    if !was_complete && now_complete {
        if let Some(on_complete) = on_complete {
            on_complete.call(string);
        }
    }
}

#[component]
pub fn OTPInput(
    #[props(default = 6)] length: usize,
    #[props(default)] value: Option<String>,
    #[props(default)] default_value: String,
    #[props(default)] on_change: Option<EventHandler<String>>,
    #[props(default)] on_complete: Option<EventHandler<String>>,
    #[props(default)] label: Option<String>,
    #[props(default)] hint: Option<String>,
    #[props(default)] success_message: Option<String>,
    #[props(default)] error_message: Option<String>,
    #[props(default)] status: OTPStatus,
    #[props(default)] mask: bool,
    #[props(default)] disabled: bool,
    #[props(default)] auto_focus: bool,
    #[props(default = "One-time passcode".to_string())] aria_label: String,
    #[props(default)] class: Option<String>,
) -> Element {
    let uid = use_hook(|| format!("beui-otp-{}", platform::next_scope_id()));
    let reduce = use_reduced_motion()();
    let length = length.max(1);

    let initial = value.clone().unwrap_or(default_value);
    let slots = use_signal(|| to_slots(&initial, length));
    let mut focused = use_signal(|| false);
    let mut active = use_signal(|| 0usize);

    // Pull in external value changes, but never collapse a hole the user made.
    use_effect(use_reactive!(|value| {
        if let Some(value) = value {
            let incoming = sanitize(&value, length);
            let mut slots = slots;
            if slots.peek().join("") != incoming {
                slots.set(to_slots(&incoming, length));
            }
        }
    }));

    // Error shake, replayed on every transition into "error".
    let mut shake = use_number_motion(0.0);
    use_effect(use_reactive!(|status| {
        if status != OTPStatus::Error || reduce {
            return;
        }
        let frames = KeyframeAnimation::new(Duration::from_millis(450))
            .add_keyframe(0.0f32, 0.0, None)
            .expect("valid shake")
            .add_keyframe(-5.0, 1.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(5.0, 2.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(-3.0, 3.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(3.0, 4.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(-1.0, 5.0 / 6.0, None)
            .expect("valid shake")
            .add_keyframe(0.0, 1.0, None)
            .expect("valid shake");
        let _ = shake.animate_keyframes(frames);
    }));

    let caret_phase = use_loop_phase(1000);
    let caret_opacity = if reduce {
        1.0
    } else {
        track(&[1.0, 1.0, 0.0, 0.0], number(&caret_phase), |t| t)
    };

    let show_success = status == OTPStatus::Success;
    let active_index = if focused() { active() as i64 } else { -1 };
    let checks: Vec<()> = if show_success { vec![()] } else { vec![] };
    let field_id = format!("{uid}-input");
    let field_selector = format!("#{field_id}");
    let current = slots();

    rsx! {
        div { class: cn!("inline-flex flex-col gap-2", class),
            if let Some(label) = label {
                label { r#for: field_id.clone(), class: "text-sm font-medium text-foreground", "{label}" }
            }
            fieldset {
                class: "relative m-0 inline-flex w-max border-0 p-0",
                onmousedown: move |event| {
                    if disabled {
                        return;
                    }
                    event.prevent_default();
                    let first_empty = slots.peek().iter().position(|c| c.is_empty());
                    let cap = first_empty.unwrap_or(length - 1);
                    let _ = event.client_coordinates();
                    active.set(cap);
                    platform::focus_selector(&field_selector, true);
                },
                input {
                    id: "{field_id}",
                    inputmode: "numeric",
                    autocomplete: "one-time-code",
                    autofocus: auto_focus,
                    disabled,
                    "aria-label": aria_label,
                    "aria-invalid": (status == OTPStatus::Error).then_some("true"),
                    value: "",
                    maxlength: "{length}",
                    class: "absolute inset-0 z-20 h-full w-full cursor-text bg-transparent text-transparent caret-transparent opacity-0 outline-none disabled:cursor-not-allowed",
                    onkeydown: move |event| {
                        if disabled
                            || event.modifiers().contains(Modifiers::CONTROL)
                            || event.modifiers().contains(Modifiers::META)
                            || event.modifiers().contains(Modifiers::ALT)
                        {
                            return;
                        }
                        match event.key() {
                            Key::Character(text)
                                if text.len() == 1 && text.chars().all(|c| c.is_ascii_digit()) =>
                            {
                                event.prevent_default();
                                insert_slots(slots, active, &text, active(), length, on_change, on_complete);
                            }
                            Key::Backspace => {
                                event.prevent_default();
                                let index = active();
                                let mut next = slots.peek().clone();
                                if !next[index].is_empty() {
                                    next[index] = String::new();
                                } else if index > 0 {
                                    next[index - 1] = String::new();
                                    active.set(index - 1);
                                }
                                commit_slots(slots, next, on_change, on_complete);
                            }
                            Key::Delete => {
                                event.prevent_default();
                                let mut next = slots.peek().clone();
                                next[active()] = String::new();
                                commit_slots(slots, next, on_change, on_complete);
                            }
                            Key::ArrowLeft => {
                                event.prevent_default();
                                active.set(active().saturating_sub(1));
                            }
                            Key::ArrowRight => {
                                event.prevent_default();
                                active.set((active() + 1).min(length - 1));
                            }
                            Key::Home => {
                                event.prevent_default();
                                active.set(0);
                            }
                            Key::End => {
                                event.prevent_default();
                                active.set(length - 1);
                            }
                            _ => {}
                        }
                    },
                    oninput: move |event| {
                        // Autofill (SMS one-time-code) and paste arrive here; a
                        // keystroke never does, because keydown cancels it.
                        let digits = sanitize(&event.value(), length);
                        if digits.is_empty() {
                            return;
                        }
                        commit_slots(slots, to_slots(&digits, length), on_change, on_complete);
                        let mut active = active;
                        active.set(digits.len().min(length - 1));
                    },
                    onfocus: move |_| focused.set(true),
                    onblur: move |_| focused.set(false),
                }
                div {
                    class: "flex items-center gap-2",
                    style: "transform:translateX({number(&shake)}px)",
                    for i in 0..length {
                        OtpSlot {
                            key: "{uid}-{i}",
                            char: current[i].clone(),
                            active: active_index == i as i64,
                            filled: !current[i].is_empty(),
                            caret_opacity,
                            status,
                            mask,
                            reduce,
                        }
                    }
                }
                AnimatePresence {
                    for _ in checks {
                        OtpCheck { key: "{CHECK_KEY}" }
                    }
                }
            }
            {
                let message = if show_success {
                    success_message.clone()
                } else if status == OTPStatus::Error {
                    error_message.clone()
                } else {
                    hint.clone()
                };
                match message {
                    Some(message) => rsx! {
                        p {
                            "aria-live": "polite",
                            class: cn!(
                                "text-sm",
                                if show_success { "text-emerald-500" } else if status == OTPStatus::Error { "text-destructive" } else { "text-muted-foreground" },
                            ),
                            "{message}"
                        }
                    },
                    None => rsx! {},
                }
            }
        }
    }
}

#[component]
fn OtpSlot(
    char: String,
    active: bool,
    filled: bool,
    caret_opacity: f32,
    status: OTPStatus,
    mask: bool,
    reduce: bool,
) -> Element {
    let show_success = status == OTPStatus::Success;
    let digits: Vec<String> = if char.is_empty() {
        Vec::new()
    } else {
        vec![char.clone()]
    };

    rsx! {
        div {
            "data-active": active.then_some("true"),
            "data-filled": filled.then_some("true"),
            class: cn!(
                "relative grid h-14 w-12 place-items-center overflow-hidden rounded-xl border text-xl font-semibold tabular-nums transition-colors duration-200",
                if show_success {
                    "border-emerald-500/60 text-foreground"
                } else if status == OTPStatus::Error {
                    "border-destructive/60 text-foreground"
                } else if filled {
                    "border-border-strong text-foreground"
                } else {
                    "border-border text-muted-foreground"
                },
                if active && !show_success && status != OTPStatus::Error { "border-foreground" } else { "" },
            ),
            if active && !show_success {
                span {
                    "aria-hidden": "true",
                    class: cn!(
                        "pointer-events-none absolute top-1/2 h-6 w-px -translate-y-1/2 bg-foreground",
                        if filled { "right-3" } else { "left-1/2 -translate-x-1/2" },
                    ),
                    style: "opacity:{caret_opacity}",
                }
            }
            AnimatePresence { initial: false,
                for digit in digits {
                    OtpDigit { key: "{digit}", digit, mask, reduce }
                }
            }
        }
    }
}

fn digit_style(opacity: f32, y: f32, blur: f32) -> MotionStyle {
    let mut style = MotionStyle {
        opacity,
        y,
        ..Default::default()
    };
    style.add_css_property("filter", format!("blur({blur}px)"));
    style
}

#[component]
fn OtpDigit(digit: String, mask: bool, reduce: bool) -> Element {
    let gate = use_gate();
    let config = if reduce {
        PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        )
    } else {
        PresenceConfig::with_transitions(
            digit_style(0.0, 14.0, 4.0),
            digit_style(1.0, 0.0, 0.0),
            digit_style(0.0, -14.0, 4.0),
            tween(220, Ease::Out),
            tween(220, Ease::Out),
        )
    };
    let style = use_presence_style(config).expect("static presence config");
    let css = presence_css(&style);
    let text = if mask { "•".to_string() } else { digit };

    rsx! {
        span {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "absolute inset-0 grid place-items-center leading-none",
            "{text}"
        }
    }
}

#[component]
fn OtpCheck() -> Element {
    let reduce = use_reduced_motion()();
    let gate = use_gate();
    let config = PresenceConfig::with_transitions(
        motion_style! { scale: 0.6, opacity: 0.0 },
        motion_style! { scale: 1.0, opacity: 1.0 },
        motion_style! { scale: 0.6, opacity: 0.0 },
        spring(SPRING_PRESS),
        spring(SPRING_PRESS),
    );
    let style = use_presence_style(config).expect("static presence config");
    let css = presence_css(&style);

    let mut draw = use_number_motion(if reduce { 1.0 } else { 0.0 });
    use_effect(move || {
        if !reduce {
            let config = tween(350, Ease::Out).with_delay(Duration::from_millis(100));
            animate_number(&mut draw, 1.0, config);
        }
    });
    let offset = 1.0 - number(&draw);

    rsx! {
        span {
            "aria-hidden": "true",
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "pointer-events-none absolute -right-7 top-1/2 -translate-y-1/2 text-emerald-500",
            svg {
                width: "20",
                height: "20",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "3",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                title { "Verified" }
                path {
                    d: "M5 13l4 4L19 7",
                    path_length: "1",
                    style: "stroke-dasharray:1;stroke-dashoffset:{offset}",
                }
            }
        }
    }
}
