//! The five slider designs — `range-slider-{inline,fluid,wave,bubble,ruler}.tsx`.
//!
//! All five share `beui-core::hooks::use_slider` for the value and the input
//! plumbing; what differs is the geometry. Recorded differences from the source
//! are called out at each component — the fast lane's rule is that a difference
//! has to be written down, not hidden.

use std::f64::consts::E;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};
use dioxus_motion::AnimationManager;

use beui_core::cn;
use beui_core::ease::{SPRING_GLIDE, SPRING_PANEL, SPRING_PRESS};
use beui_core::hooks::{use_reduced_motion, use_slider};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};
use beui_core::touch::TOUCH_GESTURE_CLASS;

use crate::motion::{animate_number, number, spring, tween, use_number_motion, Ease};
use crate::range_slider::{round6, slider_config, use_smooth_percent};

// --- fluid ------------------------------------------------------------------

/// Thumbless slider: the whole pill is the control. The fill is a full-size
/// layer clipped to the value, so the label inside it inverts as the fill covers
/// it and stays glyph-for-glyph aligned with the copy underneath.
#[component]
pub fn FluidSlider(
    #[props(default)] value: Option<f64>,
    #[props(default)] default_value: f64,
    #[props(default)] on_value_change: Option<EventHandler<f64>>,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] label: Option<String>,
    #[props(default)] format: Option<Callback<f64, String>>,
    #[props(default)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let format = format.unwrap_or_else(|| Callback::new(|v: f64| format!("{v}%")));
    let slider = use_slider(
        slider_config(min, max, step, default_value, disabled, Some(format)),
        value,
        on_value_change,
    );
    let percent = slider.percent();
    let pos = use_smooth_percent(percent, reduce);
    let mut scale = use_number_motion(1.0);
    let dragging = slider.dragging();
    use_effect(use_reactive!(|dragging| {
        if !reduce {
            animate_number(
                &mut scale,
                if dragging { 1.03 } else { 1.0 },
                spring(SPRING_PRESS),
            );
        }
    }));

    let uncovered = 100.0 - pos;
    let clip_path = format!("inset(0 {uncovered}% 0 0 round 9999px)");
    let scale_value = if reduce { 1.0 } else { number(&scale) };
    let (lo, hi) = slider.range();
    let scope = slider.scope();
    let readout = format.call(slider.current());

    rsx! {
        div {
            "data-beui-slider": scope.clone(),
            class: cn!(
                "relative flex h-12 w-full touch-none overflow-hidden rounded-full bg-muted",
                TOUCH_GESTURE_CLASS,
                if disabled { "pointer-events-none opacity-50" } else { "cursor-grab active:cursor-grabbing" },
                class,
            ),
            style: "transform:scale({scale_value})",
            onpointerdown: move |event| slider.on_pointer_down(&event),
            onpointermove: move |event| slider.on_pointer_move(&event),
            onpointerup: move |event| slider.end_drag(&event),
            onpointercancel: move |event| slider.end_drag(&event),
            onlostpointercapture: move |event| slider.end_drag(&event),
            div { class: "pointer-events-none absolute inset-0 flex items-center justify-between px-5 text-sm font-medium text-foreground",
                if let Some(label) = label.clone() { span { class: "truncate", "{label}" } } else { span {} }
                span { class: "tabular-nums", "{readout}" }
            }
            div { class: "absolute inset-0", style: "clip-path:{clip_path}",
                div { class: "absolute inset-0 bg-foreground" }
                div { class: "pointer-events-none absolute inset-0 flex items-center justify-between px-5 text-sm font-medium text-background",
                    if let Some(label) = label.clone() { span { class: "truncate", "{label}" } } else { span {} }
                    span { class: "tabular-nums", "{readout}" }
                }
            }
            button {
                "data-beui-slider-handle": scope.clone(),
                r#type: "button",
                role: "slider",
                tabindex: if disabled { -1 } else { 0 },
                "aria-label": aria_label,
                "aria-valuemin": "{lo}",
                "aria-valuemax": "{hi}",
                "aria-valuenow": "{slider.current()}",
                "aria-valuetext": slider.aria_value_text(),
                "aria-disabled": disabled.then_some("true"),
                class: "absolute inset-0 touch-none rounded-full outline-none ring-inset ring-foreground/40 focus-visible:ring-4",
                onkeydown: move |event| slider.on_key_down(&event),
            }
        }
    }
}

