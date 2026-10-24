//! `motion/overflow-actions.tsx` — a pill bar whose extra actions slide out of a
//! toggle.
//!
//! The source drives the bar's box with `layout` and compensates the exiting
//! group's `left` because `popLayout` takes it out of flow. Here the overflow
//! group's own `width` springs from 0 to its measured natural width, so the bar
//! grows with it and there is nothing to compensate; the actions fade in from a
//! 4px blur and the toggle's icon cross-fades. `SHELL_TRANSITION`
//! (`{220, 17, 0.85}`) is copied verbatim.

use crate::icons::{MoreHorizontal, X};
use crate::prelude::*;
use crate::rect::{use_element_rect, ElementRect};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

/// `SHELL_TRANSITION`, verbatim.
const SHELL_SPRING: dioxus_motion::prelude::Spring = dioxus_motion::prelude::Spring {
    stiffness: 220.0,
    damping: 17.0,
    mass: 0.85,
};

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// Bar size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverflowActionsSize {
    Sm,
    #[default]
    Md,
}

impl OverflowActionsSize {
    fn track(self) -> &'static str {
        match self {
            OverflowActionsSize::Sm => "gap-1 p-1 text-xs",
            OverflowActionsSize::Md => "gap-1.5 p-1.5 text-sm",
        }
    }

    fn group_gap(self) -> &'static str {
        match self {
            OverflowActionsSize::Sm => "gap-1",
            OverflowActionsSize::Md => "gap-1.5",
        }
    }

    fn action(self) -> &'static str {
        match self {
            OverflowActionsSize::Sm => "h-8 min-w-8 gap-1.5 px-3",
            OverflowActionsSize::Md => "h-9 min-w-9 gap-2 px-3.5",
        }
    }

    fn toggle(self) -> &'static str {
        match self {
            OverflowActionsSize::Sm => "h-8 w-8",
            OverflowActionsSize::Md => "h-9 w-9",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            OverflowActionsSize::Sm => "h-3.5 w-3.5",
            OverflowActionsSize::Md => "h-4 w-4",
        }
    }
}

/// One action.
#[derive(Clone, PartialEq)]
pub struct OverflowActionItem {
    pub id: String,
    pub label: String,
    pub icon: Option<Element>,
    pub on_click: Option<EventHandler<()>>,
    pub disabled: bool,
    pub aria_label: Option<String>,
}

impl OverflowActionItem {
    /// An action with a label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            on_click: None,
            disabled: false,
            aria_label: None,
        }
    }

    /// Attach an icon.
    pub fn icon(mut self, icon: Element) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// Per-part class overrides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OverflowActionsClassNames {
    pub root: Option<String>,
    pub track: Option<String>,
    pub action: Option<String>,
    pub primary_action: Option<String>,
    pub overflow_action: Option<String>,
    pub toggle: Option<String>,
    pub icon: Option<String>,
    pub label: Option<String>,
}

/// Props for [`OverflowActions`].
#[derive(Props, Clone, PartialEq)]
pub struct OverflowActionsProps {
    pub primary_actions: Vec<OverflowActionItem>,
    pub overflow_actions: Vec<OverflowActionItem>,
    #[props(default)]
    pub expanded: Option<bool>,
    #[props(default)]
    pub default_expanded: bool,
    #[props(default)]
    pub on_expanded_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub on_action: Option<EventHandler<OverflowActionItem>>,
    /// Collapse after an action fires. Defaults to `false`.
    #[props(default)]
    pub collapse_on_action: bool,
    #[props(default)]
    pub size: OverflowActionsSize,
    #[props(default = "Show extra actions".to_string())]
    pub open_label: String,
    #[props(default = "Hide extra actions".to_string())]
    pub close_label: String,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub class_names: OverflowActionsClassNames,
}

