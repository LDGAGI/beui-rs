//! `motion/preview-rail.tsx` — a tick rail with a preview card that follows the
//! pointer, keyboard focus or a tap.
//!
//! The three "displayed" sources are kept apart (`hovered`, `pinned`,
//! `focused`) exactly as the source does, because they end in different ways: a
//! stray mouse move must not clear a tick the keyboard or a tap chose. A tap
//! pins the card and the next outside pointerdown clears it — `use_dismiss`.
//!
//! The card is handed between rows by `layoutId` (beui-core's flip registry).
//! The source's `mode="wait"` exit between two previews is not ported: the card
//! content enters (opacity/y/blur) but a swap replaces it without an exit frame.

use crate::prelude::*;
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::{
    use_dismiss, use_hover_gesture, use_tap_gesture, DismissOptions, HoverGesture, TapGesture,
};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

/// One tick in the rail.
#[derive(Clone, PartialEq)]
pub struct PreviewRailItem {
    pub id: String,
    pub label: String,
    pub aria_label: Option<String>,
    pub description: Option<String>,
    pub href: Option<String>,
    pub target: Option<String>,
    pub rel: Option<String>,
}

impl PreviewRailItem {
    /// An item with just an id and label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            aria_label: None,
            description: None,
            href: None,
            target: None,
            rel: None,
        }
    }

    /// Attach a description shown in the default preview card.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Turn the tick into a link.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }
}

/// Rail direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreviewRailOrientation {
    #[default]
    Vertical,
    Horizontal,
}

/// Which side the card sits on (vertical only).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreviewSide {
    Before,
    #[default]
    After,
}

/// Props for [`PreviewRail`].
#[derive(Props, Clone, PartialEq)]
pub struct PreviewRailProps {
    pub items: Vec<PreviewRailItem>,
    #[props(default = "Section navigation".to_string())]
    pub label: String,
    #[props(default)]
    pub orientation: PreviewRailOrientation,
    #[props(default)]
    pub active_id: Option<String>,
    #[props(default)]
    pub default_active_id: Option<String>,
    #[props(default)]
    pub on_active_change: Option<EventHandler<String>>,
    #[props(default)]
    pub on_item_select: Option<EventHandler<PreviewRailItem>>,
    /// Custom preview renderer.
    #[props(default)]
    pub render_preview: Option<Callback<PreviewRailItem, Element>>,
    #[props(default = true)]
    pub show_preview: bool,
    #[props(default)]
    pub preview_side: PreviewSide,
    #[props(default)]
    pub highlight_active: bool,
    /// Row height (vertical) or column width (horizontal) in px. Defaults to `24`.
    #[props(default = 24.0)]
    pub item_size: f64,
    #[props(default)]
    pub children: Option<Element>,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub rail_class: Option<String>,
    #[props(default)]
    pub preview_container_class: Option<String>,
    #[props(default)]
    pub preview_class: Option<String>,
}

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// A tick whose scale follows the distance from the displayed row.
#[component]
fn RailTick(scale: f64, horizontal: bool, highlighted: bool) -> Element {
    let mut motion = use_motion(scale as f32).expect("a finite scale");
    use_effect(use_reactive((&scale,), move |(scale,)| {
        let _ = motion.animate_to(scale as f32, AnimationConfig::spring(SPRING_LAYOUT));
    }));
    let value = f64::from(current(motion));
    let style = if horizontal {
        format!("transform:scaleY({value})")
    } else {
        format!("transform:scaleX({value})")
    };
    let class = cn!(
        "block bg-current",
        if horizontal {
            "h-12 w-0.5 origin-bottom"
        } else {
            "h-0.5 w-12 origin-left"
        },
        if highlighted { "text-foreground" } else { "" },
    );
    rsx! {
        span { "data-slot": "preview-rail-tick", aria_hidden: "true", class, style }
    }
}

