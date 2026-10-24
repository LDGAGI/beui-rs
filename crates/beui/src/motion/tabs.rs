//! `components/motion/tabs.tsx` — one indicator that travels between tabs.
//!
//! The indicator is a `layoutId` shape: it is rendered *inside* the active
//! trigger, so selecting another tab unmounts it in one button and mounts it in
//! another, and the new node starts from the box the old one was last measured
//! at. That is `beui-core::flip` with a shared `layout_id`.
//!
//! The pill and segment variants also paint the active label twice — once in the
//! button's muted colour, once in `text-primary-foreground` on an `inert`
//! overlay clipped to the indicator — so the text hands its colour over at the
//! indicator's edge. The clip is driven by `src/js/tabs.js`, the same
//! per-frame read/write loop the source runs in `frame.postRender`.
//!
//! Deviations from the source:
//!
//! - the scrollable-list machinery (overflow detection, edge arrows, blur
//!   masks, auto-reveal of the selected tab) is not ported yet: the list still
//!   scrolls horizontally, but without the arrows/edge fade;
//! - labels only take their initial clip from `active`; the script narrows them
//!   once the indicator moves.

use dioxus::prelude::*;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

use beui_core::ease::tween_ease_out;
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::platform;
use beui_core::{cn, use_reduced_motion};

/// `tabs.tsx`'s indicator spring, copied verbatim. A per-component spring, not
/// one of the shared `lib/ease.ts` tokens.
const SPRING_TABS: dioxus_motion::prelude::Spring = dioxus_motion::prelude::Spring {
    stiffness: 245.0,
    damping: 36.0,
    mass: 1.2,
};

/// The three shapes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabsVariant {
    #[default]
    Pill,
    Underline,
    Segment,
}

impl TabsVariant {
    /// The `TabsList` classes, verbatim.
    const fn list_class(self) -> &'static str {
        match self {
            Self::Pill => "inline-flex items-center gap-1 rounded-full bg-card p-1",
            Self::Underline => "inline-flex items-center gap-1 border-b border-border",
            Self::Segment => "inline-flex items-center gap-0 rounded-lg bg-card p-0.5",
        }
    }

    /// The trigger radius for the two filled shapes.
    const fn radius(self) -> &'static str {
        match self {
            Self::Segment => "rounded-md",
            _ => "rounded-full",
        }
    }

    /// The indicator's own corner radius (the flip counter-scales the corners by
    /// this value so the pill does not stretch them as it springs).
    const fn indicator_radius(self) -> f64 {
        match self {
            Self::Segment => 8.0,
            _ => 9999.0,
        }
    }
}

#[derive(Clone)]
struct TabsContext {
    value: Signal<String>,
    set_value: EventHandler<String>,
    layout_id: String,
    variant: TabsVariant,
}

fn use_tabs() -> TabsContext {
    use_context::<TabsContext>()
}

/// `Tabs`.
#[component]
pub fn Tabs(
    #[props(default = None)] default_value: Option<String>,
    /// Controlled selection. `None` means uncontrolled (`default_value`).
    #[props(default = None)]
    value: Option<String>,
    #[props(default = None)] on_value_change: Option<EventHandler<String>>,
    #[props(default)] variant: TabsVariant,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let mut internal = use_signal(|| default_value.unwrap_or_default());
    let controlled = value.is_some();
    let fallback = internal();
    let current = value.clone().unwrap_or(fallback);

    let mut value_signal = use_signal(|| current.clone());
    if *value_signal.peek() != current {
        value_signal.set(current.clone());
    }

    let set_value = EventHandler::new(move |next: String| {
        if !controlled {
            internal.set(next.clone());
        }
        if let Some(callback) = on_value_change {
            callback.call(next);
        }
    });

    let layout_id = use_hook(|| format!("beui-tabs-{}", platform::next_scope_id()));
    use_context_provider(|| TabsContext {
        value: value_signal,
        set_value,
        layout_id,
        variant,
    });

    // `window.__beuiTabsClip`; idempotent, so one script for every Tabs.
    use_effect(|| {
        platform::fire(include_str!("../js/tabs.js"));
    });

    rsx! {
        div { class: class.clone().unwrap_or_default(), {children} }
    }
}

/// `TabsList`.
#[component]
pub fn TabsList(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] wrapper_class: Option<String>,
    children: Element,
) -> Element {
    let context = use_tabs();
    let list_id = format!("{}-list", context.layout_id);
    let effect_list_id = list_id.clone();
    let reduce = use_reduced_motion()();

    // Re-clip whenever the selection changes (and once on mount).
    use_effect(move || {
        let _selection = (context.value)();
        let id = platform::js_string(&effect_list_id);
        platform::fire(&format!(
            "window.__beuiTabsClip && window.__beuiTabsClip(document.getElementById({id}), {reduce})"
        ));
    });

    rsx! {
        div {
            class: cn!(
                "relative isolate flex w-full max-w-full min-w-0 items-center",
                wrapper_class.as_deref().unwrap_or(""),
            ),
            div { class: "w-full min-w-0 overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden",
                div {
                    id: "{list_id}",
                    role: "tablist",
                    class: cn!(context.variant.list_class(), "w-max", class.as_deref().unwrap_or("")),
                    {children}
                }
            }
        }
    }
}

