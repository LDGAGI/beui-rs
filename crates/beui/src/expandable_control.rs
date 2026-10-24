//! `motion/expandable-control.tsx` — a pill button and a chip whose labels
//! expand and collapse while the surface keeps its identity.
//!
//! Motion does this with `layout` (a transform driving the box) plus nested
//! projection for the content; that projection is a known gap here. Instead the
//! label's own `width` springs from 0 to its measured natural width, so the
//! transparent surface grows with it and there is nothing to counter-scale.
//! `CONTINUITY_SPRING` (`{stiffness: 220, damping: 17, mass: 0.85}`) is copied
//! verbatim, and the label fades in from a 4px blur.

use crate::prelude::*;
use crate::rect::{use_element_rect, ElementRect};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

/// `CONTINUITY_SPRING`, verbatim.
const CONTINUITY_SPRING: dioxus_motion::prelude::Spring = dioxus_motion::prelude::Spring {
    stiffness: 220.0,
    damping: 17.0,
    mass: 0.85,
};

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

fn continuity_config(reduce: bool) -> AnimationConfig {
    if reduce {
        beui_core::ease::tween_ease_out(0)
    } else {
        AnimationConfig::spring(CONTINUITY_SPRING)
    }
}

/// The shared expanded state.
#[derive(Clone, Copy)]
struct ExpandableState {
    expanded: Signal<bool>,
    controlled: bool,
    on_change: Option<EventHandler<bool>>,
}

impl ExpandableState {
    fn is_expanded(&self) -> bool {
        (self.expanded)()
    }

    fn set_expanded(&self, next: bool) {
        if !self.controlled {
            let mut signal = self.expanded;
            signal.set(next);
        }
        if let Some(handler) = self.on_change {
            handler.call(next);
        }
    }
}