/// A preview card, handed between rows through the flip registry.
#[component]
fn PreviewRailCard(
    item: PreviewRailItem,
    render_preview: Option<Callback<PreviewRailItem, Element>>,
    preview_class: Option<String>,
    horizontal: bool,
    preview_side: PreviewSide,
    layout_id: String,
    reduce: bool,
) -> Element {
    let flip = use_flip(
        item.id.clone(),
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(AnimationConfig::spring(SPRING_LAYOUT)),
    );
    let class = cn!(
        if horizontal {
            "absolute bottom-12 left-1/2 w-72 -translate-x-1/2".to_string()
        } else {
            cn!(
                "w-full max-w-sm",
                if preview_side == PreviewSide::Before {
                    "ml-auto"
                } else {
                    ""
                },
            )
        },
        preview_class,
    );
    let item_id = item.id.clone();
    rsx! {
        div { class, "data-beui-flip": flip.scope(), style: flip.style_with(""),
            PreviewCardContent { key: "{item_id}", item, render_preview, reduce }
        }
    }
}

/// The card content, entering on every swap.
#[component]
fn PreviewCardContent(
    item: PreviewRailItem,
    render_preview: Option<Callback<PreviewRailItem, Element>>,
    reduce: bool,
) -> Element {
    let mut progress = use_motion(0.0f32).expect("0.0 is finite");
    use_effect(move || {
        let _ = progress.animate_to(1.0, beui_core::ease::tween_ease_out(180));
    });
    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = if reduce {
        format!("opacity:{p};transform:none;filter:none")
    } else {
        format!(
            "opacity:{p};transform:translateY({}px);filter:blur({}px)",
            4.0 * (1.0 - p),
            6.0 * (1.0 - p)
        )
    };
    rsx! {
        div { style,
            if let Some(render) = render_preview {
                {render.call(item)}
            } else {
                DefaultPreview { item }
            }
        }
    }
}

/// `DefaultPreview`.
#[component]
fn DefaultPreview(item: PreviewRailItem) -> Element {
    rsx! {
        div {
            "data-slot": "preview-rail-card",
            class: "rounded-2xl border border-border bg-card p-4 shadow-sm",
            p { "data-slot": "preview-rail-title", class: "font-medium text-card-foreground", "{item.label}" }
            if let Some(description) = item.description.clone() {
                div {
                    "data-slot": "preview-rail-description",
                    class: "mt-1 text-sm leading-6 text-muted-foreground",
                    "{description}"
                }
            }
        }
    }
}

