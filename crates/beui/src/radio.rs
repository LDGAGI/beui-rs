//! `motion/radio.tsx` — the radio group and its items.
//!
//! The press is `whileTap` at `0.92` on `SPRING_PRESS`. The dot is a
//! `layoutId`: exactly one item renders it, and when the selection moves the
//! dot does not jump — the next item's dot mounts where the previous one was
//! last measured and glides into place. That is `beui-core::flip`'s shared-layout
//! registry (AGENTS rule 3), not a CSS transition.

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::ease::{SPRING_LAYOUT, SPRING_PRESS};
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;

use crate::motion::{animate_number, number, scale_style, spring, tween, use_number_motion, Ease};

#[derive(Clone)]
struct RadioContext {
    /// The group's current value. Controlled groups keep it in sync with the
    /// prop through the effect in [`RadioGroup`]; uncontrolled groups own it.
    value: Signal<String>,
    controlled: bool,
    on_value_change: Option<EventHandler<String>>,
    layout_id: String,
}

/// `vertical` (default) or `horizontal`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum RadioOrientation {
    #[default]
    Vertical,
    Horizontal,
}

#[component]
pub fn RadioGroup(
    #[props(default)] value: Option<String>,
    #[props(default = String::new())] default_value: String,
    #[props(default)] on_value_change: Option<EventHandler<String>>,
    #[props(default)] orientation: RadioOrientation,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let internal = use_signal(|| default_value);
    let layout_id = use_hook(|| format!("beui-radio-{}", platform::next_scope_id()));

    // A controlled group mirrors the prop; an uncontrolled one owns the value.
    let controlled = value.is_some();
    use_effect(use_reactive!(|value| {
        if let Some(value) = value {
            let mut internal = internal;
            if internal.peek().as_str() != value.as_str() {
                internal.set(value);
            }
        }
    }));

    use_context_provider(|| RadioContext {
        value: internal,
        controlled,
        on_value_change,
        layout_id,
    });

    rsx! {
        div {
            role: "radiogroup",
            class: cn!(
                "flex gap-3",
                match orientation {
                    RadioOrientation::Vertical => "flex-col",
                    RadioOrientation::Horizontal => "flex-row flex-wrap",
                },
                class,
            ),
            {children}
        }
    }
}

#[component]
pub fn RadioGroupItem(
    value: String,
    #[props(default)] label: Option<String>,
    #[props(default)] description: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] id: Option<String>,
) -> Element {
    let context = use_context::<RadioContext>();
    let auto_id = use_hook(|| format!("beui-radio-item-{}", platform::next_scope_id()));
    let id = id.unwrap_or(auto_id);
    let reduce = use_reduced_motion();
    let mut scale = use_number_motion(1.0);

    let selected = (context.value)() == value;
    let description_id = description.as_ref().map(|_| format!("{id}-description"));
    let label_id = if description.is_some() && label.is_some() {
        Some(format!("{id}-label"))
    } else {
        None
    };

    let item_value = value.clone();
    let on_value_change = context.on_value_change;

    let release = move |_event: Event<PointerData>| {
        if !disabled && !reduce() {
            animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
        }
    };

    rsx! {
        label {
            r#for: id.clone(),
            class: cn!(
                "inline-flex gap-3",
                if description.is_some() { "items-start" } else { "items-center" },
                if disabled { "cursor-not-allowed" } else { "cursor-pointer" },
                class,
            ),
            button {
                id: id.clone(),
                r#type: "button",
                role: "radio",
                "aria-checked": if selected { "true" } else { "false" },
                "aria-labelledby": label_id.clone(),
                "aria-describedby": description_id.clone(),
                disabled,
                "data-state": if selected { "checked" } else { "unchecked" },
                style: scale_style(number(&scale)),
                class: cn!(
                    "relative inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full border-2 outline-none transition-colors duration-200",
                    "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    "disabled:cursor-not-allowed disabled:opacity-60",
                    if selected { "border-primary" } else { "border-muted-foreground/50 hover:border-muted-foreground" },
                ),
                onpointerdown: move |_| {
                    if !disabled && !reduce() {
                        animate_number(&mut scale, 0.92, spring(SPRING_PRESS));
                    }
                },
                onpointerup: release,
                onpointercancel: release,
                onclick: move |_| {
                    if disabled {
                        return;
                    }
                    if !context.controlled {
                        let mut current = context.value;
                        current.set(item_value.clone());
                    }
                    if let Some(on_value_change) = on_value_change {
                        on_value_change.call(item_value.clone());
                    }
                },
                if selected {
                    RadioDot { layout_id: context.layout_id.clone() }
                }
            }
            if label.is_some() || description.is_some() {
                span {
                    class: cn!(
                        "flex min-w-0 flex-col gap-0.5 select-none",
                        if disabled { "opacity-60" } else { "" },
                    ),
                    if let Some(label) = label.clone() {
                        span {
                            id: label_id.clone(),
                            class: cn!("text-sm text-foreground", if description.is_some() { "leading-4" } else { "" }),
                            "{label}"
                        }
                    }
                    if let Some(description) = description.clone() {
                        span { id: description_id.clone(), class: "text-xs text-muted-foreground", "{description}" }
                    }
                }
            }
        }
    }
}

/// The selected dot. It mounts inside the selected item and hands its identity
/// to the flip registry, so the next item's dot starts from this one's box.
#[component]
fn RadioDot(layout_id: String) -> Element {
    let reduce = use_reduced_motion();
    let mut options = FlipOptions::default().layout_id(layout_id);
    options.transition = if reduce() {
        tween(0, Ease::Out)
    } else {
        spring(SPRING_LAYOUT)
    };
    let flip = use_flip(true, options);

    rsx! {
        span {
            "data-beui-flip": flip.scope(),
            class: "absolute inset-1 rounded-full bg-primary",
            style: flip.style_with(""),
        }
    }
}
