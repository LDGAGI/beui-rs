//! `motion/notification-stack.tsx` — a collapsed card stack that expands on
//! hover, focus or tap.
//!
//! The gesture split is the source's: a pointer *leaving* collapses, and a
//! finger never leaves, so the tap that expands sets `tap_expanded` and the
//! outside tap (consumed by `use_dismiss`) is what collapses it. A tap reports
//! as hover on its way past, so the hover path filters with `use_hover_gesture`.
//!
//! Motion's `layout` projection on the cards and the background is not ported
//! (a known gap); the cards' `y` and `clip-path` are animated per card instead,
//! which is the visible half of the same transition.

use std::cell::Cell;
use std::rc::Rc;

use crate::action_swap::{ActionSwapAnimation, ActionSwapText};
use crate::icons::{ArrowUpRight, BellOff};
use crate::prelude::*;
use beui_core::hooks::{use_dismiss, use_hover_gesture, use_tap_gesture, DismissOptions};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationManager, MotionHandle};

const STACK_PEEK: f64 = 8.0;
const STACK_INSET: f64 = 12.0;

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// One notification.
#[derive(Clone, PartialEq)]
pub struct NotificationStackItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    /// Trailing node (the source's `ReactNode`), e.g. a retry badge.
    pub trailing: Option<Element>,
}

impl NotificationStackItem {
    /// An item with a title.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            trailing: None,
        }
    }

    /// Attach a description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Attach a trailing node.
    pub fn trailing(mut self, trailing: Element) -> Self {
        self.trailing = Some(trailing);
        self
    }
}

/// Per-part class overrides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotificationStackClassNames {
    pub stack: Option<String>,
    pub card: Option<String>,
    pub content: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub trailing: Option<String>,
    pub footer: Option<String>,
    pub count: Option<String>,
}

/// Props for [`NotificationStack`].
#[derive(Props, Clone, PartialEq)]
pub struct NotificationStackProps {
    pub items: Vec<NotificationStackItem>,
    /// Controlled expanded state.
    #[props(default)]
    pub expanded: Option<bool>,
    #[props(default)]
    pub default_expanded: bool,
    #[props(default)]
    pub on_expanded_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub on_view_all: Option<EventHandler<()>>,
    #[props(default = 3)]
    pub max_visible: usize,
    #[props(default = "Notifications".to_string())]
    pub collapsed_label: String,
    #[props(default = "View all".to_string())]
    pub expanded_label: String,
    #[props(default = "All caught up".to_string())]
    pub empty_label: String,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub class_names: NotificationStackClassNames,
}

