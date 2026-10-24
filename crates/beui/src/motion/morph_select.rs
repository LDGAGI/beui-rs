//! `components/motion/select-morph.tsx` — a select whose trigger *becomes* the
//! panel.
//!
//! The trigger and the panel are two elements sharing one `layoutId`: the
//! trigger is rendered only while closed, the panel only while open, and the new
//! element starts from the box the old one was measured at — a
//! `beui-core::flip`. Nothing detaches; the trigger grows into the menu and
//! shrinks back.
//!
//! Deviations from the source:
//!
//! - the source wraps the swap in `AnimatePresence mode="popLayout"` for the
//!   crossfade; the port relies on the flip alone (the panel covers the
//!   trigger's box as it grows);
//! - the per-item stagger is not ported (the panel morph is the entrance);
//! - `duration`+`bounce` springs are converted by [`spring_from_visual_duration`]
//!   as if they were `visualDuration`+`bounce` — motion's `duration` form uses a
//!   numeric solver instead, a known approximation across the lane.

use dioxus::prelude::*;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

use crate::icons::{CheckIcon, ChevronDownIcon};
use crate::motion::spring_from_visual_duration;
use beui_core::ease::tween_ease_out;
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform;
use beui_core::{cn, use_reduced_motion};

/// Trigger and panel header share this row so the morph stays seamless.
const ROW: &str = "flex w-full items-center justify-between gap-2 px-3.5 py-2.5 text-sm";

/// `MORPH`: `{ type: "spring", duration: 0.5, bounce: 0.22 }`.
fn morph_config(reduce: bool) -> AnimationConfig {
    if reduce {
        tween_ease_out(1)
    } else {
        AnimationConfig::spring(spring_from_visual_duration(0.5, 0.22))
    }
}

#[derive(Clone)]
struct MorphSelectContext {
    value: Signal<Option<String>>,
    open: Signal<bool>,
    set_open: EventHandler<bool>,
    select: EventHandler<String>,
    labels: Signal<Vec<(String, String)>>,
    placeholder: Signal<String>,
    reduce: bool,
    uid: u32,
    disabled: bool,
}

impl MorphSelectContext {
    fn label_for(&self, value: Option<String>) -> Option<String> {
        let value = value?;
        (self.labels)()
            .into_iter()
            .find(|(key, _)| *key == value)
            .map(|(_, label)| label)
    }

    fn layout_id(&self) -> String {
        format!("beui-morph-select-{}", self.uid)
    }
}

fn use_morph_select() -> MorphSelectContext {
    use_context::<MorphSelectContext>()
}

