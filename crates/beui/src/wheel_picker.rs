//! `motion/wheel-picker.tsx` — the iOS-style spinning wheel.
//!
//! The rows sit on a 3D drum: each is `rotateX(-itemAngle·i) translateZ(radius)`
//! and the drum rotates `rotateX(itemAngle·scroll)` inside a `perspective`
//! container. A second copy of the same drum is clipped to the centre row and
//! drawn crisp, so the selected row reads solid while its neighbours recede.
//!
//! Physics constants are copied verbatim (`DECELERATION`, `MAX_VELOCITY`,
//! `easeOutCubic`, `easeOutBack` with `BACK = 1.35`).
//!
//! Differences from the source: the tick sound is not ported (`tick-sound.ts`
//! is a WebAudio player outside this lane); the release velocity is averaged over
//! the last few samples with an assumed 16 ms frame instead of the source's
//! timestamped 90 ms window; and the wheel handler is a plain Dioxus `onwheel`
//! rather than a native non-passive listener, so a fine-grained wheel may scroll
//! the page behind the picker as well.

use dioxus::html::geometry::WheelDelta;
use dioxus::prelude::*;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

use beui_core::cn;
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;
use beui_core::touch::TOUCH_GESTURE_CLASS;

/// One wheel entry.
#[derive(Clone, PartialEq, Eq)]
pub struct WheelPickerOption {
    pub label: String,
    pub value: String,
}

impl WheelPickerOption {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }

    /// A string option whose label and value are the same.
    pub fn plain(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            label: value.clone(),
            value,
        }
    }
}

const DEG: f64 = std::f64::consts::PI / 180.0;
/// Rows per ms² of deceleration (lower = a freer flick).
const DECELERATION: f64 = 0.00042;
/// Rows per ms cap on a hard fling.
const MAX_VELOCITY: f64 = 0.18;
/// Rows per pixel of wheel delta.
const WHEEL_SENS: f64 = 0.012;
/// The little spring bounce as a row snaps home.
const BACK: f64 = 1.35;

fn clamp(value: f64, lo: f64, hi: f64) -> f64 {
    value.max(lo).min(hi)
}

fn ease_out_cubic(p: f64) -> f64 {
    1.0 - (1.0 - p).powi(3)
}

fn ease_out_back(p: f64) -> f64 {
    1.0 + (BACK + 1.0) * (p - 1.0).powi(3) + BACK * (p - 1.0).powi(2)
}

#[derive(Clone, PartialEq)]
struct DragState {
    y: f64,
    scroll: f64,
    pts: Vec<f64>,
}

fn begin_drag(
    scroll: Signal<f64>,
    grabbing: Signal<bool>,
    drag: Signal<Option<DragState>>,
    mut generation: Signal<u64>,
    y: f64,
) {
    generation += 1;
    let mut grabbing = grabbing;
    grabbing.set(true);
    let mut drag = drag;
    drag.set(Some(DragState {
        y,
        scroll: scroll(),
        pts: vec![y],
    }));
}

fn move_drag(
    scroll: Signal<f64>,
    drag: Signal<Option<DragState>>,
    item_height: f64,
    last: f64,
    y: f64,
    emit: EventHandler<f64>,
) {
    let Some(mut state) = drag() else {
        return;
    };
    state.pts.push(y);
    if state.pts.len() > 8 {
        state.pts.remove(0);
    }
    let mut next = state.scroll + (state.y - y) / item_height;
    if next < 0.0 {
        next *= 0.3;
    } else if next > last {
        next = last + (next - last) * 0.3;
    }
    let mut drag = drag;
    drag.set(Some(state));
    let mut scroll = scroll;
    scroll.set(next);
    emit.call(next);
}