/// A stack of notifications that expands.
#[component]
pub fn NotificationStack(props: NotificationStackProps) -> Element {
    let NotificationStackProps {
        items,
        expanded,
        default_expanded,
        on_expanded_change,
        on_view_all,
        max_visible,
        collapsed_label,
        expanded_label,
        empty_label,
        class,
        class_names,
    } = props;

    let reduce = use_reduced_motion();
    let mut internal_expanded = use_signal(|| default_expanded);
    let is_controlled = expanded.is_some();
    let mut set_expanded = move |next: bool| {
        if !is_controlled {
            internal_expanded.set(next);
        }
        if let Some(handler) = on_expanded_change {
            handler.call(next);
        }
    };
    let is_expanded = match expanded {
        Some(value) => value,
        None => internal_expanded(),
    };

    let has_focus = use_hook(|| Rc::new(Cell::new(false)));
    let hover = use_hover_gesture();
    let tap = use_tap_gesture::<bool>();
    let mut tap_expanded = use_signal(|| false);

    let mut collapse = move || {
        tap_expanded.set(false);
        set_expanded(false);
    };
    let mut dismiss_open = use_signal(|| false);
    use_effect(use_reactive((&is_expanded,), move |(is_expanded,)| {
        dismiss_open.set(tap_expanded() && is_expanded);
    }));
    // A pointer leaving is what collapses; a finger never leaves, so the tap
    // that expanded sets `tap_expanded` and the next outside tap (consumed) is
    // what closes it.
    let scope = use_dismiss(
        dismiss_open,
        EventHandler::new(move |_| collapse()),
        DismissOptions::default().consume(),
    );

    let visible_count = if items.is_empty() {
        0
    } else {
        max_visible.max(1).min(items.len())
    };

    if visible_count == 0 {
        let class = cn!(
            "flex w-full max-w-[22rem] items-center justify-center gap-2 rounded-3xl bg-muted/70 px-5 py-8 text-sm font-medium text-muted-foreground",
            class,
        );
        return rsx! {
            div { class,
                BellOff { class: Some("h-4 w-4".to_string()) }
                "{empty_label}"
            }
        };
    }

    let visible: Vec<NotificationStackItem> = items.iter().take(visible_count).cloned().collect();
    let primary = visible[0].clone();
    let count = visible.len();
    let reduce_value = reduce();
    let is_exp = is_expanded;

    let mut toggle_expanded = move |next: bool| {
        tap_expanded.set(false);
        set_expanded(next);
    };
    let mut focus_true = {
        let has_focus = has_focus.clone();
        move || {
            has_focus.set(true);
            set_expanded(true);
        }
    };
    let mut focus_false = {
        let has_focus = has_focus.clone();
        move || {
            has_focus.set(false);
            toggle_expanded(false);
        }
    };
    let aria_label = if is_exp {
        format!("{} notifications. {expanded_label}.", items.len())
    } else {
        format!("{} notifications. Expand notifications.", items.len())
    };
    let class = cn!(
        "relative z-10 block w-full max-w-[22rem] cursor-pointer rounded-3xl text-left text-foreground outline-none",
        "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        class,
    );

    let mut on_click = {
        let mut tap_expanded = tap_expanded;
        let tap = tap.clone();
        move || {
            let gesture = tap.take();
            let was_expanded = gesture
                .as_ref()
                .map(|record| record.state)
                .unwrap_or(is_exp);
            if !was_expanded {
                toggle_expanded(true);
                if gesture
                    .as_ref()
                    .map(|record| record.pointer_type != "mouse")
                    .unwrap_or(false)
                {
                    tap_expanded.set(true);
                }
                return;
            }
            if let Some(handler) = &on_view_all {
                handler.call(());
                return;
            }
            toggle_expanded(false);
        }
    };

    rsx! {
        button {
            r#type: "button",
            "data-beui-scope": scope.attr(),
            "aria-expanded": if is_exp { "true" } else { "false" },
            aria_label,
            class,
            onpointerenter: {
                let hover = hover.clone();
                move |event: Event<PointerData>| {
                    if hover.enter(&event) {
                        set_expanded(true);
                    }
                }
            },
            onpointerleave: {
                let hover = hover.clone();
                let has_focus = has_focus.clone();
                move |event: Event<PointerData>| {
                    if hover.leave(&event) && !has_focus.get() {
                        toggle_expanded(false);
                    }
                }
            },
            onpointerdown: {
                let tap = tap.clone();
                move |event: Event<PointerData>| {
                    tap.start(&event, is_exp);
                }
            },
            onpointercancel: {
                let tap = tap.clone();
                move |_| tap.drop_record()
            },
            onfocus: move |_| focus_true(),
            onblur: move |_| focus_false(),
            onkeydown: {
                let tap = tap.clone();
                move |event: Event<KeyboardData>| {
                    tap.drop_record();
                    if event.key() != Key::Escape {
                        return;
                    }
                    event.prevent_default();
                    toggle_expanded(false);
                }
            },
            onclick: move |_| on_click(),

            // The invisible first card gives the button its compact intrinsic footprint.
            span { aria_hidden: "true", class: "invisible block p-3",
                span { class: "block",
                    span { class: cn!("block rounded-2xl border border-transparent px-4", class_names.card.clone()),
                        NotificationCardContent {
                            item: primary.clone(),
                            content: class_names.content.clone(),
                            title: class_names.title.clone(),
                            description: class_names.description.clone(),
                            trailing: class_names.trailing.clone(),
                        }
                    }
                }
                span { class: "mt-2 block h-9" }
            }

            span { class: "absolute inset-x-0 bottom-0 block p-3",
                span { aria_hidden: "true", class: "absolute inset-0 rounded-3xl bg-muted" }
                span { class: cn!("relative z-10 grid gap-1", if !is_exp { "pb-2" } else { "" }, class_names.stack.clone()),
                    for (index, item) in visible.into_iter().enumerate() {
                        StackCard {
                            key: "{item.id}",
                            item,
                            index,
                            count,
                            expanded: is_exp,
                            reduce: reduce_value,
                            class: class_names.card.clone(),
                            content: class_names.content.clone(),
                            title: class_names.title.clone(),
                            description: class_names.description.clone(),
                            trailing: class_names.trailing.clone(),
                        }
                    }
                }
                span { class: cn!("relative z-10 mt-2 flex min-h-9 items-center gap-2 px-1", class_names.footer.clone()),
                    span { class: cn!(
                        "grid size-7 shrink-0 place-items-center rounded-full bg-orange-600 text-xs font-medium text-white shadow-[inset_0_1px_2px_rgb(0_0_0/0.2),inset_0_-1px_0_rgb(255_255_255/0.16)] dark:bg-orange-500",
                        class_names.count.clone(),
                    ),
                        "{items.len()}"
                    }
                    span { class: "flex items-center text-sm font-medium",
                        ActionSwapText {
                            value: if is_exp { "expanded".to_string() } else { "collapsed".to_string() },
                            label: if is_exp { expanded_label.clone() } else { collapsed_label.clone() },
                            animation: ActionSwapAnimation::Roll,
                        }
                        if is_exp {
                            ArrowUpRight { class: Some("size-4".to_string()) }
                        }
                    }
                }
            }
        }
    }
}