// --- wave -------------------------------------------------------------------

/// Per-bar spring: soft enough that the crest wobbles as it travels.
const SPRING_BAR: dioxus_motion::prelude::Spring = dioxus_motion::prelude::Spring {
    stiffness: 420.0,
    damping: 20.0,
    mass: 0.5,
};
/// Width of the crest in bars — bigger spreads the bell wider.
const SPREAD: f64 = 2.6;

/// Equalizer slider: bars rise into a crest around the handle position and fall
/// back as it passes, so the value reads as a travelling wave.
#[component]
pub fn WaveSlider(
    #[props(default)] value: Option<f64>,
    #[props(default)] default_value: f64,
    #[props(default)] on_value_change: Option<EventHandler<f64>>,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default = 32.0)] bars: f64,
    #[props(default)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let bars = bars.max(1.0) as usize;
    let slider = use_slider(
        slider_config(min, max, step, default_value, disabled, None),
        value,
        on_value_change,
    );
    let percent = slider.percent();
    let dragging = slider.dragging();
    let head = (percent / 100.0) * (bars as f64 - 1.0);
    let (lo, hi) = slider.range();
    let scope = slider.scope();

    let lanes: Vec<(usize, f64, f64, bool)> = (0..bars)
        .map(|i| {
            let distance = (i as f64 - head).abs();
            let crest = E.powf(-(distance * distance) / (2.0 * SPREAD * SPREAD));
            (i, distance, crest, (i as f64) <= head.round())
        })
        .collect();

    rsx! {
        div {
            "data-beui-slider": scope.clone(),
            class: cn!(
                "relative flex h-20 w-full touch-none items-center justify-between gap-1",
                TOUCH_GESTURE_CLASS,
                if disabled { "pointer-events-none opacity-50" } else { "cursor-grab active:cursor-grabbing" },
                class,
            ),
            onpointerdown: move |event| slider.on_pointer_down(&event),
            onpointermove: move |event| slider.on_pointer_move(&event),
            onpointerup: move |event| slider.end_drag(&event),
            onpointercancel: move |event| slider.end_drag(&event),
            onlostpointercapture: move |event| slider.end_drag(&event),
            for (i, distance, crest, filled) in lanes {
                WaveBar { key: "{i}", distance, crest, filled, dragging, reduce }
            }
            button {
                "data-beui-slider-handle": scope.clone(),
                r#type: "button",
                role: "slider",
                tabindex: if disabled { -1 } else { 0 },
                "aria-label": aria_label,
                "aria-valuemin": "{lo}",
                "aria-valuemax": "{hi}",
                "aria-valuenow": "{slider.current()}",
                "aria-valuetext": slider.aria_value_text(),
                "aria-disabled": disabled.then_some("true"),
                class: "absolute inset-0 touch-none rounded-xl outline-none ring-foreground/30 focus-visible:ring-4",
                onkeydown: move |event| slider.on_key_down(&event),
            }
        }
    }
}

#[component]
fn WaveBar(distance: f64, crest: f64, filled: bool, dragging: bool, reduce: bool) -> Element {
    let target = if reduce {
        0.4
    } else {
        0.22 + crest * if dragging { 0.78 } else { 0.6 }
    };
    let mut scale = use_number_motion(target as f32);
    use_effect(use_reactive!(|target| {
        if !reduce {
            let delay = (distance * 0.012).min(0.12);
            let config = spring(SPRING_BAR).with_delay(Duration::from_secs_f64(delay));
            animate_number(&mut scale, target as f32, config);
        }
    }));

    rsx! {
        span {
            "data-wave-bar": "",
            class: cn!(
                "h-14 flex-1 origin-center rounded-full",
                if filled { "bg-foreground" } else { "bg-foreground/45" },
            ),
            style: "transform:scaleY({number(&scale)})",
        }
    }
}

// --- bubble -----------------------------------------------------------------