/// A pill bar with a collapsible group of extra actions.
#[component]
pub fn OverflowActions(props: OverflowActionsProps) -> Element {
    let OverflowActionsProps {
        primary_actions,
        overflow_actions,
        expanded,
        default_expanded,
        on_expanded_change,
        on_action,
        collapse_on_action,
        size,
        open_label,
        close_label,
        class,
        class_names,
    } = props;

    let reduce = use_reduced_motion();
    let internal = use_signal(|| default_expanded);
    let controlled = expanded.is_some();
    let is_expanded = match expanded {
        Some(expanded) => expanded,
        None => internal(),
    };
    let set_expanded = move |next: bool| {
        if !controlled {
            let mut signal = internal;
            signal.set(next);
        }
        if let Some(handler) = on_expanded_change {
            handler.call(next);
        }
    };
    let handle_action = EventHandler::new(move |item: OverflowActionItem| {
        if let Some(handler) = item.on_click {
            handler.call(());
        }
        if let Some(handler) = on_action {
            handler.call(item);
        }
        if collapse_on_action {
            set_expanded(false);
        }
    });

    let group_rect: ElementRect = use_element_rect();
    let mut progress = use_motion(if is_expanded { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    let reduce_value = reduce();
    use_effect(use_reactive((&is_expanded,), move |(is_expanded,)| {
        let config = if reduce_value {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(SHELL_SPRING)
        };
        let _ = progress.animate_to(if is_expanded { 1.0 } else { 0.0 }, config);
    }));

    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let group_width = group_rect.get().map(|rect| rect.width).unwrap_or(0.0);
    let group_style = format!("width:{}px", p * group_width);
    let action_style = format!("opacity:{p};filter:blur({}px)", 4.0 * (1.0 - p));
    let icon_duration = if reduce_value { 0 } else { 180 };
    let open_icon_style = format!(
        "opacity:{};filter:blur({}px);transition:opacity {icon_duration}ms {EASE_OUT_CSS}, filter {icon_duration}ms {EASE_OUT_CSS}",
        if is_expanded { 0 } else { 1 },
        if is_expanded { 3 } else { 0 },
    );
    let close_icon_style = format!(
        "opacity:{};filter:blur({}px);transition:opacity {icon_duration}ms {EASE_OUT_CSS}, filter {icon_duration}ms {EASE_OUT_CSS}",
        if is_expanded { 1 } else { 0 },
        if is_expanded { 0 } else { 3 },
    );

    let root_class = cn!("inline-flex", class_names.root.clone(), class);
    let track_class = cn!(
        "relative inline-flex items-center overflow-hidden rounded-full border border-border bg-card",
        size.track(),
        class_names.track.clone(),
    );
    let group_class = cn!("inline-flex items-center", size.group_gap());
    let toggle_class = cn!(
        "relative inline-grid shrink-0 place-items-center rounded-full bg-primary text-primary-foreground outline-none disabled:pointer-events-none disabled:opacity-50",
        "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "transition-transform duration-150 active:scale-[0.96] hover:scale-[1.03]",
        size.toggle(),
        class_names.toggle.clone(),
    );
    let toggle_label = if is_expanded { close_label } else { open_label };
    let icon_class = size.icon().to_string();

    rsx! {
        div { class: root_class,
            div { class: track_class,
                div { class: group_class,
                    for item in primary_actions.iter() {
                        ActionButton {
                            key: "{item.id}",
                            item: item.clone(),
                            size,
                            class: cn!(class_names.action.clone(), class_names.primary_action.clone()),
                            icon_class: class_names.icon.clone(),
                            label_class: class_names.label.clone(),
                            style: None,
                            on_action: Some(handle_action),
                        }
                    }
                }
                div {
                    aria_hidden: if is_expanded { "false" } else { "true" },
                    class: cn!("relative inline-flex items-center overflow-hidden", size.group_gap()),
                    style: group_style,
                    for item in overflow_actions.iter() {
                        ActionButton {
                            key: "{item.id}",
                            item: item.clone(),
                            size,
                            class: cn!(class_names.action.clone(), class_names.overflow_action.clone()),
                            icon_class: class_names.icon.clone(),
                            label_class: class_names.label.clone(),
                            style: Some(action_style.clone()),
                            on_action: Some(handle_action),
                        }
                    }
                }
                // Hidden natural-width measurement for the overflow group.
                span {
                    aria_hidden: "true",
                    class: cn!(
                        "pointer-events-none invisible absolute inline-flex w-max items-center",
                        size.group_gap(),
                    ),
                    "data-beui-rect": group_rect.attr(),
                    onmounted: move |data| group_rect.measure(data.data()),
                    for item in overflow_actions.iter() {
                        span { key: "{item.id}", class: "inline-flex shrink-0",
                            span { class: cn!(
                                "inline-flex shrink-0 items-center justify-center rounded-full bg-background font-medium text-foreground",
                                size.action(),
                            ),
                                if let Some(icon) = item.icon.clone() {
                                    span { class: cn!("inline-flex shrink-0 items-center justify-center", icon_class.clone(), class_names.icon.clone()), {icon} }
                                }
                                span { class: cn!("whitespace-nowrap", class_names.label.clone()), "{item.label}" }
                            }
                        }
                    }
                }
                button {
                    r#type: "button",
                    aria_expanded: if is_expanded { "true" } else { "false" },
                    aria_label: toggle_label.clone(),
                    title: toggle_label,
                    class: toggle_class,
                    onclick: move |_| set_expanded(!is_expanded),
                    span { class: "col-start-1 row-start-1 inline-grid place-items-center", style: open_icon_style,
                        MoreHorizontal { class: Some(icon_class.clone()) }
                    }
                    span { class: "col-start-1 row-start-1 inline-grid place-items-center", style: close_icon_style,
                        X { class: Some(icon_class.clone()) }
                    }
                }
            }
        }
    }
}

/// One action button.
#[component]
fn ActionButton(
    item: OverflowActionItem,
    size: OverflowActionsSize,
    #[props(default)] class: Option<String>,
    #[props(default)] icon_class: Option<String>,
    #[props(default)] label_class: Option<String>,
    #[props(default)] style: Option<String>,
    #[props(default)] on_action: Option<EventHandler<OverflowActionItem>>,
) -> Element {
    let class = cn!(
        "inline-flex shrink-0 items-center justify-center rounded-full bg-background font-medium text-foreground outline-none",
        "disabled:pointer-events-none disabled:opacity-45",
        "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "transition-transform duration-150 active:scale-[0.97] hover:scale-[1.008]",
        size.action(),
        class,
    );
    let icon_wrapper_class = cn!(
        "inline-flex shrink-0 items-center justify-center",
        size.icon(),
        icon_class,
    );
    let label_class = cn!("whitespace-nowrap", label_class);
    let label = item.label.clone();
    let disabled = item.disabled;
    let item_for_click = item.clone();

    rsx! {
        span { class: "inline-flex shrink-0", style,
            button {
                r#type: "button",
                disabled,
                aria_label: item.aria_label.clone(),
                title: label,
                class,
                onclick: move |_| {
                    if let Some(handler) = on_action {
                        handler.call(item_for_click.clone());
                    }
                },
                if let Some(icon) = item.icon.clone() {
                    span { class: icon_wrapper_class, {icon} }
                }
                span { class: label_class, "{item.label}" }
            }
        }
    }
}
