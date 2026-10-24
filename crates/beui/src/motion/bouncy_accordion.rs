//! `components/motion/bouncy-accordion.tsx` — connected rows that spring open.
//!
//! The open row becomes a rounded card; the rows on either side of it lose the
//! corners that touch it, so a run of rows reads as one connected group. Every
//! row's *position* is a `layout="position"` animation — one `beui-core::flip`
//! per row on the accordion's active value, so a row that is pushed by a
//! neighbour's expansion starts from where it was and springs to its new place.
//!
//! Deviations from the source:
//!
//! - `title`/`description` are `String`s and there is no `icon` slot, because an
//!   `items: Vec<_>` needs `Clone + PartialEq` and an `Element` has neither;
//! - the row's margin is set at once and left to the flip rather than animated
//!   separately, which is the same curve the source gives both;
//! - the connected-group corner radii are a CSS transition, not a spring.

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

use crate::icons::ChevronDownIcon;
use crate::motion::spring_from_visual_duration;
use beui_core::ease::{tween_ease_out, EASE_OUT_CSS};
use beui_core::flip::{use_flip, FlipMode, FlipOptions};
use beui_core::platform;
use beui_core::{cn, use_reduced_motion};

/// One row.
#[derive(Clone, Debug, PartialEq)]
pub struct BouncyAccordionItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub disabled: bool,
}

