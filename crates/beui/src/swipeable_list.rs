//! `motion/swipeable-list.tsx` — rows that reveal action rails on a horizontal
//! drag.
//!
//! The drag runs through `beui_core::drag` (pointer capture + velocity
//! sampling), per the port's rule: motion's `dragElastic: 0.04` and
//! `dragMomentum: false` become `DragElastic::uniform(0.04)` and
//! `without_momentum()`, and the release uses the source's `ROW_SETTLE`
//! (`{560, 48, 0.82}`). The flick thresholds (`OPEN_VELOCITY`, `FLING_DISTANCE`,
//! the distance ratios) are copied verbatim.
//!
//! One deviation: the release spring is not seeded with the flick velocity —
//! `Drag::settle_to` starts the settle at rest — so a fast flick snaps by the
//! velocity *threshold* rather than coasting with the extra distance.

use crate::prelude::*;
use beui_core::drag::{
    use_drag, Axes, DragAxis, DragConstraints, DragElastic, DragEnd, DragOptions, DragRange,
};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{AnimationConfig, Spring};

/// `ROW_SETTLE`, verbatim (minus motion's rest thresholds).
const ROW_SETTLE: Spring = Spring {
    stiffness: 560.0,
    damping: 48.0,
    mass: 0.82,
};

const OPEN_DISTANCE_RATIO: f64 = 0.46;
const CLOSE_DISTANCE_RATIO: f64 = 0.72;
const OPEN_VELOCITY: f64 = 720.0;
const CLOSE_VELOCITY: f64 = 320.0;
const FLING_DISTANCE: f64 = 14.0;

/// Which rail a row opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwipeSide {
    Left,
    Right,
}

/// The open row, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwipeableListValue {
    pub id: String,
    pub side: SwipeSide,
}

/// A tone for an action's icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwipeActionTone {
    #[default]
    Neutral,
    Primary,
    Success,
    Warning,
    Danger,
}

impl SwipeActionTone {
    fn class(self) -> &'static str {
        match self {
            SwipeActionTone::Neutral => "text-muted-foreground group-hover:text-foreground",
            SwipeActionTone::Primary => "text-foreground",
            SwipeActionTone::Success => "text-emerald-600 dark:text-emerald-400",
            SwipeActionTone::Warning => "text-amber-600 dark:text-amber-400",
            SwipeActionTone::Danger => "text-destructive",
        }
    }
}

/// One rail action.
#[derive(Clone, PartialEq)]
pub struct SwipeAction {
    pub id: String,
    pub label: String,
    pub icon: Element,
    pub tone: SwipeActionTone,
    pub disabled: bool,
    pub on_click: Option<EventHandler<SwipeableListItem>>,
}

impl SwipeAction {
    /// An action with a label and icon.
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Element) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            tone: SwipeActionTone::Neutral,
            disabled: false,
            on_click: None,
        }
    }

    /// Set the tone.
    pub fn tone(mut self, tone: SwipeActionTone) -> Self {
        self.tone = tone;
        self
    }
}

/// One row.
#[derive(Clone, PartialEq)]
pub struct SwipeableListItem {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub meta: Option<String>,
    pub leading: Option<Element>,
    pub content: Option<Element>,
    pub left_actions: Vec<SwipeAction>,
    pub right_actions: Vec<SwipeAction>,
    pub disabled: bool,
}

impl SwipeableListItem {
    /// A row with a title.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: None,
            description: None,
            meta: None,
            leading: None,
            content: None,
            left_actions: Vec::new(),
            right_actions: Vec::new(),
            disabled: false,
        }
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set trailing meta text.
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// Set the leading node.
    pub fn leading(mut self, leading: Element) -> Self {
        self.leading = Some(leading);
        self
    }

    /// Set the left rail.
    pub fn left_actions(mut self, actions: Vec<SwipeAction>) -> Self {
        self.left_actions = actions;
        self
    }

    /// Set the right rail.
    pub fn right_actions(mut self, actions: Vec<SwipeAction>) -> Self {
        self.right_actions = actions;
        self
    }
}

