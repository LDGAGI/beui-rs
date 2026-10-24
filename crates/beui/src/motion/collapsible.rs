//! `components/motion/collapsible.tsx` — the disclosure every accordion is built on.
//!
//! The height change is a `layout` animation, so it goes through
//! `beui-core::flip`: the content box jumps to its new height in the same commit
//! and a transform that starts as the inverse of the change decays to nothing.
//! The closed box is one pixel tall rather than zero, exactly as the source sets
//! it, so the projection has a continuous box to move from.
//!
//! Deviations from the source:
//!
//! - motion's nested projection is not ported, so the single `flip` sits on the
//!   root and its children scale with it for the length of the animation;
//! - the source measures the content with a `ResizeObserver` and writes an exact
//!   pixel height; here the open box is `h-auto`, which needs no observer and is
//!   what the flip measures anyway;
//! - `CollapsibleTrigger`'s `render` (asChild) prop is not ported — Dioxus has no
//!   `cloneElement`.

use dioxus::prelude::*;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

use crate::icons::ChevronDownIcon;
use crate::motion::Slot;
use beui_core::ease::{tween_ease_out, EASE_OUT_CSS, SPRING_LAYOUT};
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::platform;
use beui_core::{cn, use_reduced_motion};

/// `CollapsibleState` — open/disabled plus the two actions.
#[derive(Clone, Copy)]
pub struct CollapsibleState {
    pub open: bool,
    pub disabled: bool,
    set_open: EventHandler<bool>,
    toggle: EventHandler<()>,
}

impl CollapsibleState {
    /// `setOpen(open)`.
    pub fn set_open(&self, open: bool) {
        self.set_open.call(open);
    }

    /// `toggle()`.
    pub fn toggle(&self) {
        self.toggle.call(());
    }
}

#[derive(Clone)]
struct CollapsibleContext {
    state: Signal<CollapsibleState>,
    content_id: String,
    trigger_id: String,
}

fn ctx() -> CollapsibleContext {
    use_context::<CollapsibleContext>()
}

/// `useCollapsible()` — shared state and actions for custom children.
pub fn use_collapsible() -> CollapsibleState {
    let context = ctx();
    (context.state)()
}

/// `Collapsible`.
#[component]
pub fn Collapsible(
    #[props(default = None)] open: Option<bool>,
    #[props(default = false)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = false)] disabled: bool,
    /// Layout classes for the inner wrapper, separate from the animated surface.
    #[props(default = None)]
    content_class: Option<String>,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] style: Option<String>,
    children: Element,
) -> Element {
    let mut internal_open = use_signal(|| default_open);
    let controlled = open.is_some();
    let fallback_open = internal_open();
    let current_open = open.unwrap_or(fallback_open);
    let reduce = use_reduced_motion()();

    let set_open = EventHandler::new(move |next: bool| {
        if disabled || next == current_open {
            return;
        }
        if !controlled {
            internal_open.set(next);
        }
        if let Some(callback) = on_open_change {
            callback.call(next);
        }
    });
    let toggle = EventHandler::new(move |_| set_open.call(!current_open));

    let mut state = use_signal(|| CollapsibleState {
        open: current_open,
        disabled,
        set_open,
        toggle,
    });
    if state.peek().open != current_open || state.peek().disabled != disabled {
        state.set(CollapsibleState {
            open: current_open,
            disabled,
            set_open,
            toggle,
        });
    }
    let scope = use_hook(platform::next_scope_id);
    use_context_provider(|| CollapsibleContext {
        state,
        content_id: format!("beui-collapsible-{scope}-content"),
        trigger_id: format!("beui-collapsible-{scope}-trigger"),
    });

    // `layout` on the root: the one flip in the family, since its box is the one
    // whose height actually changes.
    let flip = use_flip(
        current_open,
        FlipOptions::default().transition(if reduce {
            tween_ease_out(1)
        } else {
            AnimationConfig::spring(SPRING_LAYOUT)
        }),
    );

    rsx! {
        div {
            "data-slot": "collapsible",
            "data-state": if current_open { "open" } else { "closed" },
            "data-disabled": if disabled { Some("true") } else { None },
            "data-beui-flip": flip.scope(),
            style: flip.style_with(&format!("transform-origin:50% 0%;{}", style.as_deref().unwrap_or(""))),
            class: cn!("relative w-full", class.as_deref().unwrap_or("")),
            div { class: cn!("min-w-0", content_class.as_deref().unwrap_or("")), {children} }
        }
    }
}