/// Loose enough that the bubble keeps leaning a beat after the pointer stops.
const SPRING_TILT: dioxus_motion::prelude::Spring = dioxus_motion::prelude::Spring {
    stiffness: 260.0,
    damping: 22.0,
    mass: 0.4,
};
/// Drag speed (percent/s) that maxes out lean and squash.
const FULL_TILT: f64 = 320.0;
/// The bubble has one identity; the key must be a formatted string in `rsx!`.
const BUBBLE_KEY: &str = "bubble";

/// Slider with a value bubble that pops out of the thumb on grab and leans into
/// the direction of travel.
///
/// Difference from the source: the lean is driven by the frame-to-frame change
/// in the smoothed position (`useVelocity(pos)` in motion) rather than by a
/// continuous velocity motion value, so a very fast flick leans a little less.
#[component]
pub fn BubbleSlider(
    #[props(default)] value: Option<f64>,
    #[props(default)] default_value: f64,
    #[props(default)] on_value_change: Option<EventHandler<f64>>,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] format: Option<Callback<f64, String>>,
    #[props(default)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let slider = use_slider(
        slider_config(min, max, step, default_value, disabled, format),
        value,
        on_value_change,
    );
    let percent = slider.percent();
    let pos = use_smooth_percent(percent, reduce);
    let mut thumb_scale = use_number_motion(1.0);
    let mut lean = use_number_motion(0.0);

    let dragging = slider.dragging();
    use_effect(use_reactive!(|dragging| {
        if !reduce {
            animate_number(
                &mut thumb_scale,
                if dragging { 1.25 } else { 1.0 },
                spring(SPRING_PRESS),
            );
            if !dragging {
                animate_number(&mut lean, 0.0, spring(SPRING_TILT));
            }
        }
    }));

    // Frame-to-frame velocity of the smoothed position, clamped to FULL_TILT.
    let mut previous = use_signal(|| percent);
    let mut last_time = use_signal(Time::now);
    use_effect(use_reactive!(|percent| {
        if reduce {
            return;
        }
        let now = Time::now();
        // `peek`: this effect writes both signals, and a tracked read would make
        // it subscribe to its own writes and re-run forever.
        let elapsed = now
            .duration_since(*last_time.peek())
            .as_secs_f64()
            .max(1.0e-4);
        let velocity = (percent - *previous.peek()) / elapsed;
        previous.set(percent);
        last_time.set(now);
        let clamped = (velocity / FULL_TILT).clamp(-1.0, 1.0);
        animate_number(&mut lean, clamped as f32, spring(SPRING_TILT));
    }));

    let lean_value = if reduce {
        0.0
    } else {
        f64::from(number(&lean))
    };
    let tilt = lean_value * 16.0;
    let squash = 1.0 + lean_value.abs() * 0.18;
    let stretch = 1.0 - lean_value.abs() * 0.12;
    let thumb_scale_value = if reduce { 1.0 } else { number(&thumb_scale) };
    let readout = match format {
        Some(format) => format.call(slider.current()),
        None => slider.current().to_string(),
    };
    let (lo, hi) = slider.range();
    let scope = slider.scope();
    let bubbles: Vec<()> = if dragging { vec![()] } else { Vec::new() };

    rsx! {
        div { class: cn!("relative flex h-20 w-full items-end px-5 pb-5", if disabled { "pointer-events-none opacity-50" } else { "" }, class),
            div {
                "data-beui-slider": scope.clone(),
                class: cn!(
                    "relative h-2 w-full touch-none rounded-full bg-muted",
                    TOUCH_GESTURE_CLASS,
                    if disabled { "" } else { "cursor-grab active:cursor-grabbing" },
                ),
                onpointerdown: move |event| slider.on_pointer_down(&event),
                onpointermove: move |event| slider.on_pointer_move(&event),
                onpointerup: move |event| slider.end_drag(&event),
                onpointercancel: move |event| slider.end_drag(&event),
                onlostpointercapture: move |event| slider.end_drag(&event),
                div { class: "absolute inset-y-0 left-0 rounded-full bg-foreground", style: "width:{pos}%" }
                div {
                    class: "absolute top-1/2 size-5 rounded-full border-2 border-foreground bg-background shadow-sm",
                    style: "left:{pos}%;transform:translateX(-50%) translateY(-50%) scale({thumb_scale_value})",
                }
                div { class: "pointer-events-none absolute bottom-6", style: "left:{pos}%;transform:translateX(-50%)",
                    AnimatePresence {
                        for _ in bubbles {
                            Bubble { key: "{BUBBLE_KEY}", readout: readout.clone(), tilt, squash, stretch, reduce }
                        }
                    }
                }
                button {
                    "data-beui-slider-handle": scope.clone(),
                    r#type: "button",
                    role: "slider",
                    tabindex: if disabled { -1 } else { 0 },
                    "aria-label": aria_label,
                    "aria-valuemin": "{lo}",
                    "aria-valuemax": "{hi}",
                    "aria-valuenow": "{slider.current()}",
                    "aria-valuetext": slider.aria_value_text(),
                    "aria-disabled": disabled.then_some("true"),
                    class: "absolute -inset-y-5 inset-x-0 touch-none rounded-full outline-none ring-foreground/30 focus-visible:ring-4",
                    onkeydown: move |event| slider.on_key_down(&event),
                }
            }
        }
    }
}

