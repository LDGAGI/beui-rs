//! `motion/expandable-action-bar.tsx` — a rail of icon actions whose labels
//! unfold on hover, focus or a first tap.
//!
//! The source animates the label's `width: auto ↔ 0`; here the label's own
//! `width` springs from 0 to its measured natural width (the same adaptation as
//! `ExpandableControl`), so the bar grows with it and there is nothing to
//! counter-scale. The highlight is a `layoutId` pill handed between items
//! through `beui_core::flip`.
//!
//! The gesture split is the source's: a resting pointer expands the bar, a
//! finger's first tap expands it and runs nothing, and the consumed outside tap
//! closes it (`use_dismiss`).

use crate::prelude::*;
use crate::rect::{use_element_rect, ElementRect};
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::{
    use_dismiss, use_hover_gesture, use_tap_gesture, DismissOptions, HoverGesture, TapGesture,
};
use beui_core::platform::next_scope_id;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{
    use_motion, AnimationConfig, AnimationManager, MotionHandle, Spring, Time, TimeProvider,
};

/// `ITEM_TRANSITION`, verbatim.
const ITEM_SPRING: Spring = Spring {
    stiffness: 460.0,
    damping: 34.0,
    mass: 0.62,
};

/// `LABEL_TRANSITION`, verbatim.
const LABEL_SPRING: Spring = Spring {
    stiffness: 380.0,
    damping: 32.0,
    mass: 0.7,
};

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// The bar's pointer gestures, shared with the item components through context:
/// neither `HoverGesture` nor `TapGesture` implements `PartialEq`, so neither
/// can travel as a component prop.
#[derive(Clone)]
struct ActionBarGestures {
    tap: TapGesture<bool>,
    hover: HoverGesture,
}

/// Bar size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExpandableActionBarSize {
    Sm,
    #[default]
    Md,
}

impl ExpandableActionBarSize {
    fn track(self) -> &'static str {
        match self {
            ExpandableActionBarSize::Sm => "min-h-9 gap-1 p-1 text-xs",
            ExpandableActionBarSize::Md => "min-h-11 gap-1.5 p-1.5 text-sm",
        }
    }

    fn item(self) -> &'static str {
        match self {
            ExpandableActionBarSize::Sm => "h-7 min-w-7 px-1.5",
            ExpandableActionBarSize::Md => "h-8 min-w-8 px-2",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            ExpandableActionBarSize::Sm => "h-3.5 w-3.5",
            ExpandableActionBarSize::Md => "h-4 w-4",
        }
    }
}

/// One action.
#[derive(Clone, PartialEq)]
pub struct ExpandableActionBarItem {
    pub id: String,
    pub label: String,
    pub icon: Element,
    pub on_click: Option<EventHandler<()>>,
    pub disabled: bool,
    pub active: bool,
    pub badge: Option<String>,
    pub shortcut: Option<String>,
}

impl ExpandableActionBarItem {
    /// An action with a label and icon.
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Element) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            on_click: None,
            disabled: false,
            active: false,
            badge: None,
            shortcut: None,
        }
    }

    /// Attach a shortcut label.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Attach a badge.
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

/// Per-part class overrides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExpandableActionBarClassNames {
    pub root: Option<String>,
    pub track: Option<String>,
    pub item: Option<String>,
    pub active_item: Option<String>,
    pub icon: Option<String>,
    pub label: Option<String>,
    pub badge: Option<String>,
    pub shortcut: Option<String>,
}

