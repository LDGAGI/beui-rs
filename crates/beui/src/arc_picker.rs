//! `motion/arc-picker.tsx` — a radial single-choice picker that is dragged,
//! scrolled, or driven from the keyboard.
//!
//! The geometry is the source's: `angle = clamp((index - position) * itemHeight
//! / radius, -1.5, 1.5)`, `bend = radius * (1 - cos angle)`,
//! `along = radius * sin angle`, the per-side `x` / `y` / `rotation` / `anchor`
//! mapping, the opacity ramp `clamp((visibleCount / 2 + 0.5 - distance) / 1.5,
//! 0, 1)` times 0.35 for a disabled option, and the pointer-events window
//! `distance < visibleCount / 2`. So are the derived sizes: `spacing` (the widest
//! label plus `BRACKET_SPACE * 2` on a horizontal side), `verticalInset`,
//! `curveRadius`, `shownCount`, the `horizontalHeight` profile with its 1.35 rad
//! cap, and the `24 + 4 + 6` bracket reserve.
//!
//! The interaction is the source's too: the 6px move threshold, the velocity
//! smoothing (`0.35` old / `0.65` new, per `spacing` and elapsed ms) and the
//! `COAST_SECONDS 0.18` projection; the wheel path that follows trackpad deltas
//! directly, settles `WHEEL_SETTLE_MS 120` after the last one and hands the page
//! its own scroll back at either end; arrows, Home and End, PageUp and PageDown;
//! and the per-detent `publish` that walks every enabled choice crossed in order.
//!
//! Differences: the settle runs on a delta motion over a plain position signal
//! (the same shape `CylinderCarousel` uses), the selection frame's width and
//! expansion glide on a CSS transition rather than the source's two springs, and
//! the keyboard's shift-5 jump is not wired (step is always 1).
//!
//! Two port-specific hazards, both fixed here and worth remembering for any
//! effect that reads state another effect writes:
//!
//! - The external-value effect must not settle while a gesture is in flight
//!   (the source's `!tracking.current` guard). Without it a controlled drag
//!   snaps back to the stale value on every published detent, which ratchets the
//!   value toward the end of the list.
//! - The publish effect must subscribe to `position` (the source uses
//!   `position.on("change")`). A Dioxus effect only re-runs on the signals it
//!   read, so without a tracked read of `position` a drag publishes nothing
//!   after its first frame.

use crate::prelude::*;
use beui_core::drag::{use_drag, DragAxis, DragEnd, DragOptions, DragRange};
use beui_core::platform::{eval, js_string, json, next_scope_id};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

const COAST_SECONDS: f64 = 0.18;
const WHEEL_SETTLE_MS: u64 = 120;
const BRACKET_SPACE: f64 = 24.0;
const BRACKET_GAP: f64 = 4.0;
const BRACKET_EXPANSION: f64 = 6.0;

/// Bisect bitmask, one bit per effect: 1 drag-follow, 2 tracking, 4
/// external-change, 8 frame-width, 16 measurement, 32 wheel.
const EFFECTS: u8 = 63;

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// The enabled index nearest to `at`.
fn nearest_enabled(options: &[ArcPickerOption], at: f64) -> Option<usize> {
    let mut nearest: Option<usize> = None;
    let mut distance = f64::INFINITY;
    for (index, option) in options.iter().enumerate() {
        if option.disabled {
            continue;
        }
        let candidate = (index as f64 - at).abs();
        if candidate < distance {
            nearest = Some(index);
            distance = candidate;
        }
    }
    nearest
}

/// One choice.
#[derive(Clone, PartialEq)]
pub struct ArcPickerOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl ArcPickerOption {
    /// An enabled choice.
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    /// A choice that cannot be picked.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// The side the arc bows toward.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArcPickerSide {
    Top,
    Bottom,
    Left,
    #[default]
    Right,
}

