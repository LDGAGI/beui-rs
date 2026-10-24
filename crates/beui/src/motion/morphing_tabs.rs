//! `components/motion/morphing-tabs.tsx` — a tab rail whose active tab is joined
//! to its panel by a "liquid" surface.
//!
//! The surface is one continuous SVG shape spanning the panel, with a notch cut
//! where the active tab sits; the notch follows a spring as the active tab
//! changes and while a tab is dragged to reorder. That per-frame work lives in
//! `morphing-tabs.js`, exactly as motion values would: Rust renders the static
//! DOM and owns the value and the order, and the script owns the springs, the
//! path and the drag session.
//!
//! Deviations from the source:
//!
//! - items carry `label`/`content` as `String`s and have no icon slot;
//! - the drag is driven from the rail (pointer events on the wrapper) rather than
//!   per-tab motion-value transforms, and the dragged tab's hover plate uses the
//!   hover variant only;
//! - those two components each need one full round (942 / 1174 lines).

use dioxus::prelude::*;

use crate::icons::XIcon;
use beui_core::ease::{tween_ease_out, SPRING_PRESS};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceMode,
};
use beui_core::{cn, use_reduced_motion};

/// One tab.
#[derive(Clone, Debug, PartialEq)]
pub struct MorphingTabsItem {
    pub id: String,
    pub label: String,
    pub content: String,
    pub disabled: bool,
}

impl MorphingTabsItem {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            content: content.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn content_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(120),
            tween_ease_out(80),
        );
    }
    let mut enter = motion_style! { opacity: 0.0, y: 8.0 };
    enter.add_css_property("filter", "blur(6px)");
    let mut center = motion_style! { opacity: 1.0, y: 0.0 };
    center.add_css_property("filter", "blur(0px)");
    let mut exit = motion_style! { opacity: 0.0, y: -5.0 };
    exit.add_css_property("filter", "blur(5px)");
    PresenceConfig::with_transitions(
        enter,
        center,
        exit,
        dioxus_motion::prelude::AnimationConfig::spring(SPRING_PRESS),
        tween_ease_out(120),
    )
}