/// One tick. A plain function, not a component: it only wires handlers, and a
/// component would have to carry the non-`PartialEq` gesture handles through
/// its props.
#[allow(clippy::too_many_arguments)]
fn rail_item(
    item: &PreviewRailItem,
    index: usize,
    selected: &str,
    highlighted: &str,
    displayed_index: i64,
    horizontal: bool,
    item_size: f64,
    hover: HoverGesture,
    tap: TapGesture<bool>,
    pinned: Signal<Option<String>>,
    hovered: Signal<Option<String>>,
    focused: Signal<Option<String>>,
    pinned_open: Signal<bool>,
    active_is_controlled: bool,
    internal_active: Signal<String>,
    on_active_change: Option<EventHandler<String>>,
    on_item_select: Option<EventHandler<PreviewRailItem>>,
) -> Element {
    let id = item.id.clone();
    let selected_item = item.id == selected;
    let highlighted_item = item.id == highlighted;
    let distance = if displayed_index < 0 {
        f64::INFINITY
    } else {
        (index as i64 - displayed_index).unsigned_abs() as f64
    };
    let scale = if highlighted_item {
        1.0
    } else if distance == 1.0 {
        0.68
    } else if distance == 2.0 {
        0.44
    } else {
        0.25
    };
    let shared_class = cn!(
        "relative flex text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        if horizontal {
            "h-12 w-6 items-end justify-center"
        } else {
            "h-6 w-12 items-center"
        },
    );
    let shared_style = if horizontal {
        format!("width:{item_size}px")
    } else {
        format!("height:{item_size}px")
    };
    let aria_label = item
        .aria_label
        .clone()
        .unwrap_or_else(|| item.label.clone());
    let item_for_select = item.clone();
    let href = item.href.clone();

    let on_enter = {
        let mut hovered = hovered;
        let hover = hover.clone();
        let id = id.clone();
        move |event: Event<PointerData>| {
            if hover.enter(&event) {
                hovered.set(Some(id.clone()));
            }
        }
    };
    let on_down = {
        let mut focused = focused;
        let tap = tap.clone();
        let id = id.clone();
        move |event: Event<PointerData>| {
            tap.start(&event, pinned() == Some(id.clone()));
            focused.set(None);
        }
    };
    let on_cancel = {
        let tap = tap.clone();
        move |_| tap.drop_record()
    };
    let on_key = {
        let tap = tap.clone();
        move |_| tap.drop_record()
    };
    let on_focus = {
        let mut focused = focused;
        let id = id.clone();
        move |_| focused.set(Some(id.clone()))
    };
    let mut on_click = {
        let mut pinned = pinned;
        let mut pinned_open = pinned_open;
        let mut internal_active = internal_active;
        let mut focused = focused;
        let tap = tap.clone();
        let id = id.clone();
        let item = item_for_select;
        move |event: Option<Event<MouseData>>| {
            let gesture = tap.take();
            let tapped = gesture
                .as_ref()
                .map(|record| record.pointer_type != "mouse")
                .unwrap_or(false);
            if tapped {
                if item.href.is_some()
                    && !gesture.as_ref().map(|record| record.state).unwrap_or(false)
                {
                    if let Some(event) = &event {
                        event.prevent_default();
                    }
                    pinned.set(Some(id.clone()));
                    pinned_open.set(true);
                    return;
                }
                pinned.set(Some(id.clone()));
                pinned_open.set(true);
            }
            focused.set(None);
            if !active_is_controlled {
                internal_active.set(id.clone());
            }
            if let Some(handler) = &on_active_change {
                handler.call(id.clone());
            }
            if let Some(handler) = &on_item_select {
                handler.call(item.clone());
            }
        }
    };

    if let Some(href) = href {
        rsx! {
            a {
                key: "{id}",
                "data-slot": "preview-rail-item",
                href,
                target: item.target.clone(),
                rel: item.rel.clone(),
                aria_label,
                aria_current: if selected_item { Some("page") } else { None },
                class: shared_class,
                style: shared_style,
                onpointerenter: on_enter,
                onpointerdown: on_down,
                onpointercancel: on_cancel,
                onkeydown: on_key,
                onfocus: on_focus,
                onclick: move |event: Event<MouseData>| on_click(Some(event)),
                RailTick { scale, horizontal, highlighted: highlighted_item }
            }
        }
    } else {
        rsx! {
            button {
                key: "{id}",
                "data-slot": "preview-rail-item",
                r#type: "button",
                aria_label,
                aria_current: if selected_item { Some("location") } else { None },
                class: shared_class,
                style: shared_style,
                onpointerenter: on_enter,
                onpointerdown: on_down,
                onpointercancel: on_cancel,
                onkeydown: on_key,
                onfocus: on_focus,
                onclick: move |_| on_click(None),
                RailTick { scale, horizontal, highlighted: highlighted_item }
            }
        }
    }
}