fn end_drag(
    scroll: Signal<f64>,
    grabbing: Signal<bool>,
    drag: Signal<Option<DragState>>,
    generation: Signal<u64>,
    item_height: f64,
    last: f64,
) {
    let Some(state) = drag() else {
        return;
    };
    let mut drag = drag;
    drag.set(None);
    let mut grabbing = grabbing;
    grabbing.set(false);
    // Average over the last few samples with an assumed 16 ms frame, so one
    // noisy pointermove cannot make an even flick catch or slip.
    let mut velocity = 0.0;
    if state.pts.len() > 1 {
        let window = state.pts.len().min(5);
        let first = state.pts[state.pts.len() - window];
        let latest = *state.pts.last().expect("non-empty");
        let frames = (window - 1) as f64;
        if frames > 0.0 {
            let raw = (first - latest) / item_height / (frames * 16.0);
            velocity = clamp(raw, -MAX_VELOCITY, MAX_VELOCITY);
        }
    }
    fling(scroll, generation, velocity, last);
}

/// `fling`: coast a flick to a row and settle with the spring-tipped bounce.
fn fling(scroll: Signal<f64>, generation: Signal<u64>, velocity: f64, last: f64) {
    let from = *scroll.peek();
    if !(0.0..=last).contains(&from) {
        spawn_glide(
            scroll,
            generation,
            from,
            clamp(from.round(), 0.0, last),
            260.0,
            ease_out_cubic,
        );
        return;
    }
    let dir = velocity.signum();
    let coast = ((velocity * velocity) / (2.0 * DECELERATION)) * dir;
    let to = clamp((from + coast).round(), 0.0, last);
    let duration = clamp((to - from).abs().sqrt() * 300.0 + 240.0, 280.0, 1700.0);
    spawn_glide(scroll, generation, from, to, duration, ease_out_back);
}

/// Ease the drum from `from` to `to` over `duration_ms`, then land exactly.
fn spawn_glide(
    scroll: Signal<f64>,
    mut generation: Signal<u64>,
    from: f64,
    to: f64,
    duration_ms: f64,
    ease: fn(f64) -> f64,
) {
    generation += 1;
    let id = generation();
    if (to - from).abs() < f64::EPSILON || duration_ms <= 0.0 {
        let mut scroll = scroll;
        scroll.set(to);
        return;
    }
    spawn(async move {
        let start = Time::now();
        loop {
            if generation() != id {
                return;
            }
            let elapsed = Time::now().duration_since(start).as_secs_f64() * 1000.0;
            let progress = (elapsed / duration_ms).min(1.0);
            let mut scroll = scroll;
            scroll.set(from + (to - from) * ease(progress));
            if progress >= 1.0 {
                let mut scroll = scroll;
                scroll.set(to);
                return;
            }
            if Time::delay(Duration::from_millis(16)).await.is_err() {
                return;
            }
        }
    });
}