/// Per-part class overrides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SwipeableListClassNames {
    pub root: Option<String>,
    pub item: Option<String>,
    pub rail: Option<String>,
    pub action: Option<String>,
    pub surface: Option<String>,
    pub leading: Option<String>,
    pub content: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub meta: Option<String>,
}

/// What an action reports.
#[derive(Clone, PartialEq)]
pub struct SwipeActionEvent {
    pub item: SwipeableListItem,
    pub action: SwipeAction,
    pub side: SwipeSide,
}

/// Props for [`SwipeableList`].
#[derive(Props, Clone, PartialEq)]
pub struct SwipeableListProps {
    pub items: Vec<SwipeableListItem>,
    /// `None` uncontrolled, `Some(None)` controlled closed, `Some(Some(v))` open.
    #[props(default)]
    pub value: Option<Option<SwipeableListValue>>,
    #[props(default)]
    pub default_value: Option<SwipeableListValue>,
    #[props(default)]
    pub on_value_change: Option<EventHandler<Option<SwipeableListValue>>>,
    #[props(default)]
    pub on_action: Option<EventHandler<SwipeActionEvent>>,
    #[props(default = 56.0)]
    pub action_width: f64,
    #[props(default = 34.0)]
    pub reveal_threshold: f64,
    #[props(default = true)]
    pub close_on_action: bool,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub class_names: SwipeableListClassNames,
}