/// `MorphSelect`.
#[component]
pub fn MorphSelect(
    #[props(default = None)] value: Option<String>,
    #[props(default = None)] default_value: Option<String>,
    #[props(default = None)] on_value_change: Option<EventHandler<String>>,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let mut internal = use_signal(|| default_value.clone());
    let controlled = value.is_some();
    let fallback = internal();
    let current = value.clone().or(fallback);
    let mut open = use_signal(|| false);

    let mut value_signal = use_signal(|| current.clone());
    if *value_signal.peek() != current {
        value_signal.set(current.clone());
    }

    let set_open = EventHandler::new(move |next: bool| {
        open.set(next);
    });
    let select = EventHandler::new(move |next: String| {
        if !controlled {
            internal.set(Some(next.clone()));
        }
        if let Some(callback) = on_value_change {
            callback.call(next);
        }
        open.set(false);
    });

    let uid = use_hook(platform::next_scope_id);
    let labels = use_signal(Vec::<(String, String)>::new);
    let placeholder = use_signal(|| "Select".to_string());
    use_context_provider(|| MorphSelectContext {
        value: value_signal,
        open,
        set_open,
        select,
        labels,
        placeholder,
        reduce,
        uid,
        disabled,
    });

    let close = EventHandler::new(move |_| set_open.call(false));
    let scope = use_dismiss(open, close, DismissOptions::default());

    rsx! {
        div {
            "data-beui-scope": scope.attr(),
            class: cn!("relative", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `MorphSelectValue`.
#[component]
pub fn MorphSelectValue(
    #[props(default = None)] placeholder: Option<String>,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let context = use_morph_select();
    let mut slot = context.placeholder;
    if let Some(placeholder) = &placeholder {
        if *slot.peek() != *placeholder {
            slot.set(placeholder.clone());
        }
    }
    let label = context.label_for((context.value)());
    let has_label = label.is_some();
    let text = label
        .or(placeholder)
        .unwrap_or_else(|| "Select".to_string());

    rsx! {
        span {
            class: cn!(
                if has_label { "text-foreground" } else { "text-muted-foreground" },
                class.as_deref().unwrap_or(""),
            ),
            "{text}"
        }
    }
}

/// `MorphSelectTrigger`.
#[component]
pub fn MorphSelectTrigger(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_morph_select();
    let open = (context.open)();
    let flip = use_flip(
        open,
        FlipOptions::default()
            .layout_id(context.layout_id())
            .transition(morph_config(context.reduce))
            .border_radius(12.0),
    );
    let trigger_children = children.clone();

    rsx! {
        div {
            "aria-hidden": "true",
            inert: "true",
            class: cn!(ROW, "invisible rounded-xl border border-border"),
            {children}
            ChevronDownIcon { class: "h-4 w-4" }
        }
        if !open {
            button {
                r#type: "button",
                id: "beui-morph-select-{context.uid}-trigger",
                disabled: context.disabled,
                "aria-haspopup": "listbox",
                "aria-expanded": open,
                "aria-controls": "beui-morph-select-{context.uid}-list",
                "data-beui-flip": flip.scope(),
                style: flip.style_with("border-radius:12px"),
                class: cn!(
                    ROW,
                    "absolute inset-x-0 top-0 z-10 border border-border bg-background text-foreground outline-none transition-colors",
                    "hover:border-(--color-border-strong) focus-visible:ring-2 focus-visible:ring-foreground/20",
                    "disabled:pointer-events-none disabled:opacity-50",
                    class.as_deref().unwrap_or(""),
                ),
                onclick: move |_| context.set_open.call(true),
                span { class: "min-w-0 truncate", {trigger_children} }
                span { class: "text-muted-foreground", ChevronDownIcon { class: "h-4 w-4" } }
            }
        }
    }
}

/// `MorphSelectContent`.
#[component]
pub fn MorphSelectContent(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_morph_select();
    let open = (context.open)();
    let label = context.label_for((context.value)());
    let has_label = label.is_some();
    let text = label.unwrap_or_else(|| (context.placeholder)());
    let panel_children = children.clone();
    let hidden_children = children;

    // The chevron flips as the panel opens.
    let chevron = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let chevron_signal = chevron;
    let reduce = context.reduce;
    use_effect(move || {
        let mut handle = chevron_signal;
        let _ = handle.animate_to(180.0, morph_config(reduce));
    });

    rsx! {
        div { class: "hidden", {hidden_children} }
        if open {
            MorphSelectPanel {
                class,
                text,
                has_label,
                rotation: chevron.get_value(),
                children: panel_children,
            }
        }
    }
}

#[component]
fn MorphSelectPanel(
    class: Option<String>,
    text: String,
    has_label: bool,
    rotation: f32,
    children: Element,
) -> Element {
    let context = use_morph_select();
    let flip = use_flip(
        true,
        FlipOptions::default()
            .layout_id(context.layout_id())
            .transition(morph_config(context.reduce))
            .border_radius(12.0),
    );

    rsx! {
        div {
            id: "beui-morph-select-{context.uid}-list",
            role: "listbox",
            "aria-labelledby": "beui-morph-select-{context.uid}-trigger",
            "data-beui-flip": flip.scope(),
            style: flip.style_with("border-radius:12px"),
            class: cn!(
                "absolute inset-x-0 top-0 z-30 overflow-hidden border border-border bg-background shadow-lg",
                class.as_deref().unwrap_or(""),
            ),
            button {
                r#type: "button",
                "aria-expanded": "true",
                class: cn!(ROW, "outline-none"),
                onclick: move |_| context.set_open.call(false),
                span {
                    class: cn!("min-w-0 truncate", if has_label { "text-foreground" } else { "text-muted-foreground" }),
                    "{text}"
                }
                span {
                    style: format!("display:inline-flex;transform:rotate({rotation}deg)"),
                    class: "text-muted-foreground",
                    ChevronDownIcon { class: "h-4 w-4" }
                }
            }
            div { class: "h-px bg-border" }
            ul { class: "p-1", {children} }
        }
    }
}

/// `MorphSelectItem`.
#[component]
pub fn MorphSelectItem(
    value: String,
    #[props(default = false)] disabled: bool,
    /// The text shown on the trigger. Defaults to `value`.
    #[props(default = None)]
    label: Option<String>,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_morph_select();
    let selected = (context.value)().as_deref() == Some(value.as_str());
    let label = label.unwrap_or_else(|| value.clone());

    let registry = context.labels;
    let item_value = value.clone();
    let item_label = label.clone();
    use_effect(move || {
        let mut signal = registry;
        signal
            .write()
            .push((item_value.clone(), item_label.clone()));
    });
    let registry = context.labels;
    let drop_value = value.clone();
    use_drop(move || {
        // Items render twice (hidden registrar + open panel); drop one copy.
        let mut signal = registry;
        let mut list = signal.write();
        if let Some(index) = list.iter().position(|(key, _)| *key == drop_value) {
            list.remove(index);
        }
    });

    rsx! {
        li {
            button {
                r#type: "button",
                role: "option",
                "aria-selected": selected,
                disabled,
                class: cn!(
                    "flex w-full items-center justify-between gap-2 rounded-lg px-2.5 py-1.5 text-left text-sm outline-none transition-colors",
                    if selected { "bg-muted text-foreground" } else { "text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:bg-muted" },
                    "disabled:pointer-events-none disabled:opacity-50",
                    class.as_deref().unwrap_or(""),
                ),
                onclick: {
                    let value = value.clone();
                    move |_| context.select.call(value.clone())
                },
                {children}
                if selected {
                    CheckIcon { class: "h-3.5 w-3.5 shrink-0" }
                }
            }
        }
    }
}