/// `MorphingTabs`.
#[component]
pub fn MorphingTabs(
    items: Vec<MorphingTabsItem>,
    #[props(default = None)] value: Option<String>,
    #[props(default = None)] default_value: Option<String>,
    #[props(default = None)] on_value_change: Option<EventHandler<Option<String>>>,
    /// Called once after a pointer drag or keyboard reorder completes.
    #[props(default = None)]
    on_order_change: Option<EventHandler<Vec<String>>>,
    /// Enables the close affordance on every tab when provided.
    #[props(default = None)]
    on_close: Option<EventHandler<String>>,
    #[props(default = "Tabs")] aria_label: &'static str,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let uid = use_hook(platform::next_scope_id);
    let uid_text = uid.to_string();
    let item_ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();

    // Order: retain the existing order for ids that survive, append new ones.
    let order = use_signal(|| item_ids.clone());
    {
        let current = (order)();
        let available: std::collections::HashSet<&String> = item_ids.iter().collect();
        let mut next: Vec<String> = current
            .iter()
            .filter(|id| available.contains(id))
            .cloned()
            .collect();
        for id in &item_ids {
            if !next.contains(id) {
                next.push(id.clone());
            }
        }
        if next != current {
            let mut slot = order;
            slot.set(next);
        }
    }

    let mut internal_value = use_signal(|| default_value.clone());
    let controlled = value.is_some();
    let fallback = internal_value();
    let current_value = value.clone().or(fallback);
    let active_signal = use_signal(|| current_value.clone());
    let mut active_sync = active_signal;
    if *active_sync.peek() != current_value {
        active_sync.set(current_value.clone());
    }
    // Fall back to the first enabled tab when nothing valid is selected.
    let first_enabled = items
        .iter()
        .find(|item| !item.disabled)
        .or_else(|| items.first())
        .cloned();
    let active_item = current_value
        .as_ref()
        .and_then(|id| items.iter().find(|item| &item.id == id))
        .cloned()
        .or_else(|| first_enabled.clone());
    let active_id = active_item.as_ref().map(|item| item.id.clone());
    if active_id != current_value {
        if let Some(id) = &active_id {
            let mut slot = active_signal;
            slot.set(Some(id.clone()));
        }
    }

    let items_for_active = items.clone();
    let set_active = EventHandler::new(move |next: Option<String>| {
        if next
            .as_ref()
            .and_then(|id| items_for_active.iter().find(|item| &item.id == id))
            .map(|item| item.disabled)
            .unwrap_or(false)
        {
            return;
        }
        if !controlled {
            internal_value.set(next.clone());
        }
        if let Some(callback) = on_value_change {
            callback.call(next);
        }
    });

    // Boot the script and listen for reorder commits.
    let order_signal = order;
    let uid_for_effect = uid_text.clone();
    use_effect(move || {
        let uid = uid_for_effect.clone();
        let script = format!(
            "{}\nwindow.__beuiMorphTabsInit({uid});",
            include_str!("../js/morphing-tabs.js"),
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                if let Some(rest) = message.strip_prefix("order:") {
                    let next: Vec<String> =
                        rest.split(',').map(|value| value.to_string()).collect();
                    let mut slot = order_signal;
                    if (slot)() != next {
                        slot.set(next.clone());
                    }
                    if let Some(callback) = on_order_change {
                        callback.call(next);
                    }
                    let uid = uid.clone();
                    platform::fire(&format!(
                        "window.__beuiMorphTabsMeasure && window.__beuiMorphTabsMeasure({uid})"
                    ));
                }
            }
        });
    });
    let uid_for_drop = uid_text.clone();
    use_drop(move || {
        platform::fire(&format!(
            "window.__beuiMorphTabsDestroy && window.__beuiMorphTabsDestroy({uid_for_drop})"
        ));
    });

    // Tell the script which tab is active, and re-measure when the order moves.
    let uid_for_active = uid_text.clone();
    use_effect(move || {
        let active = (active_signal)();
        let uid = uid_for_active.clone();
        let value = match active {
            Some(id) => platform::js_string(&id),
            None => "null".to_string(),
        };
        platform::fire(&format!(
            "window.__beuiMorphTabsSetActive && window.__beuiMorphTabsSetActive({uid}, {value})"
        ));
    });
    let uid_for_order = uid_text.clone();
    use_effect(move || {
        let _ = (order)();
        let uid = uid_for_order.clone();
        platform::fire(&format!(
            "window.__beuiMorphTabsMeasure && window.__beuiMorphTabsMeasure({uid})"
        ));
    });

    // Render in order.
    let ordered: Vec<MorphingTabsItem> = (order)()
        .iter()
        .filter_map(|id| items.iter().find(|item| &item.id == id).cloned())
        .collect();
    if ordered.is_empty() {
        return rsx! {};
    }

    let order_for_keys = order;
    let panel_id = format!("beui-morph-tabs-{uid}-panel");
    let labelled_by = active_id
        .as_ref()
        .map(|id| format!("beui-morph-tabs-{}-tab-{}", uid_text, safe_id(id)))
        .unwrap_or_default();

    rsx! {
        div {
            "data-morph-tabs": "{uid_text}",
            class: cn!(
                "relative isolate min-w-0 overflow-hidden rounded-[2rem] bg-[#292929] text-white",
                class.as_deref().unwrap_or(""),
            ),
            div { class: "relative h-20",
                div {
                    "data-morph-rail": "",
                    role: "tablist",
                    "aria-label": aria_label,
                    "aria-orientation": "horizontal",
                    class: "relative z-30 flex h-full gap-3 md:gap-4",
                    for item in ordered.iter().cloned() {
                        MorphingTabItemView {
                            key: "{item.id}",
                            item: item.clone(),
                            is_active: Some(&item.id) == active_id.as_ref(),
                            panel_id: panel_id.clone(),
                            uid_text: uid_text.clone(),
                            order: order_for_keys,
                            set_active,
                            on_order_change,
                            on_close,
                        }
                    }
                }
            }
            div {
                "data-morph-surface-host": "",
                class: "pointer-events-none absolute inset-x-0 top-0 h-[108px] w-full",
                svg {
                    "data-morph-surface": "",
                    "aria-hidden": "true",
                    "focusable": "false",
                    "viewBox": "0 0 1000 108",
                    "preserveAspectRatio": "none",
                    class: "h-[108px] w-full text-[#fafaf8]",
                    path { "data-morph-surface-path": "", fill: "currentColor" }
                }
            }
            div {
                id: "{panel_id}",
                role: "tabpanel",
                "aria-labelledby": labelled_by,
                class: "relative z-20 mx-4 min-h-64 overflow-hidden rounded-[1.75rem] bg-[#fafaf8] text-[#181818]",
                AnimatePresence {
                    initial: false,
                    mode: PresenceMode::PopLayout,
                    for id in active_id.iter().cloned().collect::<Vec<_>>() {
                        MorphingTabsPanel {
                            key: "{id}",
                            content: active_item.as_ref().map(|item| item.content.clone()).unwrap_or_default(),
                            reduce,
                        }
                    }
                }
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn MorphingTabItemView(
    item: MorphingTabsItem,
    is_active: bool,
    panel_id: String,
    uid_text: String,
    order: Signal<Vec<String>>,
    set_active: EventHandler<Option<String>>,
    on_order_change: Option<EventHandler<Vec<String>>>,
    on_close: Option<EventHandler<String>>,
) -> Element {
    let tab_id = format!("beui-morph-tabs-{}-tab-{}", uid_text, safe_id(&item.id));
    let item_for_click = item.clone();
    let item_for_key = item.clone();
    let item_id_for_close = item.id.clone();
    let uid_for_key = uid_text.clone();
    let mut set_order = order;
    rsx! {
        div {
            "data-morph-tab": "{item.id}",
            "data-disabled": if item.disabled { Some("true") } else { None },
            class: cn!(
                "group absolute left-0 top-0 flex touch-pan-y items-stretch",
                beui_core::touch::TOUCH_GESTURE_CLASS,
                if item.disabled { "cursor-not-allowed" } else { "cursor-grab" },
            ),
            div {
                "data-morph-tab-body": "",
                class: "relative flex items-stretch",
                if !is_active {
                    span {
                        "aria-hidden": "true",
                        class: "absolute inset-x-0 bottom-2 top-0 rounded-[1.25rem] transition-colors duration-200 bg-transparent group-hover:bg-white/[0.06]",
                    }
                }
                button {
                    id: "{tab_id}",
                    r#type: "button",
                    role: "tab",
                    "aria-selected": is_active,
                    "aria-controls": "{panel_id}",
                    "aria-disabled": if item.disabled { Some("true") } else { None },
                    tabindex: if is_active { 0 } else { -1 },
                    disabled: item.disabled,
                    class: cn!(
                        "group relative z-10 flex h-full w-full min-w-0 items-center gap-2 overflow-hidden rounded-t-[1.5rem] px-3 text-left outline-none transition-colors",
                        if is_active { "text-[#181818]" } else { "pb-2 text-white/70 hover:text-white" },
                    ),
                    onclick: move |_| {
                        set_active.call(Some(item_for_click.id.clone()));
                    },
                    onkeydown: move |event: KeyboardEvent| {
                        let key = event.key();
                        let order_now = set_order.peek().clone();
                        let Some(index) = order_now.iter().position(|id| *id == item_for_key.id) else {
                            return;
                        };
                        if event.modifiers().alt() && (key == Key::ArrowLeft || key == Key::ArrowRight) {
                            event.prevent_default();
                            let next_index = if key == Key::ArrowLeft {
                                index as isize - 1
                            } else {
                                index as isize + 1
                            };
                            if next_index < 0 || next_index as usize >= order_now.len() {
                                return;
                            }
                            let mut next = order_now.clone();
                            let moved = next.remove(index);
                            next.insert(next_index as usize, moved);
                            set_order.set(next.clone());
                            if let Some(callback) = on_order_change {
                                callback.call(next.clone());
                            }
                            let uid = uid_for_key.clone();
                            platform::fire(&format!(
                                "window.__beuiMorphTabsMeasure && window.__beuiMorphTabsMeasure({uid})"
                            ));
                            return;
                        }
                        if key != Key::ArrowLeft && key != Key::ArrowRight {
                            return;
                        }
                        event.prevent_default();
                        let direction = if key == Key::ArrowLeft { -1isize } else { 1 };
                        let len = order_now.len() as isize;
                        let next_index = (index as isize + direction).rem_euclid(len) as usize;
                        let next_id = order_now[next_index].clone();
                        set_active.call(Some(next_id.clone()));
                        let target = format!("beui-morph-tabs-{}-tab-{}", uid_for_key, safe_id(&next_id));
                        platform::fire(&format!(
                            "requestAnimationFrame(() => document.getElementById({})?.focus())",
                            platform::js_string(&target)
                        ));
                    },
                    span {
                        "aria-hidden": "true",
                        class: cn!(
                            "pointer-events-none absolute inset-x-1 top-1 opacity-0 transition-opacity group-focus-visible:opacity-100",
                            if is_active {
                                "bottom-0 rounded-t-[1.25rem] border-x-2 border-t-2 border-black/20"
                            } else {
                                "bottom-2 rounded-[1rem] border-2 border-white/60"
                            },
                        ),
                    }
                    span {
                        class: "min-w-0 truncate whitespace-nowrap text-base font-medium leading-none tracking-[-0.025em]",
                        "{item.label}"
                    }
                }
                if let Some(close) = on_close {
                    button {
                        r#type: "button",
                        "aria-label": "Close {item.label}",
                        class: cn!(
                            "absolute right-2 top-1/2 z-20 grid size-6 -translate-y-1/2 place-items-center rounded-full text-[#9aa0a8] transition-colors hover:bg-black/[0.06] hover:text-[#4b5563] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-current",
                            if is_active { "" } else { "top-[calc(50%-4px)] text-white/45 hover:bg-white/[0.08] hover:text-white/80" },
                        ),
                        onpointerdown: move |event: Event<PointerData>| event.stop_propagation(),
                        onclick: move |event: MouseEvent| {
                            event.stop_propagation();
                            close.call(item_id_for_close.clone());
                        },
                        XIcon { class: "size-3.5 stroke-[1.5]" }
                    }
                }
            }
        }
    }
}

#[component]
fn MorphingTabsPanel(content: String, reduce: bool) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(content_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);
    rsx! {
        div {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "min-h-64",
            "{content}"
        }
    }
}