/// A list of horizontally swipeable rows.
#[component]
pub fn SwipeableList(props: SwipeableListProps) -> Element {
    let SwipeableListProps {
        items,
        value,
        default_value,
        on_value_change,
        on_action,
        action_width,
        reveal_threshold,
        close_on_action,
        class,
        class_names,
    } = props;

    let mut internal = use_signal(|| default_value.clone());
    let controlled = value.is_some();
    let open_value = match &value {
        Some(inner) => inner.clone(),
        None => internal(),
    };
    let set_open_value = EventHandler::new(move |next: Option<SwipeableListValue>| {
        if !controlled {
            internal.set(next.clone());
        }
        if let Some(handler) = on_value_change {
            handler.call(next);
        }
    });
    let class = cn!(
        "flex w-full flex-col gap-2",
        class,
        class_names.root.clone()
    );

    rsx! {
        div { class,
            for item in items.iter() {
                SwipeableListRow {
                    key: "{item.id}",
                    item: item.clone(),
                    action_width,
                    reveal_threshold,
                    open_value: open_value.clone(),
                    set_open_value,
                    close_on_action,
                    on_action,
                    class_names: class_names.clone(),
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[component]
fn SwipeableListRow(
    item: SwipeableListItem,
    action_width: f64,
    reveal_threshold: f64,
    #[props(default)] open_value: Option<SwipeableListValue>,
    set_open_value: EventHandler<Option<SwipeableListValue>>,
    close_on_action: bool,
    #[props(default)] on_action: Option<EventHandler<SwipeActionEvent>>,
    #[props(default)] class_names: SwipeableListClassNames,
) -> Element {
    let reduce = use_reduced_motion();
    let left_width = item.left_actions.len() as f64 * action_width;
    let right_width = item.right_actions.len() as f64 * action_width;
    let open_side = open_value
        .as_ref()
        .filter(|value| value.id == item.id)
        .map(|value| value.side);
    let target_x = match open_side {
        Some(SwipeSide::Left) => left_width,
        Some(SwipeSide::Right) => -right_width,
        None => 0.0,
    };
    let mut commanded = use_signal(|| 0.0f64);
    let mut drag_end = use_signal(|| None::<DragEnd>);
    let options = DragOptions::default()
        .axis(DragAxis::X)
        .constraints(DragConstraints {
            x: DragRange::new(Some(-right_width), Some(left_width)),
            y: DragRange::pinned(0.0),
        })
        .elastic(DragElastic::uniform(0.04))
        .without_momentum();
    let drag = use_drag(
        options,
        Some(EventHandler::new(move |end: DragEnd| {
            drag_end.set(Some(end));
        })),
    );

    let mut settle = move |next_x: f64| {
        commanded.set(next_x);
        let config = if reduce() {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(ROW_SETTLE)
        };
        drag.settle_to(Axes { x: next_x, y: 0.0 }, config);
    };

    // A controlled change (or another row opening) settles this row.
    use_effect(use_reactive((&target_x,), move |(target_x,)| {
        if (*commanded.peek() - target_x).abs() > 0.5 {
            settle(target_x);
        }
    }));
    // Lets an action close the row it belongs to.
    use_context_provider(|| SwipeRowContext {
        close: EventHandler::new(move |_| set_open_value.call(None)),
    });

    // Release logic, mirrored from the source.
    let item_for_snap = item.clone();
    use_effect(move || {
        let Some(end) = *drag_end.read() else {
            return;
        };
        drag_end.set(None);
        let velocity = end.velocity.x;
        let latest = end.value.x;
        let left_open_threshold = reveal_threshold.max(left_width * OPEN_DISTANCE_RATIO);
        let right_open_threshold = reveal_threshold.max(right_width * OPEN_DISTANCE_RATIO);
        let mut snap_to = |side: Option<SwipeSide>| {
            let next = side.map(|side| SwipeableListValue {
                id: item_for_snap.id.clone(),
                side,
            });
            set_open_value.call(next);
            let target = match side {
                Some(SwipeSide::Left) => left_width,
                Some(SwipeSide::Right) => -right_width,
                None => 0.0,
            };
            settle(target);
        };

        if open_side == Some(SwipeSide::Left) {
            if latest < left_width * CLOSE_DISTANCE_RATIO || velocity < -CLOSE_VELOCITY {
                snap_to(None);
            } else {
                snap_to(Some(SwipeSide::Left));
            }
            return;
        }
        if open_side == Some(SwipeSide::Right) {
            if latest.abs() < right_width * CLOSE_DISTANCE_RATIO || velocity > CLOSE_VELOCITY {
                snap_to(None);
            } else {
                snap_to(Some(SwipeSide::Right));
            }
            return;
        }
        if left_width > 0.0
            && latest.abs() > 0.0
            && (latest > left_open_threshold
                || (velocity > OPEN_VELOCITY && latest > FLING_DISTANCE))
        {
            snap_to(Some(SwipeSide::Left));
            return;
        }
        if right_width > 0.0
            && latest.abs() > 0.0
            && (latest < -right_open_threshold
                || (velocity < -OPEN_VELOCITY && latest < -FLING_DISTANCE))
        {
            snap_to(Some(SwipeSide::Right));
            return;
        }
        snap_to(None);
    });

    let x = drag.offset().x;
    let rail_class = cn!(
        "absolute inset-0 z-0 flex overflow-hidden rounded-2xl",
        class_names.rail.clone(),
    );
    let surface_class = cn!(
        "relative z-10 min-h-[72px] cursor-grab touch-pan-y rounded-2xl border border-border bg-card px-4 py-3 shadow-sm active:cursor-grabbing",
        TOUCH_GESTURE_CONTENT_CLASS,
        class_names.surface.clone(),
    );
    let item_class = cn!(
        "relative isolate overflow-hidden rounded-2xl bg-muted",
        if item.disabled { "opacity-60" } else { "" },
        class_names.item.clone(),
    );
    let left_actions = item.left_actions.clone();
    let right_actions = item.right_actions.clone();
    let disabled = item.disabled;
    let leading = item.leading.clone();
    let content = item.content.clone();
    let title = item.title.clone();
    let description = item.description.clone();
    let meta = item.meta.clone();
    let item_for_actions = item.clone();

    rsx! {
        div { class: item_class,
            div {
                aria_hidden: if open_side.is_some() { "false" } else { "true" },
                inert: if open_side.is_some() { None } else { Some("true") },
                class: rail_class,
                div { class: "flex h-full overflow-hidden rounded-l-2xl",
                    for action in left_actions.iter() {
                        SwipeActionButton {
                            key: "{action.id}",
                            action: action.clone(),
                            action_width,
                            side: SwipeSide::Left,
                            focusable: open_side == Some(SwipeSide::Left),
                            item: item_for_actions.clone(),
                            close_on_action,
                            on_action,
                            class: class_names.action.clone(),
                        }
                    }
                }
                div { class: "ml-auto flex h-full overflow-hidden rounded-r-2xl",
                    for action in right_actions.iter() {
                        SwipeActionButton {
                            key: "{action.id}",
                            action: action.clone(),
                            action_width,
                            side: SwipeSide::Right,
                            focusable: open_side == Some(SwipeSide::Right),
                            item: item_for_actions.clone(),
                            close_on_action,
                            on_action,
                            class: class_names.action.clone(),
                        }
                    }
                }
            }
            div {
                class: surface_class,
                style: "transform:translateX({x}px)",
                onpointerdown: move |event: Event<PointerData>| {
                    if !disabled {
                        drag.on_pointer_down(&event);
                    }
                },
                onpointermove: move |event: Event<PointerData>| drag.on_pointer_move(&event),
                onpointerup: move |event: Event<PointerData>| drag.on_pointer_up(&event),
                onpointercancel: move |event: Event<PointerData>| drag.on_pointer_cancel(&event),
                if let Some(content) = content {
                    {content}
                } else {
                    div { class: "flex min-w-0 items-center gap-3",
                        if let Some(leading) = leading {
                            div { class: cn!("shrink-0", class_names.leading.clone()), {leading} }
                        }
                        div { class: cn!("min-w-0 flex-1", class_names.content.clone()),
                            if let Some(title) = title {
                                div { class: cn!("truncate text-sm font-medium text-foreground", class_names.title.clone()), "{title}" }
                            }
                            if let Some(description) = description {
                                div { class: cn!("mt-0.5 truncate text-xs text-muted-foreground", class_names.description.clone()), "{description}" }
                            }
                        }
                        if let Some(meta) = meta {
                            div { class: cn!("shrink-0 text-xs font-medium text-muted-foreground", class_names.meta.clone()), "{meta}" }
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[component]
fn SwipeActionButton(
    action: SwipeAction,
    action_width: f64,
    side: SwipeSide,
    focusable: bool,
    item: SwipeableListItem,
    close_on_action: bool,
    #[props(default)] on_action: Option<EventHandler<SwipeActionEvent>>,
    #[props(default)] class: Option<String>,
) -> Element {
    let context = use_context::<SwipeRowContext>();
    let class = cn!(
        "group flex h-full shrink-0 items-center justify-center outline-none",
        "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:pointer-events-none disabled:opacity-50",
        class,
    );
    let icon_class = cn!(
        "grid h-9 w-9 place-items-center rounded-full transition-[background-color,color,transform] duration-150 group-hover:bg-background group-active:scale-95",
        action.tone.class(),
    );
    let label = action.label.clone();
    let disabled = action.disabled;
    let action_for_click = action.clone();

    rsx! {
        button {
            r#type: "button",
            disabled,
            tabindex: if focusable { "0" } else { "-1" },
            aria_label: label.clone(),
            class,
            style: "width:{action_width}px",
            onclick: move |_| {
                if let Some(handler) = action_for_click.on_click {
                    handler.call(item.clone());
                }
                if let Some(handler) = on_action {
                    handler.call(SwipeActionEvent {
                        item: item.clone(),
                        action: action_for_click.clone(),
                        side,
                    });
                }
                if close_on_action {
                    context.close.call(());
                }
            },
            span { class: icon_class, {action.icon.clone()} }
            span { class: "sr-only", "{label}" }
        }
    }
}

/// Lets an action close the row it belongs to.
#[derive(Clone, Copy)]
struct SwipeRowContext {
    close: EventHandler<()>,
}
