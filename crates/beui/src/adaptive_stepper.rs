//! `motion/adaptive-stepper.tsx` — the adaptive quantity stepper.
//!
//! The stepper re-lays itself as it reaches its bounds: a button that would do
//! nothing drifts away and fades out while the value field widens to take the
//! space, all on the source's springy `cubic-bezier(0.22, 1.3, 0.71, 1)`.
//!
//! Difference from the source: beUI runs the buttons and the value through the
//! `Liquid` gooey filter (`feGaussianBlur` + high `feColorMatrix` contrast), so
//! the shapes merge as they move. That filter is its own component (`liquid.tsx`,
//! outside this lane), so the port animates plain `left`/`width` instead: the
//! geometry, timing and value roll are the same, the gooey join is not.

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::ease::SPRING_PRESS;
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;
use beui_core::presence::{
    presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

use crate::icons::{IconMinus, IconPlus};
use crate::motion::{animate_number, number, spring, tween, use_number_motion, Ease, MotionStyle};

/// The deliberately elastic separation curve from the liquid email reference.
const LIQUID_EASE: &str = "cubic-bezier(0.22, 1.3, 0.71, 1)";
const LIQUID_MS: u64 = 600;

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

fn clean_number(value: f64) -> f64 {
    (value * 1e10).round() / 1e10
}

/// `nextStep`: walk one step on the grid, keeping the phase of `min`.
fn next_step(value: f64, direction: i8, min: f64, max: f64, step: f64) -> f64 {
    if direction == 1 {
        let next_index = ((value - min) / step + 1.0e-10).floor() + 1.0;
        clean_number((min + next_index * step).min(max))
    } else {
        let previous_index = ((value - min) / step - 1.0e-10).ceil() - 1.0;
        clean_number((min + previous_index * step).max(min))
    }
}

#[derive(Clone, Copy)]
struct StepperContext {
    id: u32,
    value: Signal<f64>,
    direction: Signal<i8>,
    lower: f64,
    upper: f64,
    disabled: bool,
    reduce: bool,
    on_step: EventHandler<(i8, bool)>,
}

impl StepperContext {
    fn value(&self) -> f64 {
        (self.value)()
    }

    fn at_min(&self) -> bool {
        self.value() <= self.lower
    }

    fn at_max(&self) -> bool {
        self.value() >= self.upper
    }
}

#[component]
pub fn AdaptiveStepper(
    #[props(default)] value: Option<f64>,
    #[props(default)] default_value: f64,
    #[props(default)] on_value_change: Option<EventHandler<f64>>,
    #[props(default = 0.0)] min: f64,
    #[props(default = 10.0)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default)] name: Option<String>,
    #[props(default)] format_value_text: Option<Callback<f64, String>>,
    #[props(default = "Quantity".to_string())] aria_label: String,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let id = use_hook(platform::next_scope_id);
    let lower = if min.is_finite() { min } else { 0.0 };
    let supplied_max = if max.is_finite() { max } else { lower };
    let upper = if supplied_max > lower {
        supplied_max
    } else {
        lower
    };
    let stride = if step.is_finite() && step > 0.0 {
        step
    } else {
        1.0
    };

    let internal = use_signal(|| clamp(default_value, lower, upper));
    let controlled = value.is_some();
    let direction = use_signal(|| 0i8);

    use_effect(use_reactive!(|value| {
        if let Some(value) = value {
            let mut internal = internal;
            if *internal.peek() != value {
                internal.set(clamp(value, lower, upper));
            }
        }
    }));

    let on_step = EventHandler::new(move |(dir, restore_focus): (i8, bool)| {
        if disabled {
            return;
        }
        let current = if let Some(value) = value {
            value
        } else {
            internal()
        };
        let current = clamp(current, lower, upper);
        let next = next_step(current, dir, lower, upper, stride);
        if next == current {
            return;
        }
        if !controlled {
            let mut internal = internal;
            internal.set(next);
        }
        if let Some(on_value_change) = on_value_change {
            on_value_change.call(next);
        }
        let mut direction = direction;
        direction.set(dir);
        if restore_focus {
            // Reaching a bound disables the button that got you there; move
            // focus to the one that still works.
            let decrement = next >= upper;
            let selector = if decrement {
                format!("[data-stepper-decrement=\"{id}\"]")
            } else {
                format!("[data-stepper-increment=\"{id}\"]")
            };
            platform::focus_selector(&selector, true);
        }
    });

    use_context_provider(|| StepperContext {
        id,
        value: internal,
        direction,
        lower,
        upper,
        disabled,
        reduce,
        on_step,
    });

    let value_text = match format_value_text {
        Some(format) => format.call(clamp(internal(), lower, upper)),
        None => internal().to_string(),
    };

    rsx! {
        fieldset {
            disabled,
            class: cn!("relative isolate m-0 inline-block h-12 w-[13.5rem] border-0 p-0", class),
            legend { class: "sr-only", "{aria_label}. Current value: {value_text}" }
            div { class: "relative size-full",
                {children}
            }
            if let Some(name) = name {
                input { r#type: "hidden", name, value: "{internal()}", }
            }
        }
    }
}

#[component]
pub fn AdaptiveStepperDecrement(#[props(default)] class: Option<String>) -> Element {
    rsx! { StepperAction { direction: -1i8, class: class } }
}

#[component]
pub fn AdaptiveStepperIncrement(#[props(default)] class: Option<String>) -> Element {
    rsx! { StepperAction { direction: 1i8, class: class } }
}

#[component]
fn StepperAction(direction: i8, #[props(default)] class: Option<String>) -> Element {
    let context = use_context::<StepperContext>();
    let hidden = if direction == -1 {
        context.at_min()
    } else {
        context.at_max()
    };
    let x = if direction == -1 {
        if hidden {
            32.0
        } else {
            0.0
        }
    } else if hidden {
        136.0
    } else {
        168.0
    };
    let mut scale = use_number_motion(1.0);

    let id = context.id;
    let label = if direction == -1 {
        "Decrease value"
    } else {
        "Increase value"
    };
    let icon_opacity = if hidden { 0.0 } else { 1.0 };
    let icon_blur = if hidden { 2.0 } else { 0.0 };
    let scale_value = number(&scale);

    rsx! {
        div {
            class: "absolute top-0 left-0 h-12 w-12",
            style: "transform:translateX({x}px);transition:transform {LIQUID_MS}ms {LIQUID_EASE}",
            button {
                r#type: "button",
                "data-stepper-decrement": (direction == -1).then(|| id.to_string()),
                "data-stepper-increment": (direction == 1).then(|| id.to_string()),
                "aria-label": label,
                "aria-hidden": hidden.then_some("true"),
                tabindex: if hidden { -1 } else { 0 },
                disabled: context.disabled || hidden,
                class: cn!(
                    "grid size-full place-items-center rounded-full border border-transparent bg-transparent bg-clip-padding text-foreground outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none",
                    if hidden { "hover:bg-transparent" } else { "" },
                    if context.disabled { "opacity-50" } else { "" },
                    class,
                ),
                style: "transform:scale({scale_value})",
                onpointerdown: move |_| {
                    if !context.reduce && !context.disabled && !hidden {
                        animate_number(&mut scale, 0.94, spring(SPRING_PRESS));
                    }
                },
                onpointerup: move |_| {
                    animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
                },
                onpointercancel: move |_| {
                    animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
                },
                onkeydown: move |event| {
                    let key = event.key();
                    if key == Key::Enter || key == Key::Character(" ".to_string()) {
                        event.prevent_default();
                        context.on_step.call((direction, true));
                    }
                },
                onclick: move |_| context.on_step.call((direction, false)),
                span {
                    "aria-hidden": "true",
                    style: "opacity:{icon_opacity};filter:blur({icon_blur}px);transition:opacity 150ms {LIQUID_EASE}, filter 150ms {LIQUID_EASE}",
                    if direction == -1 {
                        IconMinus { class: "size-5", stroke_width: "2.5" }
                    } else {
                        IconPlus { class: "size-5", stroke_width: "2.5" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AdaptiveStepperValue(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<StepperContext>();
    let value = context.value();
    let (at_min, at_max) = (context.at_min(), context.at_max());
    let (x, width) = if at_min && at_max {
        (0.0, 216.0)
    } else if at_min {
        (0.0, 152.0)
    } else if at_max {
        (64.0, 152.0)
    } else {
        (64.0, 88.0)
    };
    let value_text = value.to_string();
    let rolls: Vec<f64> = if context.reduce {
        Vec::new()
    } else {
        vec![value]
    };

    rsx! {
        output {
            "aria-live": "polite",
            "aria-atomic": "true",
            class: cn!(
                "absolute top-0 left-0 flex h-12 min-w-0 items-center justify-center overflow-hidden rounded-full border border-transparent bg-transparent bg-clip-padding px-4 text-lg font-semibold tabular-nums text-foreground",
                class,
            ),
            style: "transform:translateX({x}px);width:{width}px;transition:transform {LIQUID_MS}ms {LIQUID_EASE}, width {LIQUID_MS}ms {LIQUID_EASE}",
            span { class: "sr-only", "{value_text}" }
            span {
                "aria-hidden": "true",
                class: "relative grid min-h-[1.1em] min-w-[1ch] place-items-center overflow-hidden leading-none",
                if context.reduce {
                    span { class: "col-start-1 row-start-1", "{value}" }
                } else {
                    AnimatePresence { mode: dioxus_motion::prelude::PresenceMode::PopLayout, initial: false,
                        for value in rolls {
                            ValueRoll { key: "{value}", value, direction: (context.direction)() }
                        }
                    }
                }
            }
        }
    }
}

fn roll_style(opacity: f32, percent: f32, blur: f32) -> MotionStyle {
    let mut style = MotionStyle {
        opacity,
        ..Default::default()
    };
    style.add_css_property("filter", format!("blur({blur}px)"));
    style.add_css_property("transform", format!("translateY({percent}%)"));
    style
}

#[component]
fn ValueRoll(value: f64, direction: i8) -> Element {
    let gate = use_gate();
    let distance = f64::from(direction) * 32.0;
    let config = PresenceConfig::with_transitions(
        roll_style(0.35, distance as f32, 2.0),
        roll_style(1.0, 0.0, 0.0),
        roll_style(0.0, -distance as f32, 2.0),
        tween(180, Ease::Out),
        tween(120, Ease::Out),
    );
    let style = use_presence_style(config).expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        span {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "col-start-1 row-start-1",
            "{value}"
        }
    }
}