#[component]
pub fn WheelPicker(
    options: Vec<WheelPickerOption>,
    #[props(default)] value: Option<String>,
    #[props(default)] default_value: Option<String>,
    #[props(default)] on_value_change: Option<EventHandler<String>>,
    #[props(default = 5)] visible_count: usize,
    #[props(default = 36.0)] item_height: f64,
    #[props(default)] disabled: bool,
    #[props(default)] sound: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let _ = sound;
    let reduce = use_reduced_motion()();
    let controlled = value.is_some();
    let last = options.len().saturating_sub(1) as f64;

    let index_of = {
        let options = options.clone();
        move |needle: Option<String>| -> f64 {
            let needle = needle.unwrap_or_default();
            options
                .iter()
                .position(|o| o.value == needle)
                .map(|i| i as f64)
                .unwrap_or(0.0)
        }
    };

    let initial_value = value.clone().or(default_value.clone());
    let internal = use_signal(|| initial_value.clone().unwrap_or_default());
    let mut scroll = use_signal(|| index_of(initial_value.clone()));
    let grabbing = use_signal(|| false);
    let drag = use_signal(|| None::<DragState>);
    let generation = use_signal(|| 0u64);

    let current_value = match value.clone() {
        Some(value) => value,
        None => internal(),
    };

    // Mirror an external value change onto the drum.
    use_effect(use_reactive!(|value| {
        if drag.peek().is_some() {
            return;
        }
        let target = index_of(value.clone());
        let current = *scroll.peek();
        if (current.round() - target).abs() < 0.001 {
            return;
        }
        spawn_glide(scroll, generation, current, target, 260.0, ease_out_cubic);
    }));

    let emit_value = current_value.clone();
    let emit = EventHandler::new({
        let options = options.clone();
        move |index: f64| {
            let index = clamp(index.round(), 0.0, last) as usize;
            let Some(option) = options.get(index) else {
                return;
            };
            if option.value == emit_value {
                return;
            }
            if !controlled {
                let mut internal = internal;
                internal.set(option.value.clone());
            }
            if let Some(on_value_change) = on_value_change {
                on_value_change.call(option.value.clone());
            }
        }
    });

    // Cylinder geometry.
    let rows_each_side = 1usize.max(visible_count / 2);
    let cutoff = (rows_each_side + 1) as f64;
    let item_angle = 90.0 / cutoff;
    let radius = item_height / (item_angle * DEG).tan();
    let hide_beyond = cutoff;
    let height =
        (2.0 * radius * ((rows_each_side as f64) * item_angle * DEG).sin() + item_height).round();

    let rows: Vec<(usize, WheelPickerOption)> = options.iter().cloned().enumerate().collect();
    let scroll_value = scroll();
    let drum_transform = format!(
        "translateZ({}px) rotateX({}deg)",
        -radius,
        item_angle * scroll_value
    );
    let row_styles: Vec<String> = rows
        .iter()
        .map(|(i, _)| {
            let visibility = if (*i as f64 - scroll_value).abs() > hide_beyond {
                "hidden"
            } else {
                "visible"
            };
            format!(
                "top:{}px;height:{}px;transform:rotateX({}deg) translateZ({}px);visibility:{}",
                -(item_height / 2.0),
                item_height,
                -(item_angle * *i as f64),
                radius,
                visibility
            )
        })
        .collect();

    rsx! {
        if reduce {
            ReducedWheel {
                options: options.clone(),
                height,
                item_height,
                current_value: current_value.clone(),
                disabled,
                class,
                emit,
            }
        } else {
            div {
                role: "listbox",
                "aria-label": aria_label,
                tabindex: if disabled { -1 } else { 0 },
                class: cn!(
                    "relative touch-none overflow-hidden rounded-2xl border border-border bg-card outline-none focus-visible:ring-2 focus-visible:ring-foreground/20",
                    TOUCH_GESTURE_CLASS,
                    if grabbing() { "cursor-grabbing" } else { "cursor-grab" },
                    if disabled { "pointer-events-none opacity-50" } else { "" },
                    "[mask-image:linear-gradient(to_bottom,transparent,#000_22%,#000_78%,transparent)]",
                    class,
                ),
                style: "height:{height}px;perspective:1000px",
                onkeydown: move |event| {
                    if disabled {
                        return;
                    }
                    let at = scroll().round();
                    let to = match event.key() {
                        Key::ArrowUp => at - 1.0,
                        Key::ArrowDown => at + 1.0,
                        Key::Home => 0.0,
                        Key::End => last,
                        _ => return,
                    };
                    event.prevent_default();
                    spawn_glide(scroll, generation, scroll(), clamp(to, 0.0, last), 300.0, ease_out_back);
                },
                onpointerdown: move |event| {
                    if disabled || event.pointer_type() == "touch" {
                        return;
                    }
                    begin_drag(scroll, grabbing, drag, generation, event.client_coordinates().y);
                    platform::capture_pointer(&event);
                },
                onpointermove: move |event| {
                    if event.pointer_type() == "touch" {
                        return;
                    }
                    move_drag(scroll, drag, item_height, last, event.client_coordinates().y, emit);
                },
                onpointerup: move |event| {
                    if event.pointer_type() == "touch" {
                        return;
                    }
                    platform::release_pointer(&event);
                    end_drag(scroll, grabbing, drag, generation, item_height, last);
                },
                onpointercancel: move |event| {
                    platform::release_pointer(&event);
                    end_drag(scroll, grabbing, drag, generation, item_height, last);
                },
                onwheel: move |event: Event<WheelData>| {
                    if disabled {
                        return;
                    }
                    let px = match event.delta() {
                        WheelDelta::Pixels(v) => v.y,
                        WheelDelta::Lines(v) => v.y * 16.0,
                        WheelDelta::Pages(v) => v.y * 160.0,
                    };
                    let next = clamp(scroll() + px * WHEEL_SENS, 0.0, last);
                    scroll.set(next);
                    emit.call(next);
                    // One idle timer per gesture: a fresh eased step per notch
                    // would stack overlapping animations.
                    let snapshot = generation();
                    spawn(async move {
                        let _ = Time::delay(Duration::from_millis(110)).await;
                        if generation() != snapshot {
                            return;
                        }
                        spawn_glide(scroll, generation, *scroll.peek(), scroll().round().clamp(0.0, last), 240.0, ease_out_back);
                    });
                },
                ul {
                    "aria-hidden": "true",
                    class: "absolute inset-x-0 top-1/2 m-0 h-0 list-none p-0 [backface-visibility:hidden] [transform-style:preserve-3d] [will-change:transform]",
                    style: "transform:{drum_transform}",
                    for (i, option) in rows.clone() {
                        li {
                            key: "{option.value}",
                            "data-index": "{i}",
                            class: "absolute inset-x-0 flex items-center justify-center font-medium text-muted-foreground",
                            style: "{row_styles[i]}",
                            "{option.label}"
                        }
                    }
                }
                div {
                    class: "pointer-events-none absolute inset-x-0 top-1/2 z-10 -translate-y-1/2 overflow-hidden rounded-md bg-foreground/[0.04]",
                    style: "height:{item_height}px;perspective:1000px",
                    ul {
                        "aria-hidden": "true",
                        class: "absolute inset-x-0 top-1/2 m-0 h-0 list-none p-0 [backface-visibility:hidden] [transform-style:preserve-3d] [will-change:transform]",
                        style: "transform:{drum_transform}",
                        for (i, option) in rows {
                            li {
                                key: "{option.value}",
                                "data-index": "{i}",
                                class: "absolute inset-x-0 flex items-center justify-center font-medium text-foreground",
                                style: "{row_styles[i]}",
                                "{option.label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ReducedWheel(
    options: Vec<WheelPickerOption>,
    height: f64,
    item_height: f64,
    current_value: String,
    disabled: bool,
    class: Option<String>,
    emit: EventHandler<f64>,
) -> Element {
    let pad = (height - item_height) / 2.0;
    rsx! {
        div {
            class: cn!(
                "relative overflow-hidden rounded-2xl border border-border bg-card",
                if disabled { "pointer-events-none opacity-50" } else { "" },
                class,
            ),
            style: "height:{height}px",
            div {
                class: "pointer-events-none absolute inset-x-0 top-1/2 z-10 -translate-y-1/2 border-y border-border bg-foreground/[0.04]",
                style: "height:{item_height}px",
            }
            ul {
                class: "h-full snap-y snap-mandatory overflow-y-auto scroll-smooth [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden [mask-image:linear-gradient(to_bottom,transparent,#000_22%,#000_78%,transparent)]",
                style: "padding-top:{pad}px;padding-bottom:{pad}px",
                for (index, option) in options.iter().enumerate() {
                    li { key: "{option.value}", class: "snap-center",
                        button {
                            r#type: "button",
                            disabled,
                            class: cn!(
                                "flex w-full items-center justify-center font-medium",
                                if option.value == current_value { "text-foreground" } else { "text-muted-foreground" },
                            ),
                            style: "height:{item_height}px",
                            onclick: move |_| emit.call(index as f64),
                            "{option.label}"
                        }
                    }
                }
            }
        }
    }
}