/// `TabsTrigger`.
#[component]
pub fn TabsTrigger(
    value: String,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] indicator_class: Option<String>,
    #[props(default = None)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let context = use_tabs();
    let selection = (context.value)();
    let active = selection == value;

    let activate = {
        let value = value.clone();
        let context = context.clone();
        move |event: MouseEvent| {
            let enabled = event.default_action_enabled();
            if let Some(callback) = onclick {
                callback.call(event);
            }
            if enabled {
                context.set_value.call(value.clone());
            }
        }
    };

    if context.variant == TabsVariant::Underline {
        return rsx! {
            button {
                r#type: "button",
                role: "tab",
                "aria-selected": active,
                class: cn!(
                    "relative isolate -mb-px inline-flex min-h-[44px] shrink-0 items-center whitespace-nowrap px-3 pb-2.5 pt-1 text-sm font-medium transition-colors",
                    if active { "text-foreground" } else { "text-muted-foreground hover:text-foreground" },
                    class.as_deref().unwrap_or(""),
                ),
                onclick: activate,
                {children.clone()}
                if active {
                    TabsUnderlineIndicator {
                        layout_id: context.layout_id.clone(),
                        state: selection.clone(),
                        indicator_class: indicator_class.clone(),
                    }
                }
            }
        };
    }

    let initial_clip = if active {
        "inset(0)"
    } else {
        "inset(0 100% 0 0)"
    };
    let radius = context.variant.radius();

    rsx! {
        div { class: "relative shrink-0",
            if active {
                TabsIndicator {
                    layout_id: context.layout_id.clone(),
                    state: selection.clone(),
                    variant: context.variant,
                    indicator_class: indicator_class.clone(),
                }
            }
            button {
                r#type: "button",
                role: "tab",
                "aria-selected": active,
                "data-tabs-value": "{value}",
                class: cn!(
                    "relative z-10 inline-flex items-center justify-center whitespace-nowrap bg-transparent px-3.5 py-1.5 text-sm font-medium outline-none",
                    "text-muted-foreground hover:text-foreground",
                    radius,
                    class.as_deref().unwrap_or(""),
                ),
                onclick: activate,
                {children.clone()}
                span {
                    "data-tabs-label": "",
                    "aria-hidden": "true",
                    inert: "true",
                    style: format!("clip-path:{initial_clip}"),
                    class: "pointer-events-none absolute inset-0 inline-flex items-center justify-center text-primary-foreground [gap:inherit] [padding:inherit]",
                    {children}
                }
            }
        }
    }
}

/// The filled-shape indicator. Mounted per selection, so `layout_id` is what
/// makes it read the previous box out of the registry.
#[component]
fn TabsIndicator(
    layout_id: String,
    state: String,
    variant: TabsVariant,
    indicator_class: Option<String>,
) -> Element {
    let flip = use_flip(
        state,
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(AnimationConfig::spring(SPRING_TABS))
            .border_radius(variant.indicator_radius()),
    );
    let radius = if variant == TabsVariant::Segment {
        8
    } else {
        9999
    };

    rsx! {
        span {
            "data-tabs-indicator": "",
            "data-beui-flip": flip.scope(),
            style: flip.style_with(&format!("border-radius:{radius}px")),
            class: cn!("absolute inset-0 bg-primary", variant.radius(), indicator_class.as_deref().unwrap_or("")),
        }
    }
}

/// The underline variant's 1px rule.
#[component]
fn TabsUnderlineIndicator(
    layout_id: String,
    state: String,
    indicator_class: Option<String>,
) -> Element {
    let flip = use_flip(
        state,
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(AnimationConfig::spring(SPRING_TABS)),
    );

    rsx! {
        span {
            "data-tabs-indicator": "",
            "data-beui-flip": flip.scope(),
            style: flip.style_with(""),
            class: cn!("absolute bottom-0 left-0 right-0 h-px bg-primary", indicator_class.as_deref().unwrap_or("")),
        }
    }
}

/// `TabsContent` — the selected panel enters; the others stay mounted, hidden.
#[component]
pub fn TabsContent(
    value: String,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_tabs();
    let active = (context.value)() == value;

    if !active {
        return rsx! {
            div { hidden: true, class: class.clone().unwrap_or_default(), {children} }
        };
    }

    rsx! {
        TabsPanel { class, children }
    }
}

#[component]
fn TabsPanel(class: Option<String>, children: Element) -> Element {
    let reduce = use_reduced_motion()();
    let progress = use_motion(0.0f32).expect("0.0 is a finite animation value");
    use_effect(move || {
        let mut handle = progress;
        let _ = handle.animate_to(1.0, tween_ease_out(if reduce { 1 } else { 180 }));
    });

    let progress = progress.get_value();
    let y = if reduce { 0.0 } else { (1.0 - progress) * 4.0 };

    rsx! {
        div {
            style: format!("opacity:{progress};transform:translateY({y}px)"),
            class: cn!("mt-4", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}