#[component]
fn Bubble(readout: String, tilt: f64, squash: f64, stretch: f64, reduce: bool) -> Element {
    let gate = use_gate();
    let config = if reduce {
        PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(120, Ease::Out),
            tween(120, Ease::Out),
        )
    } else {
        PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0, scale: 0.4, y: 10.0 },
            motion_style! { opacity: 1.0, scale: 1.0, y: 0.0 },
            motion_style! { opacity: 0.0, scale: 0.5, y: 8.0 },
            spring(SPRING_PANEL),
            tween(120, Ease::Out),
        )
    };
    let style = use_presence_style(config).expect("static presence config");
    let css = presence_css(&style);
    let reaction = if reduce {
        String::new()
    } else {
        format!(";transform:rotate({tilt}deg) scaleX({squash}) scaleY({stretch});transform-origin:center bottom")
    };

    rsx! {
        div {
            inert: gate.inert_attr(),
            style: "{gate.style_with(&css)}{reaction}",
            class: "relative rounded-xl bg-foreground px-2.5 py-1 text-sm font-medium tabular-nums text-background shadow-md",
            "{readout}"
            span { class: "absolute -bottom-1 left-1/2 size-2.5 -translate-x-1/2 rotate-45 rounded-[3px] bg-foreground" }
        }
    }
}

// --- ruler ------------------------------------------------------------------

/// Settle spring for the snap after a flick — quick, no overshoot past the tick.
const SPRING_SNAP: dioxus_motion::prelude::Spring = dioxus_motion::prelude::Spring {
    stiffness: 500.0,
    damping: 40.0,
    mass: 0.6,
};

