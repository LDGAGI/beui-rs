//! `motion/color-selector.tsx` — a radio group of color swatches.
//!
//! The selection ring is a `layoutId`: exactly one item renders it, and when the
//! selection moves the next item's ring mounts where the previous one was last
//! measured and glides into place. That is `beui-core::flip`'s shared-layout
//! registry (AGENTS rule 3).
//!
//! Difference from the source: beUI bends the glide along a low arc with
//! motion's `arc({ strength: -0.2 })`; `beui-core::flip` interpolates the
//! transform directly, so the hop is a straight line.

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::ease::{SPRING_LAYOUT, SPRING_PRESS};
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;

use crate::motion::{animate_number, number, scale_style, spring, use_number_motion};

#[derive(Clone, PartialEq)]
struct ColorSelectorContext {
    /// The group's current value. A controlled group mirrors the prop through
    /// the effect in [`ColorSelector`]; an uncontrolled one owns it.
    value: Signal<String>,
    controlled: bool,
    on_value_change: Option<EventHandler<String>>,
    name: String,
    disabled: bool,
    required: bool,
}

/// A single color choice. Give it a [`ColorSelectorLabel`] or an `aria-label`.
#[component]
pub fn ColorSelector(
    #[props(default)] value: Option<String>,
    #[props(default)] default_value: String,
    #[props(default)] on_value_change: Option<EventHandler<String>>,
    #[props(default)] name: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] required: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let auto_name = use_hook(|| format!("beui-color-{}", platform::next_scope_id()));
    let internal = use_signal(|| default_value);
    let controlled = value.is_some();

    use_effect(use_reactive!(|value| {
        if let Some(value) = value {
            let mut internal = internal;
            if internal.peek().as_str() != value.as_str() {
                internal.set(value);
            }
        }
    }));

    use_context_provider(|| ColorSelectorContext {
        value: internal,
        controlled,
        on_value_change,
        name: name.clone().unwrap_or(auto_name),
        disabled,
        required,
    });

    rsx! {
        fieldset {
            disabled,
            class: cn!("min-w-0 border-0 p-0", class),
            {children}
        }
    }
}

#[component]
pub fn ColorSelectorLabel(#[props(default)] class: Option<String>, children: Element) -> Element {
    rsx! {
        legend { class: cn!("mb-3 p-0 text-sm font-medium text-muted-foreground", class), {children} }
    }
}

#[component]
pub fn ColorSelectorList(#[props(default)] class: Option<String>, children: Element) -> Element {
    rsx! {
        div { class: cn!("flex flex-wrap items-center gap-3 p-1", class), {children} }
    }
}

#[component]
pub fn ColorSelectorItem(
    value: String,
    color: String,
    label: String,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let context = use_context::<ColorSelectorContext>();
    let reduce = use_reduced_motion()();
    let mut scale = use_number_motion(1.0);
    let selected = (context.value)() == value;
    let unavailable = disabled || context.disabled;
    let item_value = value.clone();

    rsx! {
        label {
            tabindex: "-1",
            class: cn!(
                "relative inline-flex shrink-0 rounded-full",
                if unavailable { "cursor-not-allowed opacity-40" } else { "cursor-pointer" },
            ),
            style: scale_style(number(&scale)),
            onpointerdown: move |_| {
                if !unavailable && !reduce {
                    animate_number(&mut scale, 0.94, spring(SPRING_PRESS));
                }
            },
            onpointerup: move |_| {
                if !unavailable && !reduce {
                    animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
                }
            },
            onpointercancel: move |_| {
                animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
            },
            input {
                r#type: "radio",
                name: context.name.clone(),
                value: value.clone(),
                checked: selected,
                disabled: unavailable,
                required: context.required,
                "aria-label": label,
                class: "peer sr-only",
                onchange: move |_| {
                    if disabled {
                        return;
                    }
                    if !context.controlled {
                        let mut current = context.value;
                        current.set(item_value.clone());
                    }
                    if let Some(on_value_change) = context.on_value_change {
                        on_value_change.call(item_value.clone());
                    }
                },
            }
            span {
                "aria-hidden": "true",
                class: cn!(
                    "relative flex size-11 items-center justify-center rounded-full border border-foreground/10 bg-muted/60",
                    "peer-focus-visible:outline-2 peer-focus-visible:outline-offset-4 peer-focus-visible:outline-foreground",
                    class,
                ),
                if selected {
                    ColorRing { color: color.clone() }
                }
                span { class: "size-5 rounded-full border border-black/5", style: "background-color:{color}" }
            }
        }
    }
}

#[component]
fn ColorRing(color: String) -> Element {
    let reduce = use_reduced_motion()();
    let mut options = FlipOptions::default().layout_id("color-selection");
    options.transition = if reduce {
        crate::motion::tween(0, crate::motion::Ease::Out)
    } else {
        spring(SPRING_LAYOUT)
    };
    let flip = use_flip(true, options);

    rsx! {
        span {
            "data-beui-flip": flip.scope(),
            class: "pointer-events-none absolute -inset-1 rounded-full border-2",
            style: flip.style_with(&format!(
                "border-color:color-mix(in srgb, {color} 65%, transparent)"
            )),
        }
    }
}
