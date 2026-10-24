//! `motion/shared-layout-bg.tsx` — a hover pill that morphs between rows.
//!
//! The source clones its children and injects the pill into the hovered one; a
//! Dioxus `Element` cannot be iterated, so the same shape is expressed as a list
//! of [`SharedLayoutItem`]s. The pill itself is handed between rows by
//! `layoutId`, which in this port is `beui_core::flip`'s registry: the new node
//! is pinned to the box the old one was last measured at and springs to its own.
//!
//! The source's exit fade on the pill wrapper is not ported (the wrapper
//! unmounts immediately when the pointer leaves the list); the enter fade is.

use crate::prelude::*;
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::platform::next_scope_id;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

/// One row of a [`SharedLayoutBg`].
#[derive(Clone, PartialEq)]
pub struct SharedLayoutItem {
    pub id: String,
    pub content: Element,
    pub class: Option<String>,
}

impl SharedLayoutItem {
    /// A row with content and no extra class.
    pub fn new(id: impl Into<String>, content: Element) -> Self {
        Self {
            id: id.into(),
            content,
            class: None,
        }
    }

    /// Add a class to the row wrapper.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }
}

/// The semantic container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SharedLayoutTag {
    #[default]
    Div,
    Ul,
}

/// Props for [`SharedLayoutBg`].
#[derive(Props, Clone, PartialEq)]
pub struct SharedLayoutBgProps {
    pub items: Vec<SharedLayoutItem>,
    #[props(default)]
    pub r#as: SharedLayoutTag,
    /// Tailwind class applied to the moving pill.
    #[props(default)]
    pub pill_class: Option<String>,
    /// Horizontal inset of the pill relative to each row (px). Defaults to `20`.
    #[props(default = 20.0)]
    pub inset: f64,
    /// Optional positioning override for the pill wrapper inside each item.
    #[props(default)]
    pub pill_container_class: Option<String>,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub on_mouse_leave: Option<EventHandler<()>>,
    /// Optional ARIA role for the container (e.g. `tree`, `listbox`).
    #[props(default)]
    pub role: Option<String>,
    #[props(default)]
    pub aria_label: Option<String>,
    #[props(default)]
    pub aria_multiselectable: Option<String>,
}

/// A vertical list with a shared hover background.
#[component]
pub fn SharedLayoutBg(props: SharedLayoutBgProps) -> Element {
    let SharedLayoutBgProps {
        items,
        r#as,
        pill_class,
        inset,
        pill_container_class,
        class,
        on_mouse_leave,
        role,
        aria_label,
        aria_multiselectable,
    } = props;

    let mut active = use_signal(|| None::<String>);
    let uid = use_hook(next_scope_id);
    let layout_id = format!("shared-bg-{uid}");
    let pill_class = cn!(
        "pointer-events-none h-full w-full rounded-2xl bg-muted/80",
        pill_class
    );
    let pill_container_class = cn!(
        "pointer-events-none absolute inset-y-0",
        pill_container_class
    );
    let active_id = active();
    let class = cn!("flex w-full flex-col", class);

    let rows = rsx! {
        for item in items {
            SharedLayoutRow {
                key: "{item.id}",
                id: item.id.clone(),
                active_id: active_id.clone(),
                layout_id: layout_id.clone(),
                pill_class: pill_class.clone(),
                pill_container_class: pill_container_class.clone(),
                inset,
                class: item.class.clone(),
                on_hover: move |id: String| active.set(Some(id)),
                {item.content}
            }
        }
    };

    let leave = move |_| {
        active.set(None);
        if let Some(handler) = &on_mouse_leave {
            handler.call(());
        }
    };

    match r#as {
        SharedLayoutTag::Div => {
            rsx! { div { class, role, aria_label, "aria-multiselectable": aria_multiselectable, onmouseleave: leave, {rows} } }
        }
        SharedLayoutTag::Ul => {
            rsx! { ul { class, role, aria_label, "aria-multiselectable": aria_multiselectable, onmouseleave: leave, {rows} } }
        }
    }
}

/// One row: the pill wrapper (when any row is hovered) plus the content.
#[component]
fn SharedLayoutRow(
    id: String,
    active_id: Option<String>,
    layout_id: String,
    pill_class: String,
    pill_container_class: String,
    inset: f64,
    class: Option<String>,
    on_hover: EventHandler<String>,
    children: Element,
) -> Element {
    let is_active = active_id.as_deref() == Some(id.as_str());
    let row_class = cn!("relative", class);
    let hover_id = id.clone();

    rsx! {
        div {
            class: row_class,
            onmouseenter: move |_| on_hover.call(hover_id.clone()),
            if active_id.is_some() {
                PillContainer {
                    class: pill_container_class,
                    style: format!("left:-{inset}px;right:-{inset}px"),
                    if is_active {
                        SharedPill { layout_id, pill_class }
                    }
                }
            }
            div { class: "relative z-10", {children} }
        }
    }
}

/// The fading wrapper the pill lives in.
#[component]
fn PillContainer(class: String, style: String, children: Element) -> Element {
    let mut progress = use_motion(0.0f32).expect("0.0 is finite");
    use_effect(move || {
        let _ = progress.animate_to(1.0, beui_core::ease::tween_ease_out(200));
    });
    let p = f64::from(*progress.current().read());
    let style = format!("{style};opacity:{p};filter:blur({}px)", 6.0 * (1.0 - p));
    rsx! {
        div { class, style, {children} }
    }
}

/// The pill itself, handed between rows through the flip registry.
#[component]
fn SharedPill(layout_id: String, pill_class: String) -> Element {
    let flip = use_flip(
        layout_id.clone(),
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(AnimationConfig::spring(SPRING_LAYOUT)),
    );
    rsx! {
        div {
            "data-beui-flip": flip.scope(),
            class: pill_class,
            style: flip.style_with(""),
        }
    }
}