impl ArcPickerSide {
    fn is_horizontal(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    fn name(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// Props for [`ArcPicker`].
#[derive(Props, Clone, PartialEq)]
pub struct ArcPickerProps {
    pub options: Vec<ArcPickerOption>,
    #[props(default)]
    pub value: Option<String>,
    #[props(default)]
    pub default_value: Option<String>,
    /// Called for each enabled choice crossed while scrolling, dragging or coasting.
    #[props(default)]
    pub on_value_change: Option<EventHandler<String>>,
    #[props(default)]
    pub side: ArcPickerSide,
    /// Radius in pixels. Defaults to `280`.
    #[props(default = 280.0)]
    pub radius: f64,
    /// Minimum spacing along the arc. Defaults to `48`.
    #[props(default = 48.0)]
    pub item_height: f64,
    #[props(default = 7)]
    pub visible_count: usize,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub name: Option<String>,
    #[props(default = "Choose an option".to_string())]
    pub aria_label: String,
    #[props(default)]
    pub class: Option<String>,
}

/// A radial choice picker.
#[component]
pub fn ArcPicker(props: ArcPickerProps) -> Element {
    let ArcPickerProps {
        options,
        value,
        default_value,
        on_value_change,
        side,
        radius,
        item_height,
        visible_count,
        disabled,
        name,
        aria_label,
        class,
    } = props;

    let reduced_motion = use_reduced_motion();
    let reduce = reduced_motion();
    let horizontal = side.is_horizontal();
    let scope = use_hook(next_scope_id);
    let root_selector = format!("[data-beui-arc=\"{scope}\"]");

    let first_enabled = options
        .iter()
        .find(|option| !option.disabled)
        .map(|option| option.value.clone());
    let controlled = value.is_some();
    let internal_value = use_signal(|| default_value.clone().or(first_enabled.clone()));
    let internal_now = internal_value();
    let candidate = value.clone().or(internal_now).or(first_enabled.clone());
    let selected_value = options
        .iter()
        .find(|option| Some(&option.value) == candidate.as_ref() && !option.disabled)
        .map(|option| option.value.clone())
        .or(first_enabled.clone());
    let selected_index = options
        .iter()
        .position(|option| Some(&option.value) == selected_value.as_ref())
        .unwrap_or(0) as f64;

    let safe_radius = if radius.is_finite() {
        radius.max(120.0)
    } else {
        280.0
    };
    let row_height = if item_height.is_finite() {
        item_height.max(32.0)
    } else {
        48.0
    };
    let count = (if visible_count == 0 { 7 } else { visible_count }).max(3);

    // ── state ────────────────────────────────────────────────────────────────
    let position = use_signal(|| selected_index);
    let settle_base = use_signal(|| selected_index);
    let mut settle_motion = use_motion(0.0f32).expect("0 is a finite value");
    let settling = use_signal(|| false);
    let tracking = use_signal(|| false);
    let activity = use_signal(|| 0.0f64);
    let instant = use_signal(|| false);
    let request = use_signal(|| selected_value.clone());
    let suppress_click = use_signal(|| false);
    let frame_width = use_signal(|| 0.0f64);
    let moved = use_signal(|| false);
    let dragging = use_signal(|| false);
    let start_position = use_signal(|| 0.0f64);
    let wheel_target = use_signal(|| None::<f64>);
    let measurements = use_signal(|| (0.0f64, 0.0f64));
    let wheel_anchor = use_signal(|| 0.0f64);

    // ── derived sizes ────────────────────────────────────────────────────────
    let (root_width, label_width) = measurements();
    let spacing = if horizontal {
        row_height.max(label_width + BRACKET_SPACE * 2.0)
    } else {
        row_height
    };
    let vertical_inset = BRACKET_SPACE
        .max((root_width * 0.42).min((root_width - label_width - BRACKET_SPACE).max(0.0)));
    let vertical_anchor: f64 = if root_width > 0.0 {
        if side == ArcPickerSide::Left {
            root_width - vertical_inset
        } else {
            vertical_inset
        }
    } else if side == ArcPickerSide::Left {
        root_width * 0.58
    } else {
        root_width * 0.42
    };
    let curve_radius = if horizontal {
        safe_radius.max(spacing * 2.0)
    } else {
        safe_radius
    };
    let shown_count = if horizontal {
        (count as f64).min(1.0f64.max(
            (if root_width > 0.0 {
                root_width
            } else {
                count as f64 * spacing
            }) / spacing,
        ))
    } else {
        count as f64
    };
    let horizontal_height = (row_height * 2.0)
        .max(
            curve_radius
                * (1.0
                    - (1.35f64)
                        .min((shown_count * spacing) / (2.0 * curve_radius))
                        .cos())
                + row_height
                + 24.0,
        )
        .ceil();
    let rendered_position = if settling() {
        settle_base() + f64::from(current(settle_motion))
    } else {
        position()
    };

    // ── closures ─────────────────────────────────────────────────────────────
    let stop = {
        move || {
            let mut settling = settling;
            let mut tracking = tracking;
            let mut activity = activity;
            settle_motion.stop();
            settling.set(false);
            tracking.set(false);
            activity.set(0.0);
        }
    };

    let settle = {
        let mut settling = settling;
        let mut tracking = tracking;
        let mut activity = activity;
        let mut settle_base = settle_base;
        let mut position = position;
        move |index: f64, immediate: bool| {
            settle_motion.stop();
            if immediate || reduce {
                settle_base.set(index);
                position.set(index);
                settle_motion.reset();
                settling.set(false);
                tracking.set(false);
                activity.set(0.0);
                return;
            }
            let from = *position.peek();
            settle_base.set(from);
            settle_motion.reset();
            settling.set(true);
            let _ = settle_motion.animate_to(
                (index - from) as f32,
                AnimationConfig::spring(SPRING_LAYOUT),
            );
        }
    };

    let publish = {
        let options = options.clone();
        let mut request = request;
        let mut internal_value = internal_value;
        move |index: usize| {
            if disabled {
                return;
            }
            let Some(option) = options.get(index) else {
                return;
            };
            if option.disabled || request().as_deref() == Some(option.value.as_str()) {
                return;
            }
            request.set(Some(option.value.clone()));
            if !controlled {
                internal_value.set(Some(option.value.clone()));
            }
            if let Some(handler) = on_value_change {
                handler.call(option.value.clone());
            }
        }
    };

    // ── pointer drag ─────────────────────────────────────────────────────────
    let on_drag_end = EventHandler::new({
        let options = options.clone();
        let mut settle = settle;
        let mut moved = moved;
        let mut tracking = tracking;
        let mut activity = activity;
        let mut suppress_click = suppress_click;
        move |end: DragEnd| {
            if !moved() {
                return;
            }
            suppress_click.set(true);
            let velocity = if horizontal {
                end.velocity.x
            } else {
                end.velocity.y
            };
            // A positive drag distance rolls the arc back, so the projection is
            // subtracted.
            let projected = if reduce {
                rendered_position
            } else {
                rendered_position - (velocity / spacing) * COAST_SECONDS
            };
            if let Some(target) = nearest_enabled(&options, projected) {
                settle(target as f64, false);
            }
            tracking.set(false);
            activity.set(0.0);
            moved.set(false);
        }
    });
    let drag = use_drag(
        DragOptions::default()
            .axis(if horizontal { DragAxis::X } else { DragAxis::Y })
            .range(DragRange::FREE, DragRange::FREE)
            .without_momentum(),
        Some(on_drag_end),
    );

    use_effect({
        let mut position = position;
        let mut moved = moved;
        let mut tracking = tracking;
        let mut activity = activity;
        let options_len = options.len();
        move || {
            if EFFECTS & 1 == 0 {
                return;
            }
            if !dragging() {
                return;
            }
            let offset = drag.offset();
            let along = if horizontal { offset.x } else { offset.y };
            if !moved() && along.abs() < 6.0 {
                return;
            }
            if !moved() {
                moved.set(true);
            }
            if !tracking() {
                tracking.set(true);
                activity.set(if reduce { 0.0 } else { 1.0 });
            }
            let next = clamp(
                start_position() - along / spacing,
                0.0,
                options_len.saturating_sub(1) as f64,
            );
            position.set(if reduce { next.round() } else { next });
        }
    });

    // Every enabled choice crossed is published in order. The source subscribes
    // to `position.on("change")`; a tracked read of `position` is the Dioxus
    // spelling. Without it the effect only runs when `tracking`/`request`
    // change, so a drag publishes nothing after its first frame.
    use_effect({
        let options = options.clone();
        let mut activity = activity;
        let mut publish = publish.clone();
        move || {
            if EFFECTS & 2 == 0 {
                return;
            }
            let at = position();
            if !tracking() || disabled {
                return;
            }
            let Some(next) = nearest_enabled(&options, at) else {
                return;
            };
            let previous = request()
                .as_deref()
                .and_then(|value| options.iter().position(|option| option.value == value));
            match previous {
                None => publish(next),
                Some(previous) if previous == next => {}
                Some(previous) => {
                    let step: i64 = if next > previous { 1 } else { -1 };
                    let mut index = previous as i64 + step;
                    loop {
                        publish(index.max(0) as usize);
                        if index == next as i64 {
                            break;
                        }
                        index += step;
                    }
                }
            }
            let _ = &mut activity;
        }
    });

    // An external selection owns the control immediately — unless the user is
    // mid-gesture and the change is the selection that gesture just published
    // (the source's `!tracking.current` guard). Without that guard a controlled
    // drag settles back to the stale value on every published detent, which
    // ratchets the value toward the end of the list.
    let selected_for_effect = selected_value.clone();
    use_effect(use_reactive(
        (&selected_index, &disabled, &reduce),
        move |(selected_index, _disabled, _reduce)| {
            if EFFECTS & 4 == 0 {
                return;
            }
            let mut settle = settle;
            let mut instant = instant;
            let mut request = request;
            let mut stop = stop;
            // `peek` the gesture state: the source reads it from a ref, so a
            // flip of `tracking` must not, on its own, re-run this effect.
            let tracking_now = *tracking.peek();
            let accepted_gesture =
                tracking_now && request.peek().as_deref() == selected_for_effect.as_deref();
            if accepted_gesture {
                instant.set(false);
                return;
            }
            stop();
            request.set(selected_for_effect.clone());
            // `peek`, so clearing the flag cannot re-trigger this effect, and a
            // guard, so a settle onto the position it is already at never starts
            // a no-op spring: that spring never completes, which keeps the render
            // reading the motion every frame and takes the page down with it.
            let jump = *instant.peek() || disabled || reduce;
            instant.set(false);
            if (*position.peek() - selected_index).abs() > 0.001 {
                settle(selected_index, jump);
            }
        },
    ));

    // The frame is as wide as the selected button.
    use_effect(use_reactive((&selected_value,), {
        let root_selector = root_selector.clone();
        move |(selected_value,)| {
            if EFFECTS & 8 == 0 {
                return;
            }
            let selector = format!(
                "{root_selector} [data-value=\"{}\"]",
                selected_value.clone().unwrap_or_default()
            );
            spawn(async move {
                let mut frame_width = frame_width;
                let script = format!(
                    "const el = document.querySelector({});\nreturn el ? el.offsetWidth : 0;",
                    js_string(&selector)
                );
                if let Some(width) = json::<f64>(&script).await {
                    frame_width.set(width);
                }
            });
        }
    }));

    // The root width and the widest label, from one observer.
    let revision: String = options
        .iter()
        .map(|option| format!("{}|{}|{}", option.value, option.label, option.disabled))
        .collect::<Vec<String>>()
        .join(",");
    use_effect(use_reactive((&revision,), {
        let root_selector = root_selector.clone();
        move |_| {
            if EFFECTS & 16 == 0 {
                return;
            }
            let root_selector = root_selector.clone();
            spawn(async move {
                let script = format!(
                    "const root = document.querySelector({selector});\n\
                     if (!root) return;\n\
                     const send = () => {{\n\
                       const buttons = [...root.querySelectorAll('[data-slot=\"arc-picker-option\"]')];\n\
                       const widest = buttons.reduce((max, button) => Math.max(max, button.offsetWidth), 0);\n\
                       dioxus.send([root.clientWidth, widest]);\n\
                     }};\n\
                     send();\n\
                     const observer = new ResizeObserver(send);\n\
                     observer.observe(root);\n\
                     for (const button of root.querySelectorAll('[data-slot=\"arc-picker-option\"]')) observer.observe(button);\n\
                     for (;;) {{ if ((await dioxus.recv()) === \"close\") break; }}\n\
                     observer.disconnect();",
                    selector = js_string(&root_selector),
                );
                let Some(mut handle) = eval(&script) else {
                    return;
                };
                let mut measurements = measurements;
                while let Ok(next) = handle.recv::<Vec<f64>>().await {
                    if next.len() >= 2 {
                        let value = (next[0], next[1]);
                        if measurements.peek().to_owned() != value {
                            measurements.set(value);
                        }
                    }
                }
            });
        }
    }));

    // The wheel, installed as a real listener so it can preventDefault. The JS
    // side decides when to hand the page its own scroll back, using the checked
    // button's index plus the deltas seen since it last moved.
    let wheel_key = format!("{revision}|{spacing}|{horizontal}");
    use_effect(use_reactive((&wheel_key,), {
        let root_selector = root_selector.clone();
        let options = options.clone();
        move |_| {
            if EFFECTS & 32 == 0 {
                return;
            }
            let root_selector = root_selector.clone();
            let options_key: Vec<(String, bool)> = options
                .iter()
                .map(|option| (option.value.clone(), option.disabled))
                .collect();
            let options_for_settle = options.clone();
            spawn(async move {
                let script = format!(
                    "const root = document.querySelector({selector});\n\
                     if (!root) return;\n\
                     const spacing = {spacing};\n\
                     const horizontal = {horizontal};\n\
                     let acc = 0;\n\
                     let selected = -1;\n\
                     const onWheel = (event) => {{\n\
                       if (event.ctrlKey) return;\n\
                       const useX = Math.abs(event.deltaX) > Math.abs(event.deltaY);\n\
                       if (!horizontal && useX) return;\n\
                       let delta = horizontal && useX ? event.deltaX : event.deltaY;\n\
                       delta *= event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? (horizontal ? root.clientWidth : root.clientHeight) : 1;\n\
                       if (!delta) return;\n\
                       const buttons = [...root.querySelectorAll('[data-slot=\"arc-picker-option\"]')];\n\
                       const checked = buttons.findIndex((button) => button.getAttribute('aria-checked') === 'true');\n\
                       if (checked !== selected) {{ selected = checked; acc = 0; }}\n\
                       const enabled = buttons.map((button) => !button.disabled);\n\
                       const first = enabled.indexOf(true);\n\
                       const last = enabled.lastIndexOf(true);\n\
                       if (first < 0) return;\n\
                       const from = selected + acc / spacing;\n\
                       // At either end the page keeps its own scroll.\n\
                       if ((from <= first && delta < 0) || (from >= last && delta > 0)) return;\n\
                       event.preventDefault();\n\
                       acc += delta;\n\
                       dioxus.send(delta);\n\
                     }};\n\
                     root.addEventListener('wheel', onWheel, {{ passive: false }});\n\
                     for (;;) {{ if ((await dioxus.recv()) === \"close\") break; }}\n\
                     root.removeEventListener('wheel', onWheel);",
                    selector = js_string(&root_selector),
                    spacing = spacing,
                    horizontal = if horizontal { "true" } else { "false" },
                );
                let Some(mut handle) = eval(&script) else {
                    return;
                };
                let mut position = position;
                let mut tracking = tracking;
                let mut activity = activity;
                let mut wheel_target = wheel_target;
                let mut wheel_anchor = wheel_anchor;
                while let Ok(delta) = handle.recv::<f64>().await {
                    let enabled: Vec<usize> = options_key
                        .iter()
                        .enumerate()
                        .filter(|(_, (_, option_disabled))| !option_disabled)
                        .map(|(index, _)| index)
                        .collect();
                    let (Some(first), Some(last)) =
                        (enabled.first().copied(), enabled.last().copied())
                    else {
                        continue;
                    };
                    let at = wheel_anchor();
                    let mut settle = settle;
                    if reduce {
                        let step = if delta > 0.0 { 1i64 } else { -1i64 };
                        let index =
                            clamp((at as i64 + step) as f64, first as f64, last as f64) as usize;
                        wheel_anchor.set(index as f64);
                        settle(index as f64, true);
                        continue;
                    }
                    tracking.set(true);
                    activity.set(1.0);
                    let from = wheel_target().unwrap_or(at);
                    // Trackpad deltas already carry momentum; follow them
                    // directly rather than lagging behind a spring.
                    let next = clamp(from + delta / spacing, first as f64, last as f64);
                    wheel_target.set(Some(next));
                    wheel_anchor.set(next);
                    position.set(next);
                    // Settle a moment after the last event.
                    let settle = settle;
                    let options_for_settle = options_for_settle.clone();
                    let mut wheel_target = wheel_target;
                    let mut tracking = tracking;
                    let mut activity = activity;
                    spawn(async move {
                        let mut settle = settle;
                        let _ = json::<bool>(&format!(
                            "await new Promise((resolve) => setTimeout(resolve, {WHEEL_SETTLE_MS}));\nreturn true;"
                        ))
                        .await;
                        let Some(target) = wheel_target() else {
                            return;
                        };
                        wheel_target.set(None);
                        if let Some(index) = nearest_enabled(&options_for_settle, target) {
                            settle(index as f64, false);
                        }
                        tracking.set(false);
                        activity.set(0.0);
                    });
                }
            });
        }
    }));

    // ── render ───────────────────────────────────────────────────────────────
    let option_views: Vec<Element> = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let selected = Some(&option.value) == selected_value.as_ref();
            let angle = clamp(
                (index as f64 - rendered_position) * spacing / curve_radius,
                -1.5,
                1.5,
            );
            let bend = curve_radius * (1.0 - angle.cos());
            let along = curve_radius * angle.sin();
            let (x, y) = if horizontal {
                (along, if side == ArcPickerSide::Top { bend } else { -bend })
            } else {
                (if side == ArcPickerSide::Left { bend } else { -bend }, along)
            };
            let rotation = if matches!(side, ArcPickerSide::Left | ArcPickerSide::Bottom) {
                -angle
            } else {
                angle
            } * 180.0
                / std::f64::consts::PI;
            let anchor = if horizontal {
                "-50%"
            } else if side == ArcPickerSide::Left {
                "-100%"
            } else {
                "0%"
            };
            let distance = (index as f64 - rendered_position).abs();
            let opacity = clamp((shown_count / 2.0 + 0.5 - distance) / 1.5, 0.0, 1.0)
                * if disabled || option.disabled { 0.35 } else { 1.0 };
            let pointer_events = if distance < shown_count / 2.0 { "auto" } else { "none" };
            let top = if horizontal {
                if side == ArcPickerSide::Top {
                    "36px"
                } else {
                    "calc(100% - 36px)"
                }
            } else {
                "50%"
            };
            let left = if horizontal {
                "50%".to_string()
            } else {
                format!("{vertical_anchor}px")
            };
            let transform_origin = if horizontal {
                "center"
            } else if side == ArcPickerSide::Left {
                "right center"
            } else {
                "left center"
            };
            let max_width = if horizontal {
                "calc(100% - 4rem)"
            } else {
                "calc(100% - 3rem)"
            };
            let style = format!(
                "top:{top};left:{left};transform-origin:{transform_origin};max-width:{max_width};\
                 transform:translate3d({x}px, {y}px, 0) translate({anchor}, -50%) rotate({rotation}deg);\
                 opacity:{opacity};pointer-events:{pointer_events}"
            );
            let option_class = cn!(
                "absolute rounded-md px-2 py-1 text-2xl font-light whitespace-nowrap outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-35",
                if horizontal { "text-center" } else { "" },
                if !horizontal && side == ArcPickerSide::Left {
                    "text-right"
                } else if !horizontal {
                    "text-left"
                } else {
                    ""
                },
                if selected {
                    "text-primary"
                } else {
                    "text-muted-foreground"
                },
            );
            let value = option.value.clone();
            let option_disabled = option.disabled;
            let options_for_click = options.clone();
            let mut publish_for_click = publish.clone();
            let mut settle_for_click = settle;
            rsx! {
                button {
                    key: "{option.value}",
                    r#type: "button",
                    role: "radio",
                    aria_checked: if selected { "true" } else { "false" },
                    disabled: disabled || option_disabled,
                    tabindex: if selected && !disabled && !option_disabled { 0 } else { -1 },
                    "data-slot": "arc-picker-option",
                    "data-value": "{option.value}",
                    class: option_class,
                    style,
                    onclick: {
                        let mut suppress_click = suppress_click;
                        move |_| {
                            if suppress_click() {
                                suppress_click.set(false);
                                return;
                            }
                            if let Some(index) = options_for_click
                                .iter()
                                .position(|candidate| candidate.value == value)
                            {
                                settle_for_click(index as f64, true);
                                publish_for_click(index);
                            }
                        }
                    },
                    span { "data-slot": "arc-picker-label", class: "block overflow-hidden text-ellipsis",
                        "{option.label}"
                    }
                }
            }
        })
        .collect();

    let expansion = activity();
    let leading_extent = if horizontal {
        frame_width() / 2.0
    } else if side == ArcPickerSide::Left {
        frame_width()
    } else {
        0.0
    };
    let trailing_extent = if horizontal {
        frame_width() / 2.0
    } else if side == ArcPickerSide::Right {
        frame_width()
    } else {
        0.0
    };
    let gap = BRACKET_GAP + clamp(expansion, 0.0, 1.0) * BRACKET_EXPANSION;
    let scale_y = 1.0 - clamp(expansion, 0.0, 1.0) * 0.08;
    let frame_opacity = if frame_width() > 0.0 { 0.85 } else { 0.0 };
    let frame_top = if horizontal {
        if side == ArcPickerSide::Top {
            "36px"
        } else {
            "calc(100% - 36px)"
        }
    } else {
        "50%"
    };
    let frame_left = if horizontal {
        "50%".to_string()
    } else {
        format!("{vertical_anchor}px")
    };
    let frame_transition = if reduce {
        "none".to_string()
    } else {
        "transform 300ms cubic-bezier(0.16, 1, 0.3, 1), opacity 200ms ease-out".to_string()
    };
    let leading_style = format!(
        "transform:translate3d({}px, -50%, 0) scaleY({scale_y});opacity:{frame_opacity};transition:{frame_transition}",
        -leading_extent - gap
    );
    let trailing_style = format!(
        "transform:translate3d({}px, -50%, 0) scaleY({scale_y});opacity:{frame_opacity};transition:{frame_transition}",
        trailing_extent + gap
    );
    let frame_style = format!(
        "top:{frame_top};left:{frame_left};visibility:{}",
        if selected_value.is_some() {
            "visible"
        } else {
            "hidden"
        }
    );
    let root_style = format!(
        "touch-action:{};height:{}px;mask-image:linear-gradient(to {}, transparent, black 12%, black 88%, transparent)",
        if horizontal { "pan-y pinch-zoom" } else { "pan-x pinch-zoom" },
        if horizontal { horizontal_height } else { count as f64 * row_height },
        if horizontal { "right" } else { "bottom" },
    );
    let root_class =
        cn!(
        "relative w-full min-w-0 cursor-grab select-none overflow-hidden active:cursor-grabbing",
        if disabled { "cursor-default opacity-50" } else { "" },
        class,
    );
    let instructions = format!(
        "Drag {} or scroll to choose. Use arrow keys to move, Home and End to jump.",
        if horizontal {
            "horizontally"
        } else {
            "vertically"
        }
    );
    let hidden_value = selected_value.clone().unwrap_or_default();
    let frame_visible = selected_value.is_some();

    rsx! {
        div {
            "data-beui-arc": "{scope}",
            role: "radiogroup",
            aria_label: "{aria_label}",
            aria_orientation: if horizontal { "horizontal" } else { "vertical" },
            aria_disabled: if disabled { Some("true") } else { None },
            "data-side": side.name(),
            class: root_class,
            style: root_style,
            onpointerdown: {
                let mut dragging = dragging;
                let mut moved = moved;
                let mut suppress_click = suppress_click;
                let mut start_position = start_position;
                let mut stop = stop;
                move |event: Event<PointerData>| {
                    if disabled || selected_value.is_none() {
                        return;
                    }
                    stop();
                    suppress_click.set(false);
                    moved.set(false);
                    start_position.set(position());
                    drag.on_pointer_down(&event);
                    dragging.set(true);
                }
            },
            onpointermove: move |event: Event<PointerData>| drag.on_pointer_move(&event),
            onpointerup: {
                let mut dragging = dragging;
                move |event: Event<PointerData>| {
                    drag.on_pointer_up(&event);
                    dragging.set(false);
                }
            },
            onpointercancel: {
                let mut dragging = dragging;
                move |event: Event<PointerData>| {
                    drag.on_pointer_cancel(&event);
                    dragging.set(false);
                }
            },
            onclick: {
                let mut suppress_click = suppress_click;
                move |event: Event<MouseData>| {
                    if suppress_click() {
                        event.prevent_default();
                        event.stop_propagation();
                        suppress_click.set(false);
                    }
                }
            },
            onkeydown: {
                let options = options.clone();
                let mut publish = publish.clone();
                let mut settle = settle;
                let mut instant = instant;
                let mut wheel_anchor = wheel_anchor;
                move |event: Event<KeyboardData>| {
                    if disabled {
                        return;
                    }
                    let enabled: Vec<usize> = options
                        .iter()
                        .enumerate()
                        .filter(|(_, option)| !option.disabled)
                        .map(|(index, _)| index)
                        .collect();
                    if enabled.is_empty() {
                        return;
                    }
                    let at = enabled
                        .iter()
                        .position(|index| {
                            options.get(*index).map(|option| &option.value) == request().as_ref()
                        })
                        .unwrap_or(0);
                    let next = match event.key() {
                        Key::ArrowDown | Key::ArrowRight => Some(at as i64 + 1),
                        Key::ArrowUp | Key::ArrowLeft => Some(at as i64 - 1),
                        Key::Home => Some(0),
                        Key::End => Some(enabled.len() as i64 - 1),
                        Key::PageDown => Some(at as i64 + 5),
                        Key::PageUp => Some(at as i64 - 5),
                        _ => None,
                    };
                    let Some(next) = next else {
                        return;
                    };
                    event.prevent_default();
                    let clamped = clamp(next as f64, 0.0, enabled.len() as f64 - 1.0) as usize;
                    let index = enabled[clamped];
                    instant.set(true);
                    wheel_anchor.set(index as f64);
                    settle(index as f64, true);
                    publish(index);
                }
            },
            span { class: "sr-only", "{instructions}" }
            if let Some(name) = name.clone() {
                input {
                    r#type: "hidden",
                    name,
                    value: "{hidden_value}",
                    disabled,
                }
            }
            for view in option_views {
                {view}
            }
            div {
                aria_hidden: "true",
                "data-slot": "arc-picker-selection-frame",
                class: "pointer-events-none absolute h-0 w-0 text-2xl font-light text-primary",
                style: frame_style,
                if frame_visible {
                    span { "data-slot": "arc-picker-bracket", "data-edge": "leading", class: "absolute right-0", style: leading_style, "[" }
                    span { "data-slot": "arc-picker-bracket", "data-edge": "trailing", class: "absolute left-0", style: trailing_style, "]" }
                }
            }
            if options.is_empty() {
                span { class: "absolute inset-0 flex items-center justify-center text-sm text-muted-foreground",
                    "No options"
                }
            }
        }
    }
}