/// Ruler slider: the scale scrolls under a fixed needle instead of a handle
/// moving along a track.
///
/// Difference from the source: the release snaps onto the nearest tick with
/// `SPRING_SNAP`, but there is no inertia — a flick does not coast before it
/// snaps.
#[component]
pub fn RulerSlider(
    #[props(default)] value: Option<f64>,
    #[props(default)] default_value: f64,
    #[props(default)] on_value_change: Option<EventHandler<f64>>,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default = 14.0)] gap: f64,
    #[props(default = 5.0)] major_every: f64,
    #[props(default)] unit: Option<String>,
    #[props(default)] format: Option<Callback<f64, String>>,
    #[props(default)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let decimals = decimals_of(step);
    let readout = move |value: f64| format!("{value:.decimals$}");
    let value_text = format.or_else(|| {
        unit.clone()
            .map(|unit| Callback::new(move |v: f64| format!("{} {}", readout(v), unit)))
    });
    let slider = use_slider(
        slider_config(min, max, step, default_value, disabled, value_text),
        value,
        on_value_change,
    );
    let current = slider.current();
    let (lo, hi) = slider.range();
    let stride = slider.step();
    let scope = slider.scope();

    let span = round6((hi - lo) / stride);
    let whole_steps = span.floor() as usize;
    let remainder = span - whole_steps as f64;
    let max_offset = span * gap;
    let major_every = major_every.max(1.0) as usize;

    // The strip position is a plain signal while dragging (MotionHandle has no
    // setter) plus a settle motion for the snap; both fold back into `base`.
    let mut base = use_signal(|| -((current - lo) / stride) * gap);
    let mut offset = use_signal(|| 0.0f64);
    let mut settle = use_number_motion(0.0);
    let mut gesture_start = use_signal(|| None::<(i32, f64, f64)>);

    use_effect(use_reactive!(|current| {
        if gesture_start.peek().is_none() {
            base.set(-((current - lo) / stride) * gap);
            offset.set(0.0);
        }
    }));

    let strip_x = base() + offset() + f64::from(number(&settle));
    let ticks: Vec<(f64, bool, f64)> = {
        let mut ticks: Vec<(f64, bool, f64)> = (0..=whole_steps)
            .map(|i| {
                (
                    round6(lo + i as f64 * stride),
                    i % major_every == 0,
                    i as f64 * gap,
                )
            })
            .collect();
        if remainder > 0.0 {
            ticks.push((hi, true, max_offset));
        }
        ticks
    };

    let start_drag = move |event: Event<PointerData>| {
        if disabled {
            return;
        }
        event.prevent_default();
        platform::capture_pointer(&event);
        gesture_start.set(Some((
            event.pointer_id(),
            event.client_coordinates().x,
            base() + offset(),
        )));
        settle.reset();
    };
    let move_drag = move |event: Event<PointerData>| {
        let Some((id, start_x, start_base)) = *gesture_start.peek() else {
            return;
        };
        if id != event.pointer_id() || disabled {
            return;
        }
        let dx = event.client_coordinates().x - start_x;
        let next = (start_base + dx).min(0.0).max(-max_offset);
        offset.set(next - base());
    };
    let end_drag = move |event: Event<PointerData>| {
        let Some((id, _start_x, _start_base)) = *gesture_start.peek() else {
            return;
        };
        if id != event.pointer_id() {
            return;
        }
        gesture_start.set(None);
        platform::release_pointer(&event);
        let raw_value = lo + (-(base() + offset()) / gap) * stride;
        let target_value = crate::range_slider::snap_value(raw_value, lo, hi, stride);
        let target_x = -((target_value - lo) / stride) * gap;
        let current_x = base() + offset();
        let delta = target_x - current_x;
        base.set(current_x);
        offset.set(0.0);
        let mut settle_handle = settle;
        settle_handle.reset();
        if reduce || delta.abs() < 0.5 {
            base.set(target_x);
        } else {
            let _ = settle_handle.animate_to(delta as f32, spring(SPRING_SNAP));
            let mut base = base;
            spawn(async move {
                let _ = Time::delay(Duration::from_millis(500)).await;
                base.set(target_x);
                let mut settle = settle_handle;
                settle.reset();
            });
        }
        slider.commit(target_value);
    };

    rsx! {
        div {
            "data-beui-slider": scope.clone(),
            class: cn!(
                "relative w-full touch-none overflow-hidden rounded-2xl outline-none ring-foreground/30 focus-visible:ring-4",
                TOUCH_GESTURE_CLASS,
                if disabled { "pointer-events-none opacity-50" } else { "cursor-grab active:cursor-grabbing" },
                class,
            ),
            onpointerdown: start_drag,
            onpointermove: move_drag,
            onpointerup: end_drag,
            onpointercancel: end_drag,
            onlostpointercapture: end_drag,
            onkeydown: move |event| slider.on_key_down(&event),
            div { class: "pointer-events-none flex items-baseline justify-center gap-1 pt-1 pb-3",
                span { class: "text-3xl font-semibold tabular-nums text-foreground", "{readout(current)}" }
                if let Some(unit) = unit.clone() {
                    span { class: "text-sm text-muted-foreground", "{unit}" }
                }
            }
            div { class: "relative h-12 [mask-image:linear-gradient(to_right,transparent,black_18%,black_82%,transparent)]",
                div {
                    class: "absolute inset-y-0 left-1/2",
                    style: "transform:translateX({strip_x}px);margin-left:{-(gap / 2.0)}px;width:{max_offset + gap}px",
                    for (tick_value, major, tick_offset) in ticks {
                        span {
                            key: "{tick_value}",
                            class: "absolute bottom-0 flex -translate-x-1/2 flex-col items-center pb-[18px]",
                            style: "left:{tick_offset + gap / 2.0}px",
                            span { class: cn!("w-px rounded-full", if major { "h-7 bg-foreground/70" } else { "h-3.5 bg-foreground/45" }) }
                            if major {
                                span { class: "absolute bottom-0 text-[10px] tabular-nums text-muted-foreground", "{tick_value}" }
                            }
                        }
                    }
                }
                div { class: "pointer-events-none absolute bottom-5 left-1/2 -translate-x-1/2",
                    span { class: "block h-9 w-[3px] rounded-full bg-foreground" }
                }
            }
            button {
                "data-beui-slider-handle": scope.clone(),
                r#type: "button",
                role: "slider",
                tabindex: if disabled { -1 } else { 0 },
                "aria-label": aria_label,
                "aria-valuemin": "{lo}",
                "aria-valuemax": "{hi}",
                "aria-valuenow": "{current}",
                "aria-valuetext": slider.aria_value_text(),
                "aria-disabled": disabled.then_some("true"),
                class: "sr-only",
            }
        }
    }
}

