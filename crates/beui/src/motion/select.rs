//! `components/motion/select.tsx` — a listbox that unfolds out of its field.
//!
//! The panel is absolutely positioned *inside* the field, so it never needs a
//! portal; on open it flips above the trigger when there is more room there, and
//! animates its height, the gap on the side facing the trigger and its near
//! corners, so the trigger and the panel look pinched apart.
//!
//! Deviations from the source:
//!
//! - the height is a real spring (`spring_from_visual_duration(0.42, 0.14)`, the
//!   source's own `visualDuration`/`bounce`); the opacity, gap and corner tweens
//!   are CSS transitions matching the source's durations and delays, because a
//!   `dioxus-motion` handle animates one property group at one speed;
//! - the trigger's corner pinch (a keyframe that only dips mid-flight and rests
//!   at 12 either way) and the per-item stagger are not ported — both are
//!   mid-flight only, so every rest state is identical.

use dioxus::prelude::*;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

use crate::icons::{CheckIcon, ChevronDownIcon};
use crate::motion::spring_from_visual_duration;
use beui_core::ease::{tween_ease_out, EASE_OUT_CSS};
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform;
use beui_core::{cn, use_reduced_motion};

/// Which side of the trigger the panel opens on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectPlacement {
    #[default]
    Bottom,
    Top,
}

/// `SelectContext` — value, open state, the label registry and the ids.
#[derive(Clone)]
struct SelectContext {
    value: Signal<Option<String>>,
    open: Signal<bool>,
    set_open: EventHandler<bool>,
    select: EventHandler<String>,
    labels: Signal<Vec<(String, String)>>,
    reduce: bool,
    trigger_id: String,
    list_id: String,
    disabled: bool,
    placement: Signal<SelectPlacement>,
}

fn use_select() -> SelectContext {
    use_context::<SelectContext>()
}