/// The label's spring progress: 0 collapsed, 1 expanded.
fn use_continuity(expanded: bool, reduce: bool) -> f64 {
    let mut progress = use_motion(if expanded { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    use_effect(use_reactive((&expanded,), move |(expanded,)| {
        let _ = progress.animate_to(if expanded { 1.0 } else { 0.0 }, continuity_config(reduce));
    }));
    f64::from(current(progress).clamp(0.0, 1.0))
}

/// Props for [`ExpandableButton`].
#[derive(Props, Clone, PartialEq)]
pub struct ExpandableButtonProps {
    pub icon: Element,
    pub label: Element,
    #[props(default)]
    pub expanded: Option<bool>,
    #[props(default)]
    pub default_expanded: bool,
    #[props(default)]
    pub on_expanded_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub aria_label: Option<String>,
    #[props(default)]
    pub on_click: Option<EventHandler<()>>,
}

/// A pill button whose label slides out beside its icon.
#[component]
pub fn ExpandableButton(props: ExpandableButtonProps) -> Element {
    let ExpandableButtonProps {
        icon,
        label,
        expanded,
        default_expanded,
        on_expanded_change,
        class,
        disabled,
        aria_label,
        on_click,
    } = props;

    let reduce = use_reduced_motion();
    let internal = use_signal(|| default_expanded);
    let controlled = expanded.is_some();
    let state = ExpandableState {
        expanded: internal,
        controlled,
        on_change: on_expanded_change,
    };
    let is_expanded = match expanded {
        Some(expanded) => expanded,
        None => internal(),
    };
    let label_rect: ElementRect = use_element_rect();
    let p = use_continuity(is_expanded, reduce());
    let label_width = label_rect.get().map(|rect| rect.width).unwrap_or(0.0);
    let blur = 4.0 * (1.0 - p);
    let label_clone = label.clone();

    let class = cn!(
        "relative inline-flex h-11 min-w-11 items-center overflow-hidden rounded-full border border-border bg-transparent p-1 pr-1 text-sm font-medium text-foreground outline-none transition-colors focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
        class,
    );
    let label_span_style = format!("width:{}px", p * label_width);
    let label_inner_style = format!("opacity:{p};filter:blur({blur}px)");
    let aria = aria_label.unwrap_or_else(|| "Expand".to_string());

    rsx! {
        span { class: "inline-flex",
            button {
                r#type: "button",
                disabled,
                aria_label: aria,
                aria_expanded: if is_expanded { "true" } else { "false" },
                class,
                style: "border-radius:9999px",
                onclick: move |_| {
                    if let Some(handler) = on_click {
                        handler.call(());
                    }
                    state.set_expanded(!state.is_expanded());
                },
                span { aria_hidden: "true", class: "grid size-9 shrink-0 place-items-center", {icon} }
                span {
                    aria_hidden: "true",
                    class: "relative inline-flex items-center overflow-hidden whitespace-nowrap pr-3",
                    style: label_span_style,
                    span { class: "inline-flex w-max items-center whitespace-nowrap", style: label_inner_style, {label} }
                }
                // Hidden natural-width measurement for the label.
                span {
                    aria_hidden: "true",
                    class: "pointer-events-none invisible absolute inline-flex w-max items-center whitespace-nowrap pr-3",
                    "data-beui-rect": label_rect.attr(),
                    onmounted: move |data| label_rect.measure(data.data()),
                    {label_clone}
                }
            }
        }
    }
}

/// Props for [`ExpandableChip`].
#[derive(Props, Clone, PartialEq)]
pub struct ExpandableChipProps {
    pub label: Element,
    pub action_icon: Element,
    pub action_label: String,
    #[props(default)]
    pub on_action: Option<EventHandler<()>>,
    /// Collapse and refocus the trigger after the action. Defaults to `true`.
    #[props(default = true)]
    pub collapse_on_action: bool,
    #[props(default)]
    pub expanded: Option<bool>,
    #[props(default)]
    pub default_expanded: bool,
    #[props(default)]
    pub on_expanded_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub label_class: Option<String>,
    #[props(default)]
    pub action_class: Option<String>,
}

/// A chip whose action button slides out of the trailing edge.
#[component]
pub fn ExpandableChip(props: ExpandableChipProps) -> Element {
    let ExpandableChipProps {
        label,
        action_icon,
        action_label,
        on_action,
        collapse_on_action,
        expanded,
        default_expanded,
        on_expanded_change,
        disabled,
        class,
        label_class,
        action_class,
    } = props;

    let reduce = use_reduced_motion();
    let internal = use_signal(|| default_expanded);
    let controlled = expanded.is_some();
    let state = ExpandableState {
        expanded: internal,
        controlled,
        on_change: on_expanded_change,
    };
    let is_expanded = match expanded {
        Some(expanded) => expanded,
        None => internal(),
    };
    let p = use_continuity(is_expanded, reduce());
    let blur = 4.0 * (1.0 - p);

    let container_class = cn!(
        "inline-flex h-10 items-center overflow-hidden rounded-full border border-border bg-transparent text-sm text-foreground has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-inset has-[:focus-visible]:ring-ring",
        class,
    );
    // `pr-3` (12px) expanded → `pr-1` (4px) collapsed, animated with the spring.
    let trigger_padding = 12.0 - 8.0 * p;
    let trigger_class = cn!(
        "h-full whitespace-nowrap py-0 pl-3 font-medium outline-none disabled:pointer-events-none disabled:opacity-50",
        label_class,
    );
    let action_span_style = format!("width:{}px", p * 32.0);
    let action_inner_style = format!("opacity:{p};filter:blur({blur}px)");
    let action_class = cn!(
        "flex h-10 w-8 shrink-0 items-center justify-center rounded-full text-muted-foreground outline-none transition-colors hover:text-foreground disabled:pointer-events-none",
        if disabled { "opacity-50" } else { "" },
        action_class,
    );

    rsx! {
        span { class: "inline-flex",
            div { class: container_class, style: "border-radius:9999px",
                button {
                    r#type: "button",
                    disabled,
                    aria_expanded: if is_expanded { "true" } else { "false" },
                    class: trigger_class,
                    style: "padding-right:{trigger_padding}px",
                    onclick: move |_| state.set_expanded(!state.is_expanded()),
                    {label}
                }
                span {
                    aria_hidden: if is_expanded { "false" } else { "true" },
                    class: "relative inline-flex h-10 shrink-0 items-center overflow-hidden",
                    style: action_span_style,
                    button {
                        r#type: "button",
                        aria_label: action_label,
                        disabled: disabled || !is_expanded,
                        class: action_class,
                        style: action_inner_style,
                        onclick: move |_| {
                            if let Some(handler) = on_action {
                                handler.call(());
                            }
                            if collapse_on_action {
                                state.set_expanded(false);
                            }
                        },
                        span { aria_hidden: "true", {action_icon} }
                    }
                }
            }
        }
    }
}