/// `CollapsibleTrigger`.
#[component]
pub fn CollapsibleTrigger(
    #[props(default = None)] class: Option<String>,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let context = ctx();
    let state = (context.state)();
    let unavailable = state.disabled || disabled;

    rsx! {
        button {
            id: "{context.trigger_id}",
            r#type: "button",
            disabled: unavailable,
            "aria-expanded": state.open,
            "aria-controls": "{context.content_id}",
            "data-slot": "collapsible-trigger",
            "data-state": if state.open { "open" } else { "closed" },
            "data-disabled": if unavailable { Some("true") } else { None },
            class: cn!(
                "flex min-h-10 w-full items-center gap-3 rounded-xl px-3 py-2 text-left text-sm font-medium outline-none transition-colors hover:bg-muted/60 focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
                class.as_deref().unwrap_or(""),
            ),
            onclick: move |event| {
                let enabled = event.default_action_enabled();
                if let Some(callback) = onclick {
                    callback.call(event);
                }
                if enabled && !unavailable {
                    state.toggle();
                }
            },
            {children}
        }
    }
}

/// `CollapsibleContent`.
#[component]
pub fn CollapsibleContent(
    #[props(default = None)] class: Option<String>,
    /// Padding and layout classes for the measured content, inside the clip.
    #[props(default = None)]
    content_class: Option<String>,
    #[props(default = None)] style: Option<String>,
    children: Element,
) -> Element {
    let context = ctx();
    let state = (context.state)();
    let reduce = use_reduced_motion()();

    // Closing while focus is inside hands it back to the trigger, as the source
    // does with its content ref.
    let content_id = context.content_id.clone();
    let trigger_id = context.trigger_id.clone();
    use_effect(move || {
        if state.open {
            return;
        }
        let content = platform::js_string(&content_id);
        let trigger = platform::js_string(&trigger_id);
        platform::fire(&format!(
            "(() => {{ const c = document.getElementById({content}); \
             if (c && c.contains(document.activeElement)) \
             document.getElementById({trigger})?.focus({{ preventScroll: true }}); }})()"
        ));
    });

    // The closed box is 1px so the flip has a continuous box, not a zero one.
    let duration = if state.open { 180 } else { 120 };
    let inner_style = if reduce {
        String::new()
    } else {
        format!("transition:opacity {duration}ms {EASE_OUT_CSS}")
    };

    rsx! {
        div {
            id: "{context.content_id}",
            role: "region",
            "aria-labelledby": "{context.trigger_id}",
            "aria-hidden": if state.open { None } else { Some("true") },
            inert: if state.open { None } else { Some("true") },
            "data-slot": "collapsible-content",
            "data-state": if state.open { "open" } else { "closed" },
            style: style.clone().unwrap_or_default(),
            class: cn!(
                "overflow-hidden",
                if state.open { "h-auto" } else { "h-px" },
                class.as_deref().unwrap_or(""),
            ),
            div {
                style: inner_style,
                class: cn!(
                    "flow-root",
                    if state.open { "opacity-100" } else { "opacity-0" },
                    content_class.as_deref().unwrap_or(""),
                ),
                {children}
            }
        }
    }
}

/// `CollapsibleIndicator` — the rotating chevron.
#[component]
pub fn CollapsibleIndicator(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = ctx();
    let reduce = use_reduced_motion()();
    let rotation = use_motion(0.0f32).expect("0.0 is a finite animation value");

    use_effect(move || {
        let target = if (context.state)().open { 180.0 } else { 0.0 };
        let mut handle = rotation;
        let _ = handle.animate_to(
            target,
            if reduce {
                tween_ease_out(1)
            } else {
                AnimationConfig::spring(SPRING_LAYOUT)
            },
        );
    });

    rsx! {
        span {
            "aria-hidden": "true",
            "data-slot": "collapsible-indicator",
            style: format!("transform:rotate({}deg)", rotation.get_value()),
            class: cn!(
                "ml-auto inline-flex shrink-0 items-center justify-center text-muted-foreground",
                class.as_deref().unwrap_or(""),
            ),
            Slot { default: rsx! { ChevronDownIcon { class: "size-4" } }, {children} }
        }
    }
}