/// Props for [`ExpandableActionBar`].
#[derive(Props, Clone, PartialEq)]
pub struct ExpandableActionBarProps {
    pub items: Vec<ExpandableActionBarItem>,
    #[props(default)]
    pub expanded: Option<bool>,
    #[props(default)]
    pub default_expanded: bool,
    #[props(default)]
    pub on_expanded_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub active_id: Option<String>,
    #[props(default)]
    pub on_action: Option<EventHandler<ExpandableActionBarItem>>,
    #[props(default)]
    pub size: ExpandableActionBarSize,
    /// Expand on a resting hover (and arm the two-tap touch path). Default `true`.
    #[props(default = true)]
    pub expand_on_hover: bool,
    #[props(default = true)]
    pub expand_on_focus: bool,
    /// Milliseconds to wait before collapsing. Defaults to `90`.
    #[props(default = 90.0)]
    pub collapse_delay: f64,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub class_names: ExpandableActionBarClassNames,
}

/// A rail of actions that unfolds its labels.
#[component]
pub fn ExpandableActionBar(props: ExpandableActionBarProps) -> Element {
    let ExpandableActionBarProps {
        items,
        expanded,
        default_expanded,
        on_expanded_change,
        active_id,
        on_action,
        size,
        expand_on_hover,
        expand_on_focus,
        collapse_delay,
        class,
        class_names,
    } = props;

    let reduce = use_reduced_motion();
    let layout_id = use_hook(|| format!("action-bar-{}", next_scope_id()));
    let mut internal_expanded = use_signal(|| default_expanded);
    let controlled = expanded.is_some();
    let is_expanded = match expanded {
        Some(expanded) => expanded,
        None => internal_expanded(),
    };
    let mut hovered = use_signal(|| None::<String>);
    let tap_expanded = use_signal(|| false);
    let mut collapse_generation = use_signal(|| 0u32);
    let tap = use_tap_gesture::<bool>();
    let hover = use_hover_gesture();
    use_context_provider(|| ActionBarGestures {
        tap: tap.clone(),
        hover: hover.clone(),
    });

    let set_expanded = EventHandler::new(move |next: bool| {
        if !controlled {
            internal_expanded.set(next);
        }
        if let Some(handler) = on_expanded_change {
            handler.call(next);
        }
    });
    let open = EventHandler::new(move |_| {
        let next = *collapse_generation.peek() + 1;
        collapse_generation.set(next);
        set_expanded.call(true);
    });
    let close = EventHandler::new(move |_| {
        let run = *collapse_generation.peek() + 1;
        collapse_generation.set(run);
        let collapse_generation = collapse_generation;
        let set_expanded = set_expanded;
        let mut hovered = hovered;
        let mut tap_expanded = tap_expanded;
        spawn(async move {
            if Time::delay(dioxus_motion::Duration::from_millis(
                collapse_delay.round() as u64
            ))
            .await
            .is_err()
            {
                return;
            }
            if *collapse_generation.peek() == run {
                set_expanded.call(false);
                hovered.set(None);
                tap_expanded.set(false);
            }
        });
    });

    // The consumed outside tap closes the bar a touch left open.
    let mut dismiss_open = use_signal(|| false);
    use_effect(use_reactive((&is_expanded,), move |(is_expanded,)| {
        dismiss_open.set(tap_expanded() && is_expanded);
    }));
    let scope = use_dismiss(
        dismiss_open,
        EventHandler::new(move |_| close.call(())),
        DismissOptions::default().consume(),
    );

    let active_item_id = active_id.clone().or_else(|| {
        items
            .iter()
            .find(|item| item.active)
            .map(|item| item.id.clone())
    });
    let highlight_id = hovered().or(active_item_id);
    let reduce_value = reduce();
    let class = cn!("inline-flex max-w-full", class_names.root.clone(), class);
    let track_class = cn!(
        "scrollbar-hide relative inline-flex max-w-full items-center overflow-x-auto overflow-y-hidden rounded-full border border-border bg-card/90 shadow-2xl backdrop-blur-xl",
        size.track(),
        class_names.track.clone(),
    );

    rsx! {
        div {
            "data-beui-scope": scope.attr(),
            class,
            onpointerenter: {
                let hover = hover.clone();
                move |event: Event<PointerData>| {
                    if hover.enter(&event) && expand_on_hover {
                        open.call(());
                    }
                }
            },
            onpointerleave: {
                let hover = hover.clone();
                move |event: Event<PointerData>| {
                    if !hover.leave(&event) {
                        return;
                    }
                    hovered.set(None);
                    if expand_on_hover {
                        close.call(());
                    }
                }
            },
            onfocus: move |_| {
                if expand_on_focus {
                    open.call(());
                }
            },
            onblur: move |_| {
                if expand_on_focus {
                    close.call(());
                }
            },
            div { class: track_class,
                for item in items.iter() {
                    ActionBarItemView {
                        key: "{item.id}",
                        item: item.clone(),
                        size,
                        expanded: is_expanded,
                        highlighted: highlight_id.as_deref() == Some(item.id.as_str()),
                        layout_id: layout_id.clone(),
                        class_names: class_names.clone(),
                        reduce: reduce_value,
                        expand_on_hover,
                        tap_expanded,
                        on_enter: EventHandler::new({
                            let mut hovered = hovered;
                            let id = item.id.clone();
                            move |_| hovered.set(Some(id.clone()))
                        }),
                        on_first_tap: EventHandler::new({
                            let mut hovered = hovered;
                            let id = item.id.clone();
                            move |_| {
                                open.call(());
                                hovered.set(Some(id.clone()));
                            }
                        }),
                        on_action: EventHandler::new(move |item: ExpandableActionBarItem| {
                            if let Some(handler) = item.on_click {
                                handler.call(());
                            }
                            if let Some(handler) = on_action {
                                handler.call(item);
                            }
                        }),
                    }
                }
            }
        }
    }
}