/// The card content column.
#[component]
fn NotificationCardContent(
    item: NotificationStackItem,
    #[props(default)] content: Option<String>,
    #[props(default)] title: Option<String>,
    #[props(default)] description: Option<String>,
    #[props(default)] trailing: Option<String>,
) -> Element {
    rsx! {
        span { class: cn!("flex min-w-0 flex-col gap-1.5 py-4", content),
            span { class: "flex min-w-0 items-start justify-between gap-3",
                span { class: cn!("min-w-0 text-sm font-medium leading-snug", title), "{item.title}" }
                if let Some(trailing_node) = item.trailing.clone() {
                    span { class: cn!("shrink-0 text-xs", trailing), {trailing_node} }
                }
            }
            if let Some(description_text) = item.description.clone() {
                span { class: cn!("text-xs leading-relaxed text-muted-foreground", description),
                    "{description_text}"
                }
            }
        }
    }
}

/// One card in the stack; its `y` and `clip-path` follow the expanded state.
#[component]
fn StackCard(
    item: NotificationStackItem,
    index: usize,
    count: usize,
    expanded: bool,
    reduce: bool,
    class: Option<String>,
    content: Option<String>,
    title: Option<String>,
    description: Option<String>,
    trailing: Option<String>,
) -> Element {
    let mut progress = use_motion(if expanded { 1.0f32 } else { 0.0f32 }).expect("0/1 are finite");
    use_effect(use_reactive((&expanded,), move |(expanded,)| {
        let target = if expanded { 1.0 } else { 0.0 };
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            beui_core::ease::tween_ease_out(320)
        };
        let _ = progress.animate_to(target, config);
    }));

    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let y = (1.0 - p) * index as f64 * STACK_PEEK;
    let inset = (1.0 - p) * index as f64 * STACK_INSET;
    let style = format!(
        "z-index:{};grid-column:1;grid-row:{};transform:translateY({y}px);clip-path:inset(0px {inset}px round 16px)",
        count - index,
        if expanded { index + 1 } else { 1 },
    );
    let card_class = cn!(
        "block rounded-2xl border border-border/60 bg-background px-4",
        class,
    );
    let hidden = index != 0 && !expanded;

    rsx! {
        span { "data-slot": "notification-card", class: card_class, style,
            span { class: cn!("block", if hidden { "invisible" } else { "" }),
                NotificationCardContent { item, content, title, description, trailing }
            }
        }
    }
}
