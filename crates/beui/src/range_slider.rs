//! `motion/range-slider.tsx` — the base slider.
//!
//! The value comes from `beui-core::hooks::use_slider` (snapping, pointer
//! capture, keyboard, ARIA). The visuals are a spring-smoothed position: the
//! thumb and the fill both follow `percent` through `SPRING_GLIDE`, so the fill
//! keeps gliding for a beat after a click jumps the value, and the thumb scales
//! to `1.35` while dragging on a bouncy `{500, 14, 0.7}`.
//!
//! The track is measured on mount because the thumb's travel is `8 + (width -
//! 20) * p/100` px; the default `292` is the source's own initial guess.

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::ease::SPRING_GLIDE;
use beui_core::hooks::{use_reduced_motion, use_slider, SliderConfig};
use beui_core::platform;
use beui_core::touch::TOUCH_GESTURE_CLASS;

use crate::motion::{animate_number, number, spring, use_number_motion};

/// Bouncy grab feedback for the thumb scale only.
const SPRING_BOUNCY: dioxus_motion::prelude::Spring = dioxus_motion::prelude::Spring {
    stiffness: 500.0,
    damping: 14.0,
    mass: 0.7,
};

/// The one place the base and every variant configure their value.
pub fn slider_config(
    min: f64,
    max: f64,
    step: f64,
    default_value: f64,
    disabled: bool,
    value_text: Option<Callback<f64, String>>,
) -> SliderConfig {
    SliderConfig {
        min,
        max,
        step,
        default_value,
        disabled,
        value_text,
    }
}

/// `Number(x.toFixed(6))` for the tick grid.
pub fn round6(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

/// `snapSliderValue` from `lib/hooks/use-slider`, re-exported for the variants.
pub use beui_core::hooks::snap_slider_value as snap_value;

/// A spring that follows `percent` — `useSpring(target, SPRING_GLIDE)`.
pub fn use_smooth_percent(percent: f64, reduce: bool) -> f64 {
    let mut smooth = use_number_motion(percent as f32);
    use_effect(use_reactive!(|percent| {
        if !reduce {
            animate_number(&mut smooth, percent as f32, spring(SPRING_GLIDE));
        }
    }));
    if reduce {
        percent
    } else {
        f64::from(number(&smooth))
    }
}

/// The tick values for `showTicks`, following the source's floor-not-round rule.
// The source writes `!(steps > 0)`, so a NaN range falls into the empty branch
// rather than drawing a NaN tick; the negated form is the point.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn tick_values(min: f64, max: f64, step: f64, show_ticks: bool) -> Vec<f64> {
    if !show_ticks || step <= 0.0 {
        return Vec::new();
    }
    let steps = (((max - min) / step) * 1e6).round() / 1e6;
    let steps = steps.floor();
    if !(steps > 0.0) || steps > 50.0 {
        return Vec::new();
    }
    (0..=(steps as usize))
        .map(|i| round6(min + i as f64 * step))
        .collect()
}

#[component]
pub fn RangeSlider(
    #[props(default)] value: Option<f64>,
    #[props(default)] default_value: f64,
    #[props(default)] on_value_change: Option<EventHandler<f64>>,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default = true)] show_ticks: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let slider = use_slider(
        slider_config(min, max, step, default_value, disabled, None),
        value,
        on_value_change,
    );
    let percent = slider.percent();
    let pos = use_smooth_percent(percent, reduce);

    let mut track_width = use_signal(|| 292.0f64);
    let mut thumb_scale = use_number_motion(1.0);
    let dragging = slider.dragging();
    use_effect(use_reactive!(|dragging| {
        if !reduce {
            animate_number(
                &mut thumb_scale,
                if dragging { 1.35 } else { 1.0 },
                spring(SPRING_BOUNCY),
            );
        }
    }));

    let width = track_width();
    let thumb_x = 8.0 + (width - 20.0).max(0.0) * pos / 100.0;
    let fill_x = if pos >= 100.0 {
        "0%".to_string()
    } else {
        format!("calc({}% + {}px)", pos - 100.0, 14.0 - 0.16 * pos)
    };
    let scale_y = if reduce { 1.0 } else { number(&thumb_scale) };
    let ticks = tick_values(min, max, step, show_ticks);
    let (lo, hi) = slider.range();
    let scope = slider.scope();

    rsx! {
        div {
            "data-beui-slider": scope.clone(),
            class: cn!(
                "relative flex h-10 w-full touch-none items-center overflow-hidden rounded-lg bg-muted",
                TOUCH_GESTURE_CLASS,
                if disabled { "pointer-events-none opacity-50" } else { "cursor-grab active:cursor-grabbing" },
                class,
            ),
            onmounted: move |data| {
                spawn(async move {
                    if let Some(rect) = platform::client_rect(&data).await {
                        track_width.set(rect.width);
                    }
                });
            },
            onpointerdown: move |event| slider.on_pointer_down(&event),
            onpointermove: move |event| slider.on_pointer_move(&event),
            onpointerup: move |event| slider.end_drag(&event),
            onpointercancel: move |event| slider.end_drag(&event),
            onlostpointercapture: move |event| slider.end_drag(&event),
            div { "aria-hidden": "true", class: "pointer-events-none absolute inset-x-[2px] inset-y-0 overflow-hidden rounded-lg",
                div {
                    class: "absolute inset-0 rounded-lg bg-foreground/15",
                    style: "transform:translateX({fill_x})",
                }
            }
            div { class: "pointer-events-none absolute inset-x-[10px] inset-y-0",
                for tick in ticks {
                    {
                        let tp = if max > min { ((tick - min) / (max - min)) * 100.0 } else { 0.0 };
                        rsx! {
                            span {
                                key: "{tick}",
                                class: "absolute top-1/2 size-1 -translate-x-1/2 -translate-y-1/2 rounded-full bg-foreground/25",
                                style: "left:{tp}%",
                            }
                        }
                    }
                }
            }
            div {
                "data-beui-slider-handle": scope.clone(),
                role: "slider",
                tabindex: if disabled { -1 } else { 0 },
                "aria-label": aria_label,
                "aria-valuemin": "{lo}",
                "aria-valuemax": "{hi}",
                "aria-valuenow": "{slider.current()}",
                "aria-valuetext": slider.aria_value_text(),
                "aria-disabled": disabled.then_some("true"),
                class: "absolute left-0 top-1/2 h-6 w-1 rounded-full bg-foreground outline-none ring-inset ring-foreground/30 focus-visible:ring-4",
                style: "transform:translateX({thumb_x}px) translateY(-50%) scaleY({scale_y})",
                onkeydown: move |event| slider.on_key_down(&event),
            }
        }
    }
}