/// A rail of ticks with a preview card.
#[component]
pub fn PreviewRail(props: PreviewRailProps) -> Element {
    let PreviewRailProps {
        items,
        label,
        orientation,
        active_id,
        default_active_id,
        on_active_change,
        on_item_select,
        render_preview,
        show_preview,
        preview_side,
        highlight_active,
        item_size,
        children,
        class,
        rail_class,
        preview_container_class,
        preview_class,
    } = props;

    let reduce = use_reduced_motion();
    let horizontal = orientation == PreviewRailOrientation::Horizontal;
    let internal_active = use_signal(|| {
        default_active_id
            .clone()
            .or_else(|| items.first().map(|item| item.id.clone()))
            .unwrap_or_default()
    });
    let hovered = use_signal(|| None::<String>);
    let mut pinned = use_signal(|| None::<String>);
    let mut pinned_open = use_signal(|| false);
    let mut focused = use_signal(|| None::<String>);
    let tap = use_tap_gesture::<bool>();
    let hover = use_hover_gesture();

    let scope = use_dismiss(
        pinned_open,
        EventHandler::new(move |_| {
            pinned.set(None);
            pinned_open.set(false);
            focused.set(None);
        }),
        DismissOptions::default(),
    );

    let requested = match active_id.clone() {
        Some(active) => active,
        None => internal_active(),
    };
    let selected = if items.iter().any(|item| item.id == requested) {
        requested
    } else {
        items
            .first()
            .map(|item| item.id.clone())
            .unwrap_or_default()
    };
    let displayed = hovered().or(pinned()).or(focused()).unwrap_or_default();
    let highlighted = if !displayed.is_empty() {
        displayed.clone()
    } else if highlight_active {
        selected.clone()
    } else {
        String::new()
    };
    let displayed_index = items
        .iter()
        .position(|item| item.id == highlighted)
        .map(|index| index as i64)
        .unwrap_or(-1);
    let row_template = if items.is_empty() {
        None
    } else {
        Some(format!("repeat({}, {item_size}px)", items.len()))
    };
    let layout_id = "preview-rail-card".to_string();

    let root_class = cn!(
        "isolate relative flex w-full overflow-visible",
        if horizontal {
            "min-h-64 flex-col items-center justify-center"
        } else {
            "min-h-80"
        },
        class,
    );
    let nav_class = cn!(
        "relative z-10 grid shrink-0",
        if horizontal {
            "h-12 w-fit max-w-full self-center justify-center"
        } else {
            "w-12 content-center"
        },
        rail_class,
    );
    let nav_style = match &row_template {
        Some(template) if horizontal => format!("grid-template-columns:{template}"),
        Some(template) => format!("grid-template-rows:{template}"),
        None => String::new(),
    };
    let preview_container_class = cn!(
        "pointer-events-none absolute z-50 grid",
        if horizontal {
            "top-1/2 left-1/2 h-5 w-fit max-w-full -translate-x-1/2 -translate-y-1/2 justify-center"
                .to_string()
        } else if preview_side == PreviewSide::Before {
            "inset-y-0 right-16 left-4 content-center".to_string()
        } else {
            "inset-y-0 right-4 left-16 content-center".to_string()
        },
        preview_container_class,
    );
    let preview_container_style = match &row_template {
        Some(template) if horizontal => format!("grid-template-columns:{template}"),
        Some(template) => format!("grid-template-rows:{template}"),
        None => String::new(),
    };
    let reduce_value = reduce();

    rsx! {
        div {
            "data-beui-scope": scope.attr(),
            class: root_class,
            onblur: move |_| {
                focused.set(None);
                pinned.set(None);
                pinned_open.set(false);
            },
            nav {
                aria_label: label,
                class: nav_class,
                style: nav_style,
                onpointerleave: {
                    let mut hovered = hovered;
                    let hover = hover.clone();
                    move |event: Event<PointerData>| {
                        if hover.leave(&event) {
                            hovered.set(None);
                        }
                    }
                },
                for (index, item) in items.iter().enumerate() {
                    {rail_item(
                        item,
                        index,
                        &selected,
                        &highlighted,
                        displayed_index,
                        horizontal,
                        item_size,
                        hover.clone(),
                        tap.clone(),
                        pinned,
                        hovered,
                        focused,
                        pinned_open,
                        active_id.is_some(),
                        internal_active,
                        on_active_change,
                        on_item_select,
                    )}
                }
            }
            if show_preview {
                div {
                    aria_hidden: "true",
                    class: preview_container_class,
                    style: preview_container_style,
                    for item in items.iter() {
                        div {
                            key: "{item.id}",
                            style: if horizontal {
                                format!("width:{item_size}px")
                            } else {
                                format!("height:{item_size}px")
                            },
                            class: cn!("relative flex items-center", if horizontal { "justify-center" } else { "" }),
                            if item.id == displayed {
                                PreviewRailCard {
                                    item: item.clone(),
                                    render_preview,
                                    preview_class: preview_class.clone(),
                                    horizontal,
                                    preview_side,
                                    layout_id: layout_id.clone(),
                                    reduce: reduce_value,
                                }
                            }
                        }
                    }
                }
            }
            if let Some(children) = children {
                div { class: "min-h-0 min-w-0 flex-1", {children} }
            }
        }
    }
}