fn decimals_of(step: f64) -> usize {
    let text = format!("{step}");
    match text.split('.').nth(1) {
        Some(fraction) => fraction.trim_end_matches('0').len(),
        None => 0,
    }
}

// --- inline -----------------------------------------------------------------

const STOP_COUNT: usize = 10;
const HANDLE_START: f64 = 8.0;
const HANDLE_END_INSET: f64 = 12.0;
const TEXT_INSET: f64 = 20.0;

/// An always-visible inline slider with an inset fill and a thumb that parts
/// around its labels.
///
/// Difference from the source: the thumb is a solid 4px stem rather than the
/// stem-plus-two-caps that split around the label text as it passes. The stops,
/// the drag mapping, the keyboard steps and the tick suppression over text are
/// all unchanged.
#[component]
pub fn InlineSlider(
    label: String,
    #[props(default)] value: Option<f64>,
    #[props(default)] default_value: f64,
    #[props(default)] on_value_change: Option<EventHandler<f64>>,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] format: Option<Callback<f64, String>>,
    #[props(default)] value_text: Option<Callback<f64, String>>,
    #[props(default = true)] show_ticks: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let stride = if step > 0.0 { step } else { 1.0 };
    let format = format.unwrap_or_else(|| Callback::new(|v: f64| v.to_string()));
    let slider = use_slider(
        slider_config(
            min,
            max,
            stride,
            default_value,
            disabled,
            value_text.or(Some(format)),
        ),
        value,
        on_value_change,
    );
    let current = slider.current();
    let (lo, hi) = slider.range();

    // (width, left, labelWidth, readoutWidth)
    let mut geometry = use_signal(|| (292.0f64, 0.0f64, 22.0f64, 24.0f64));
    let mut dragging = use_signal(|| false);
    let mut thumb_scale = use_number_motion(1.0);
    let mut gesture = use_signal(|| None::<(i32, f64, f64)>); // id, left, offset
    let mut drag_x = use_signal(|| 0.0f64);
    let mut settling = use_number_motion(0.0);

    let (width, left, label_width, readout_width) = geometry();
    let end_x = HANDLE_START.max(width - HANDLE_END_INSET);
    let stops = Rc::new(inline_stops(lo, hi, stride, end_x));
    let resting_x = map_stops(&stops, current, 0, 1);

    use_effect(use_reactive!(|resting_x| {
        if gesture.peek().is_none() {
            drag_x.set(resting_x);
        }
    }));

    let dragging_now = dragging();
    use_effect(use_reactive!(|dragging_now| {
        if !reduce {
            animate_number(
                &mut thumb_scale,
                if dragging_now { 1.35 } else { 1.0 },
                spring(SPRING_PRESS),
            );
        }
    }));

    let handle_x = drag_x() + f64::from(number(&settling));
    let fill_right = if handle_x >= end_x {
        width - 2.0
    } else {
        handle_x + 8.0
    };
    let fill_x = fill_right - width + 2.0;
    let label_bounds = (TEXT_INSET, TEXT_INSET + label_width);
    let readout_bounds = (width - TEXT_INSET - readout_width, width - TEXT_INSET);
    let overlaps = |x: f64, bounds: (f64, f64)| x + 2.0 >= bounds.0 && x - 2.0 <= bounds.1;
    let ticks: Vec<f64> = if show_ticks {
        stops
            .iter()
            .map(|stop| stop.1)
            .filter(|x| !overlaps(*x, label_bounds) && !overlaps(*x, readout_bounds))
            .collect()
    } else {
        Vec::new()
    };
    let scope = slider.scope();
    let stops_move = stops.clone();
    let stops_up = stops.clone();
    let stops_key = stops.clone();

    rsx! {
        div {
            "data-beui-slider": scope.clone(),
            class: cn!(
                "relative h-10 w-full touch-none select-none overflow-hidden rounded-lg bg-muted",
                TOUCH_GESTURE_CLASS,
                if disabled { "pointer-events-none opacity-50" } else { "cursor-grab active:cursor-grabbing" },
                class,
            ),
            onmounted: move |data| {
                spawn(async move {
                    if let Some(rect) = platform::client_rect(&data).await {
                        let (_, _, label, readout) = *geometry.peek();
                        geometry.set((rect.width, rect.left, label, readout));
                    }
                });
            },
            onpointerdown: move |event| {
                if disabled || gesture.peek().is_some() {
                    return;
                }
                event.prevent_default();
                let pointer_x = event.client_coordinates().x - left;
                let thumb = handle_x;
                let offset = if (pointer_x - thumb - 2.0).abs() <= 12.0 { pointer_x - thumb } else { 2.0 };
                platform::capture_pointer(&event);
                gesture.set(Some((event.pointer_id(), left, offset)));
                dragging.set(true);
            },
            onpointermove: move |event| {
                let Some((id, left, offset)) = *gesture.peek() else { return };
                if id != event.pointer_id() || disabled {
                    return;
                }
                let x = (event.client_coordinates().x - left - offset).min(end_x).max(HANDLE_START);
                drag_x.set(x);
                slider.commit(map_stops(&stops_move, x, 1, 0));
            },
            onpointerup: move |event| {
                let Some((id, _, _)) = *gesture.peek() else { return };
                if id != event.pointer_id() {
                    return;
                }
                gesture.set(None);
                dragging.set(false);
                platform::release_pointer(&event);
                let stop = nearest_stop(&stops_up, drag_x());
                slider.commit(stop.0);
                let target = stop.1;
                if reduce || (target - drag_x()).abs() < 0.5 {
                    drag_x.set(target);
                    settling.reset();
                } else {
                    let delta = (target - drag_x()) as f32;
                    drag_x.set(target);
                    let mut settle = settling;
                    settle.reset();
                    let _ = settle.animate_to(delta, spring(SPRING_GLIDE));
                    let mut drag_x = drag_x;
                    spawn(async move {
                        let _ = Time::delay(Duration::from_millis(500)).await;
                        drag_x.set(target);
                        let mut settle = settle;
                        settle.reset();
                    });
                }
            },
            onpointercancel: move |_| {
                gesture.set(None);
                dragging.set(false);
            },
            onlostpointercapture: move |_| {
                gesture.set(None);
                dragging.set(false);
            },
            div { "aria-hidden": "true", class: "pointer-events-none absolute inset-x-[2px] inset-y-0 overflow-hidden rounded-lg",
                div { class: "absolute inset-0 rounded-lg bg-foreground/15", style: "transform:translateX({fill_x}px)" }
            }
            div { "aria-hidden": "true", class: "pointer-events-none absolute inset-0 text-foreground",
                span {
                    class: "absolute left-5 top-1/2 max-w-[40%] -translate-y-1/2 truncate text-sm font-medium leading-5",
                    onmounted: move |data| {
                        spawn(async move {
                            if let Some(rect) = platform::client_rect(&data).await {
                                let (width, left, _, readout) = *geometry.peek();
                                geometry.set((width, left, rect.width, readout));
                            }
                        });
                    },
                    "{label}"
                }
                span {
                    class: "absolute right-5 top-1/2 max-w-[40%] -translate-y-1/2 truncate text-[13px] font-semibold leading-[18px] tracking-tight tabular-nums",
                    onmounted: move |data| {
                        spawn(async move {
                            if let Some(rect) = platform::client_rect(&data).await {
                                let (width, left, label, _) = *geometry.peek();
                                geometry.set((width, left, label, rect.width));
                            }
                        });
                    },
                    "{format.call(current)}"
                }
                for tick in ticks {
                    span {
                        key: "{tick}",
                        class: "absolute top-1/2 size-1 -translate-y-1/2 rounded-full bg-foreground/25",
                        style: "left:{tick}px",
                    }
                }
            }
            div { "aria-hidden": "true",
                class: "pointer-events-none absolute left-0 top-2 h-6 w-1 text-foreground",
                style: "transform:translateX({handle_x}px) scaleY({number(&thumb_scale)})",
                span { class: "absolute inset-y-0 w-1 rounded-full bg-current" }
            }
            button {
                "data-beui-slider-handle": scope.clone(),
                r#type: "button",
                role: "slider",
                tabindex: if disabled { -1 } else { 0 },
                "aria-label": aria_label.unwrap_or_else(|| label.clone()),
                "aria-valuemin": "{lo}",
                "aria-valuemax": "{hi}",
                "aria-valuenow": "{current}",
                "aria-valuetext": slider.aria_value_text(),
                "aria-disabled": disabled.then_some("true"),
                class: "absolute inset-0 touch-none rounded-lg border-0 outline-none",
                onkeydown: move |event| {
                    if disabled {
                        return;
                    }
                    let next = match event.key() {
                        Key::ArrowRight | Key::ArrowUp => stops_key.iter().find(|s| s.0 > current).map(|s| s.0).unwrap_or(hi),
                        Key::ArrowLeft | Key::ArrowDown => stops_key.iter().rev().find(|s| s.0 < current).map(|s| s.0).unwrap_or(lo),
                        Key::Home => lo,
                        Key::End => hi,
                        Key::PageUp => hi,
                        Key::PageDown => lo,
                        _ => return,
                    };
                    event.prevent_default();
                    slider.commit(next);
                },
            }
        }
    }
}