impl BouncyAccordionItem {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            disabled: false,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// `BouncyAccordion`.
#[component]
pub fn BouncyAccordion(
    items: Vec<BouncyAccordionItem>,
    /// Controlled value; `Some` means controlled.
    #[props(default = None)]
    value: Option<String>,
    #[props(default = None)] default_value: Option<String>,
    #[props(default = None)] on_value_change: Option<EventHandler<Option<String>>>,
    #[props(default = true)] collapsible: bool,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let mut internal = use_signal(|| default_value.clone());
    let controlled = value.is_some();
    let fallback = internal();
    let active = value.clone().or(fallback);

    let set_active = EventHandler::new(move |next: Option<String>| {
        if !controlled {
            internal.set(next.clone());
        }
        if let Some(callback) = on_value_change {
            callback.call(next);
        }
    });

    let base = use_hook(platform::next_scope_id);
    let active_index = items
        .iter()
        .position(|item| active.as_deref() == Some(item.id.as_str()))
        .map(|index| index as isize)
        .unwrap_or(-1);

    rsx! {
        div { class: cn!("w-full", class.as_deref().unwrap_or("")),
            for (index, item) in items.iter().enumerate() {
                {
                    let is_open = active.as_deref() == Some(item.id.as_str());
                    let previous_is_open = active_index == index as isize - 1;
                    let next_is_open = active_index == index as isize + 1;
                    let active_for_toggle = active.clone();
                    rsx! {
                        BouncyAccordionRow {
                            key: "{item.id}",
                            item: item.clone(),
                            open: is_open,
                            starts_group: is_open || index == 0 || previous_is_open,
                            ends_group: is_open || index + 1 == items.len() || next_is_open,
                            separated: index > 0 && (is_open || previous_is_open),
                            content_id: format!("beui-acc-{base}-{}-content", item.id),
                            trigger_id: format!("beui-acc-{base}-{}-trigger", item.id),
                            reduce,
                            active: active.clone(),
                            on_toggle: move |id: String| {
                                let next = if active_for_toggle.as_deref() == Some(id.as_str()) {
                                    if collapsible { None } else { Some(id) }
                                } else {
                                    Some(id)
                                };
                                set_active.call(next);
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn BouncyAccordionRow(
    item: BouncyAccordionItem,
    open: bool,
    starts_group: bool,
    ends_group: bool,
    separated: bool,
    content_id: String,
    trigger_id: String,
    reduce: bool,
    active: Option<String>,
    on_toggle: EventHandler<String>,
) -> Element {
    // `open` is a plain prop; the animation effects read the signal so they
    // re-run when it flips.
    let mut open_signal = use_signal(|| open);
    if *open_signal.peek() != open {
        open_signal.set(open);
    }

    let row_spring = spring_from_visual_duration(0.55, 0.38);
    let flip = use_flip(
        active,
        FlipOptions::default()
            .mode(FlipMode::Position)
            .transition(if reduce {
                tween_ease_out(1)
            } else {
                AnimationConfig::spring(row_spring)
            }),
    );

    let content_node: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let content_height = use_signal(|| 0.0_f64);
    use_effect(move || {
        let Some(node) = content_node() else {
            return;
        };
        let mut height = content_height;
        spawn(async move {
            if let Some(rect) = platform::client_rect(&node).await {
                height.set(rect.height);
            }
        });
    });

    // Height springs (open 0.58/0.32, close 0.46/0.26).
    let progress = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let progress_signal = progress;
    use_effect(move || {
        let is_open = open_signal();
        let mut handle = progress_signal;
        let config = if reduce {
            tween_ease_out(1)
        } else if is_open {
            AnimationConfig::spring(spring_from_visual_duration(0.58, 0.32))
        } else {
            AnimationConfig::spring(spring_from_visual_duration(0.46, 0.26))
        };
        let _ = handle.animate_to(if is_open { 1.0 } else { 0.0 }, config);
    });

    // Description fades on its own tween (0.18 s ease-out).
    let description = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let description_signal = description;
    use_effect(move || {
        let is_open = open_signal();
        let mut handle = description_signal;
        let _ = handle.animate_to(
            if is_open { 1.0 } else { 0.0 },
            tween_ease_out(if reduce { 1 } else { 180 }),
        );
    });

    // Chevron rotates on its own spring (0.42/0.28).
    let chevron = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let chevron_signal = chevron;
    use_effect(move || {
        let is_open = open_signal();
        let mut handle = chevron_signal;
        let config = if reduce {
            tween_ease_out(1)
        } else {
            AnimationConfig::spring(spring_from_visual_duration(0.42, 0.28))
        };
        let _ = handle.animate_to(if is_open { 180.0 } else { 0.0 }, config);
    });

    let has_description = item.description.is_some();
    let height = if has_description {
        f64::from(progress.get_value()) * (content_height)()
    } else {
        0.0
    };
    let opacity = description.get_value();
    let rotation = chevron.get_value();

    let margin = if separated { 12.0 } else { 0.0 };
    let (top_radius, bottom_radius) = (
        if starts_group { 28.0 } else { 0.0 },
        if ends_group { 28.0 } else { 0.0 },
    );

    rsx! {
        div {
            "data-beui-flip": flip.scope(),
            style: flip.style_with(&format!("margin-top:{margin}px")),
            div {
                "data-state": if open { "open" } else { "closed" },
                style: format!(
                    "border-top-left-radius:{top_radius}px;border-top-right-radius:{top_radius}px;\
                     border-bottom-left-radius:{bottom_radius}px;border-bottom-right-radius:{bottom_radius}px;\
                     transition:border-radius 550ms cubic-bezier(0.34,1.4,0.64,1)",
                ),
                class: cn!("overflow-hidden bg-card text-card-foreground", if item.disabled { "opacity-50" } else { "" }),
                button {
                    id: "{trigger_id}",
                    r#type: "button",
                    disabled: item.disabled,
                    "aria-expanded": open,
                    "aria-controls": "{content_id}",
                    class: cn!(
                        "flex min-h-[54px] w-full items-center gap-4 px-5 text-left outline-none transition-colors",
                        "focus-visible:bg-muted/25 disabled:pointer-events-none",
                    ),
                    onclick: {
                        let id = item.id.clone();
                        move |_| on_toggle.call(id.clone())
                    },
                    span { class: "min-w-0 flex-1 truncate text-[15px] font-medium text-foreground",
                        "{item.title}"
                    }
                    span {
                        "aria-hidden": "true",
                        style: format!("display:inline-grid;place-items:center;transform:rotate({rotation}deg)"),
                        class: "h-6 w-6 shrink-0 text-muted-foreground",
                        ChevronDownIcon { class: "h-4 w-4" }
                    }
                }
                div {
                    id: "{content_id}",
                    role: "region",
                    "aria-labelledby": "{trigger_id}",
                    "aria-hidden": if open { None } else { Some("true") },
                    inert: if open { None } else { Some("true") },
                    style: format!("height:{height}px;overflow:hidden"),
                    div {
                        onmounted: move |event| {
                            let mut node = content_node;
                            node.set(Some(event.data()));
                        },
                        style: format!("opacity:{opacity};transition:opacity {EASE_OUT_CSS}"),
                        class: "px-5 pb-5",
                        div { class: "text-[15px] leading-6 text-muted-foreground",
                            if let Some(description) = &item.description {
                                "{description}"
                            }
                        }
                    }
                }
            }
        }
    }
}