/// `Select`.
#[component]
pub fn Select(
    /// Controlled value; `Some` means controlled (Rust has no `undefined`).
    #[props(default = None)]
    value: Option<String>,
    #[props(default = None)] default_value: Option<String>,
    #[props(default = None)] on_value_change: Option<EventHandler<String>>,
    /// Controlled open state, so a stacked layout can keep one panel open.
    #[props(default = None)]
    open: Option<bool>,
    #[props(default = false)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let mut internal_value = use_signal(|| default_value.clone());
    let mut internal_open = use_signal(|| default_open);
    let controlled_value = value.is_some();
    let controlled_open = open.is_some();
    let fallback_value = internal_value();
    let fallback_open = internal_open();
    let current_value = value.clone().or(fallback_value);
    let current_open = open.unwrap_or(fallback_open);

    let mut value_signal = use_signal(|| current_value.clone());
    if *value_signal.peek() != current_value {
        value_signal.set(current_value.clone());
    }
    let mut open_signal = use_signal(|| current_open);
    if *open_signal.peek() != current_open {
        open_signal.set(current_open);
    }

    let set_open = EventHandler::new(move |next: bool| {
        if !controlled_open {
            internal_open.set(next);
        }
        if let Some(callback) = on_open_change {
            callback.call(next);
        }
    });
    let select = EventHandler::new(move |next: String| {
        if !controlled_value {
            internal_value.set(Some(next.clone()));
        }
        if let Some(callback) = on_value_change {
            callback.call(next);
        }
        set_open.call(false);
    });

    let base = use_hook(platform::next_scope_id);
    let labels = use_signal(Vec::<(String, String)>::new);
    let placement = use_signal(SelectPlacement::default);
    use_context_provider(|| SelectContext {
        value: value_signal,
        open: open_signal,
        set_open,
        select,
        labels,
        reduce,
        trigger_id: format!("beui-select-{base}-trigger"),
        list_id: format!("beui-select-{base}-list"),
        disabled,
        placement,
    });

    // Outside pointer / Escape.
    let dismiss_close = EventHandler::new(move |_| set_open.call(false));
    let scope = use_dismiss(open_signal, dismiss_close, DismissOptions::default());

    rsx! {
        div {
            "data-beui-scope": scope.attr(),
            class: cn!("relative", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `SelectTrigger`.
#[component]
pub fn SelectTrigger(#[props(default = None)] class: Option<String>, children: Element) -> Element {
    let context = use_select();
    let open = (context.open)();

    rsx! {
        button {
            r#type: "button",
            id: "{context.trigger_id}",
            disabled: context.disabled,
            "aria-haspopup": "listbox",
            "aria-expanded": open,
            "aria-controls": "{context.list_id}",
            class: cn!(
                "relative z-10 flex w-full items-center justify-between gap-2 rounded-xl border border-border bg-background px-3 py-2 text-sm text-foreground outline-none transition-colors",
                "hover:border-(--color-border-strong) focus-visible:ring-2 focus-visible:ring-foreground/20",
                "disabled:pointer-events-none disabled:opacity-50",
                class.as_deref().unwrap_or(""),
            ),
            onclick: move |_| context.set_open.call(!(context.open)()),
            {children}
            span {
                "aria-hidden": "true",
                style: format!(
                    "display:inline-flex;transition:transform 400ms {EASE_OUT_CSS};transform:rotate({}deg)",
                    if open { 180 } else { 0 },
                ),
                class: "text-muted-foreground",
                ChevronDownIcon { class: "h-4 w-4" }
            }
        }
    }
}

/// `SelectValue`.
#[component]
pub fn SelectValue(
    #[props(default = None)] placeholder: Option<String>,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let context = use_select();
    let value = (context.value)();
    let selected_label = value.and_then(|value| {
        (context.labels)()
            .into_iter()
            .find(|(key, _)| *key == value)
            .map(|(_, label)| label)
    });
    let has_label = selected_label.is_some();
    let text = selected_label
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

/// `SelectContent`.
#[component]
pub fn SelectContent(#[props(default = None)] class: Option<String>, children: Element) -> Element {
    let context = use_select();
    let open = (context.open)();
    let reduce = context.reduce;
    let is_top = (context.placement)() == SelectPlacement::Top;

    let content_height = use_signal(|| 0.0_f64);
    // The panel's own inner element, found by the script for its `offsetHeight`.
    let inner_marker = context.list_id.clone();
    let trigger_id = context.trigger_id.clone();
    let height_signal = content_height;
    let placement_signal = context.placement;
    let list_id_for_effect = context.list_id.clone();

    // Measure the panel and pick the side whenever the panel opens or the
    // option set changes. This is `popover-position`'s sibling: one script read
    // of the trigger box, the viewport and the panel's content height.
    let open_signal = context.open;
    let labels = context.labels;
    use_effect(move || {
        let is_open = open_signal();
        let _labels_len = (labels)().len();
        let script = format!(
            "(() => {{ const trigger = document.getElementById({trigger}); \
             const inner = document.querySelector({inner}); \
             if (!inner) {{ dioxus.send('0:bottom'); return; }} const h = inner.offsetHeight; \
             if (!trigger) {{ dioxus.send(h + ':bottom'); return; }} \
             const r = trigger.getBoundingClientRect(); \
             const below = window.innerHeight - r.bottom; const above = r.top; \
             const placement = (below < h + 16 && above > below) ? 'top' : 'bottom'; \
             dioxus.send(h + ':' + placement); }})()",
            trigger = platform::js_string(&trigger_id),
            inner = platform::js_string(&format!("[data-beui-select-inner=\"{inner_marker}\"]")),
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        let mut height = height_signal;
        let mut placement = placement_signal;
        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                let mut parts = message.split(':');
                let Some(height_text) = parts.next() else {
                    continue;
                };
                let Ok(parsed) = height_text.parse::<f64>() else {
                    continue;
                };
                height.set(parsed);
                if !is_open {
                    continue;
                }
                if parts.next() == Some("top") {
                    placement.set(SelectPlacement::Top);
                } else {
                    placement.set(SelectPlacement::Bottom);
                }
            }
        });
        let _ = list_id_for_effect;
    });

    // The height is the one property with a real spring.
    let progress = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let progress_signal = progress;
    use_effect(move || {
        let is_open = (context.open)();
        let mut handle = progress_signal;
        let config = if reduce {
            tween_ease_out(1)
        } else if is_open {
            AnimationConfig::spring(spring_from_visual_duration(0.42, 0.14))
        } else {
            tween_ease_out(260)
        };
        let _ = handle.animate_to(if is_open { 1.0 } else { 0.0 }, config);
    });

    let fraction = f64::from(progress.get_value());
    let height = if reduce {
        if open {
            (content_height)()
        } else {
            0.0
        }
    } else {
        fraction * (content_height)()
    };

    let near_gap = if open { 8.0 } else { 0.0 };
    let near_radius = if open { 12.0 } else { 0.0 };
    let (top_radius, bottom_radius) = if is_top {
        (12.0, near_radius)
    } else {
        (near_radius, 12.0)
    };

    let transition = if open {
        format!(
            "opacity 180ms {EASE_OUT_CSS}, border-radius 300ms {EASE_OUT_CSS} 140ms, margin 600ms cubic-bezier(0.34,1.56,0.64,1) 120ms"
        )
    } else {
        format!(
            "opacity 160ms {EASE_OUT_CSS} 120ms, border-radius 160ms {EASE_OUT_CSS} 0ms, margin 300ms cubic-bezier(0.34,1.2,0.64,1) 0ms"
        )
    };
    let (margin_top, margin_bottom) = if is_top {
        (0.0, near_gap)
    } else {
        (near_gap, 0.0)
    };

    let style = format!(
        "transform-origin:{};overflow:hidden;pointer-events:{};height:{height}px;opacity:{};\
         margin-top:{margin_top}px;margin-bottom:{margin_bottom}px;\
         border-top-left-radius:{top_radius}px;border-top-right-radius:{top_radius}px;\
         border-bottom-left-radius:{bottom_radius}px;border-bottom-right-radius:{bottom_radius}px;\
         transition:{transition}",
        if is_top { "bottom" } else { "top" },
        if open { "auto" } else { "none" },
        if open { 1 } else { 0 },
    );

    rsx! {
        div {
            id: "{context.list_id}",
            role: "listbox",
            "aria-labelledby": "{context.trigger_id}",
            "aria-hidden": if open { None } else { Some("true") },
            inert: if open { None } else { Some("true") },
            "data-placement": if is_top { "top" } else { "bottom" },
            style,
            class: cn!(
                "absolute left-0 right-0 z-20 rounded-xl border border-border bg-background shadow-lg",
                if is_top { "bottom-full" } else { "top-full" },
                class.as_deref().unwrap_or(""),
            ),
            div {
                class: "p-1",
                "data-beui-select-inner": "{context.list_id}",
                {children}
            }
        }
    }
}

/// `SelectItem`.
#[component]
pub fn SelectItem(
    value: String,
    #[props(default = false)] disabled: bool,
    /// The text shown on the trigger when this item is selected. Defaults to
    /// `value`, because a Dioxus `Element` child cannot be read back as text.
    #[props(default = None)]
    label: Option<String>,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_select();
    let selected = (context.value)().as_deref() == Some(value.as_str());
    let label = label.unwrap_or_else(|| value.clone());

    let registry = context.labels;
    let item_value = value.clone();
    let item_label = label.clone();
    use_effect(move || {
        let mut signal = registry;
        let mut list = signal.write();
        if !list.iter().any(|(key, _)| *key == item_value) {
            list.push((item_value.clone(), item_label.clone()));
        }
    });
    let registry = context.labels;
    let drop_value = value.clone();
    use_drop(move || {
        let mut signal = registry;
        signal.write().retain(|(key, _)| *key != drop_value);
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