/// `(value, x)` for the ten evenly spaced stops, deduplicated and snapped.
fn inline_stops(min: f64, max: f64, step: f64, end_x: f64) -> Vec<(f64, f64)> {
    let mut values: Vec<f64> = Vec::new();
    for index in 0..STOP_COUNT {
        let raw = min + (index as f64 / (STOP_COUNT - 1) as f64) * (max - min);
        let snapped = crate::range_slider::snap_value(raw, min, max, step);
        if !values.iter().any(|v| (*v - snapped).abs() < 1.0e-9) {
            values.push(snapped);
        }
    }
    let count = values.len();
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let x = if count == 1 {
                HANDLE_START
            } else {
                HANDLE_START + (index as f64 / (count - 1) as f64) * (end_x - HANDLE_START)
            };
            (value, x)
        })
        .collect()
}

/// `mapBetweenStops`: linear between the two stops bracketing `point`, reading
/// field `from` and returning field `to` (`0` is value, `1` is x).
fn map_stops(stops: &[(f64, f64)], point: f64, from: usize, to: usize) -> f64 {
    if stops.is_empty() {
        return 0.0;
    }
    let at = |stop: (f64, f64), index: usize| if index == 0 { stop.0 } else { stop.1 };
    let upper_index = stops
        .iter()
        .position(|stop| at(*stop, from) >= point)
        .unwrap_or(stops.len() - 1);
    let upper = stops[upper_index];
    let lower = stops[upper_index.saturating_sub(1)];
    let (lower_from, upper_from) = (at(lower, from), at(upper, from));
    if (lower_from - upper_from).abs() < f64::EPSILON {
        return at(upper, to);
    }
    at(lower, to)
        + ((point - lower_from) / (upper_from - lower_from)) * (at(upper, to) - at(lower, to))
}

fn nearest_stop(stops: &[(f64, f64)], x: f64) -> (f64, f64) {
    stops
        .iter()
        .copied()
        .reduce(|nearest, stop| {
            if (stop.1 - x).abs() < (nearest.1 - x).abs() {
                stop
            } else {
                nearest
            }
        })
        .unwrap_or((0.0, HANDLE_START))
}