/// One action.
#[allow(clippy::too_many_arguments)]
#[component]
fn ActionBarItemView(
    item: ExpandableActionBarItem,
    size: ExpandableActionBarSize,
    expanded: bool,
    highlighted: bool,
    layout_id: String,
    class_names: ExpandableActionBarClassNames,
    reduce: bool,
    expand_on_hover: bool,
    tap_expanded: Signal<bool>,
    on_enter: EventHandler<()>,
    on_first_tap: EventHandler<()>,
    on_action: EventHandler<ExpandableActionBarItem>,
) -> Element {
    let gestures = use_context::<ActionBarGestures>();
    let label_rect: ElementRect = use_element_rect();
    let shortcut_rect: ElementRect = use_element_rect();
    use_effect(move || {
        label_rect.refresh();
        shortcut_rect.refresh();
    });
    let mut progress = use_motion(if expanded { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    use_effect(use_reactive((&expanded,), move |(expanded,)| {
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(LABEL_SPRING)
        };
        let _ = progress.animate_to(if expanded { 1.0 } else { 0.0 }, config);
    }));
    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let label_width = label_rect.get().map(|rect| rect.width).unwrap_or(0.0);
    let shortcut_width = shortcut_rect.get().map(|rect| rect.width).unwrap_or(0.0);
    let label_style = format!("width:{}px;opacity:{p}", label_width * p);
    let shortcut_style = format!("width:{}px;opacity:{p}", shortcut_width * p);
    let label_inner_style = if reduce {
        "transform:translateX(0px);filter:blur(0px)".to_string()
    } else {
        format!(
            "transform:translateX({}px);filter:blur({}px)",
            -4.0 * (1.0 - p),
            3.0 * (1.0 - p)
        )
    };

    let button_class = cn!(
        "relative isolate inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full font-medium text-muted-foreground outline-none transition-[color,background-color] duration-150 ease-out",
        "focus-visible:text-foreground disabled:pointer-events-none disabled:opacity-40",
        if highlighted { "text-foreground" } else { "" },
        size.item(),
        class_names.item.clone(),
        if item.active { class_names.active_item.clone() } else { None },
    );
    let item_for_click = item.clone();
    let label = item.label.clone();
    let shortcut = item.shortcut.clone();
    let badge = item.badge.clone();
    let icon = item.icon.clone();
    let disabled = item.disabled;

    rsx! {
        button {
            r#type: "button",
            disabled,
            title: label.clone(),
            class: button_class,
            onpointerenter: {
                let hover = gestures.hover.clone();
                move |event: Event<PointerData>| {
                    if hover.enter(&event) {
                        on_enter.call(());
                    }
                }
            },
            onpointerdown: {
                let tap = gestures.tap.clone();
                move |event: Event<PointerData>| {
                    tap.start(&event, expanded);
                }
            },
            onpointercancel: {
                let tap = gestures.tap.clone();
                move |_| tap.drop_record()
            },
            onkeydown: {
                let tap = gestures.tap.clone();
                move |_| tap.drop_record()
            },
            onclick: {
                let tap = gestures.tap.clone();
                let mut tap_expanded = tap_expanded;
                move |_| {
                    let gesture = tap.take();
                    let first_tap = gesture
                        .as_ref()
                        .map(|record| record.pointer_type != "mouse" && !record.state)
                        .unwrap_or(false)
                        && !tap_expanded();
                    if first_tap && expand_on_hover {
                        tap_expanded.set(true);
                        on_first_tap.call(());
                        return;
                    }
                    on_action.call(item_for_click.clone());
                }
            },
            if highlighted {
                HighlightPill { layout_id, reduce }
            }
            span {
                class: cn!(
                    "inline-flex shrink-0 items-center justify-center",
                    size.icon(),
                    class_names.icon.clone(),
                ),
                {icon}
            }
            span {
                aria_hidden: if expanded { "false" } else { "true" },
                class: cn!(
                    "inline-block overflow-hidden whitespace-nowrap",
                    class_names.label.clone(),
                ),
                style: label_style.clone(),
                span { class: "inline-block whitespace-nowrap", style: label_inner_style, "{label}" }
            }
            if let Some(shortcut) = shortcut.clone() {
                span {
                    aria_hidden: if expanded { "false" } else { "true" },
                    class: cn!(
                        "hidden overflow-hidden whitespace-nowrap text-[10px] text-muted-foreground sm:inline-block",
                        class_names.shortcut.clone(),
                    ),
                    style: shortcut_style.clone(),
                    "{shortcut}"
                }
            }
            if let Some(badge) = badge.clone() {
                span {
                    class: cn!(
                        "ml-0.5 inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-destructive px-1 text-[10px] leading-none text-primary-foreground",
                        if expanded { "" } else { "absolute right-0.5 top-0.5" },
                        class_names.badge.clone(),
                    ),
                    "{badge}"
                }
            }
            // Hidden natural-width measurements for the two animated spans.
            span {
                aria_hidden: "true",
                class: "pointer-events-none invisible absolute inline-block w-max whitespace-nowrap",
                "data-beui-rect": label_rect.attr(),
                onmounted: move |data| label_rect.measure(data.data()),
                "{label}"
            }
            if let Some(shortcut) = shortcut {
                span {
                    aria_hidden: "true",
                    class: "pointer-events-none invisible absolute inline-block w-max whitespace-nowrap text-[10px]",
                    "data-beui-rect": shortcut_rect.attr(),
                    onmounted: move |data| shortcut_rect.measure(data.data()),
                    "{shortcut}"
                }
            }
        }
    }
}

/// The shared highlight, handed between items by the flip registry.
#[component]
fn HighlightPill(layout_id: String, reduce: bool) -> Element {
    let transition = if reduce {
        beui_core::ease::tween_ease_out(0)
    } else {
        AnimationConfig::spring(ITEM_SPRING)
    };
    let flip = use_flip(
        layout_id.clone(),
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(transition),
    );
    rsx! {
        span {
            aria_hidden: "true",
            "data-beui-flip": flip.scope(),
            class: "absolute inset-0 -z-10 rounded-full bg-muted/80",
            style: flip.style_with(""),
        }
    }
}
