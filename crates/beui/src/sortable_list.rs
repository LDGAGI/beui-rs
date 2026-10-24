//! `motion/sortable-list.tsx` — a reorderable list with a drag handle,
//! keyboard moves, undo and announcements. `SortableStack` is the same
//! component under the name the registry re-exports.
//!
//! The source builds on motion's `Reorder`; here the rows are ordinary flow
//! children (stable keys, so a reorder moves the DOM nodes) animated by
//! [`beui_core::flip`], and the drag runs from the handle with pointer capture,
//! the way `dragListener={false}` scopes it in the source.
//!
//! Live reordering is preserved: the row boxes are measured once when the drag
//! starts, the pointer is compared against those original positions to pick the
//! target index, and the list is reordered in place every time the index
//! changes. The dragged row keeps its own transform — `dy` minus the layout
//! shift it has just taken (`target_top - original_top`) — so it stays under the
//! finger while the other rows FLIP around it.
//!
//! One structural difference: the source exposes `SortableListGroup` / `Item` /
//! `Handle` / `ItemContent` parts, which cannot round-trip a Dioxus `Element`,
//! so the default composition is built in and rows arrive as
//! `SortableItem { id, label, content }`.

use crate::icons::{GripVertical, Undo};
use crate::prelude::*;
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::platform::{capture_pointer, js_string, json, next_scope_id, release_pointer};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

/// The drag's bookkeeping.
#[derive(Clone, PartialEq)]
struct DragState {
    id: String,
    start_y: f64,
    original_index: usize,
    target_index: usize,
    /// Rows in the order the drag started from.
    start_order: Vec<String>,
    /// Original layout positions: `(top, height)` per row.
    boxes: Vec<(f64, f64)>,
}

impl DragState {
    /// How far the row's layout has moved from where the finger first grabbed it.
    fn layout_shift(&self) -> f64 {
        if self.boxes.is_empty() {
            return 0.0;
        }
        let last = self.boxes.len() - 1;
        let original = self.original_index.min(last);
        let target = self.target_index.min(last);
        self.boxes[target].0 - self.boxes[original].0
    }
}

/// One row.
#[derive(Clone, PartialEq)]
pub struct SortableItem {
    pub id: String,
    /// Announced while moving and used for the handle's label.
    pub label: String,
    pub content: Element,
}

impl SortableItem {
    /// A row with a label and its content.
    pub fn new(id: impl Into<String>, label: impl Into<String>, content: Element) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            content,
        }
    }
}

/// Props for [`SortableList`].
#[derive(Props, Clone, PartialEq)]
pub struct SortableListProps {
    pub items: Vec<SortableItem>,
    /// The new id order, whenever a move, a drag or an undo changes it.
    pub on_reorder: EventHandler<Vec<String>>,
    #[props(default = "Sortable items".to_string())]
    pub label: String,
    #[props(default)]
    pub disabled: bool,
    /// Show the automatic undo button. Defaults to `true`.
    #[props(default = true)]
    pub show_undo: bool,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub item_class: Option<String>,
}

/// A reorderable list.
#[component]
pub fn SortableList(props: SortableListProps) -> Element {
    let SortableListProps {
        items,
        on_reorder,
        label,
        disabled,
        show_undo,
        class,
        item_class,
    } = props;

    let scope = use_hook(next_scope_id);
    let ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
    let previous = use_signal(|| None::<Vec<String>>);
    let drag_state = use_signal(|| None::<DragState>);
    let announcement = use_signal(String::new);
    let mut pending_focus = use_signal(|| None::<String>);

    let can_undo = previous()
        .map(|previous| {
            previous.len() == ids.len()
                && previous.iter().all(|id| ids.contains(id))
                && previous.iter().zip(ids.iter()).any(|(a, b)| a != b)
        })
        .unwrap_or(false);

    // Undo sends focus back to the first row that moved.
    let ids_for_focus = ids.clone();
    use_effect(move || {
        let Some(id) = pending_focus() else {
            return;
        };
        if !ids_for_focus.contains(&id) {
            return;
        }
        pending_focus.set(None);
        let selector = format!("[data-sortable-id=\"{id}\"]");
        spawn(async move {
            let script = format!(
                "const el = document.querySelector({});\n\
                 if (el) el.focus({{ preventScroll: true }});\n\
                 return true;",
                js_string(&selector)
            );
            let _ = json::<bool>(&script).await;
        });
    });

    let reorder_to = {
        let mut previous = previous;
        let mut announcement = announcement;
        let items = items.clone();
        move |next: Vec<String>, moved_id: Option<&str>, quiet: bool| {
            if !disabled {
                let current: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
                previous.set(Some(current));
            }
            if !quiet {
                if let Some(moved_id) = moved_id {
                    let position = next
                        .iter()
                        .position(|id| id == moved_id)
                        .map(|index| index + 1)
                        .unwrap_or(1);
                    let moved_label = items
                        .iter()
                        .find(|item| item.id == moved_id)
                        .map(|item| item.label.clone())
                        .unwrap_or_default();
                    announcement.set(format!(
                        "{moved_label} moved to position {position} of {}.",
                        next.len()
                    ));
                }
            }
            on_reorder.call(next);
        }
    };

    let move_item = {
        let items = items.clone();
        let mut reorder_to = reorder_to.clone();
        move |id: String, direction: i32| {
            if disabled {
                return;
            }
            let current: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
            let Some(from) = current.iter().position(|candidate| *candidate == id) else {
                return;
            };
            let last = current.len().saturating_sub(1);
            let to = match direction {
                -1 => from.saturating_sub(1),
                1 => (from + 1).min(last),
                -2 => 0,
                2 => last,
                _ => return,
            };
            if from == to {
                return;
            }
            let mut next = current.clone();
            let moved = next.remove(from);
            next.insert(to, moved);
            reorder_to(next, Some(&id), false);
        }
    };

    let undo = {
        let ids = ids.clone();
        let mut previous = previous;
        let mut announcement = announcement;
        let mut pending_focus = pending_focus;
        let mut reorder_to = reorder_to.clone();
        move || {
            if disabled || !can_undo {
                return;
            }
            let Some(previous_ids) = previous() else {
                return;
            };
            let target = previous_ids
                .iter()
                .zip(ids.iter())
                .find(|(a, b)| a != b)
                .map(|(a, _)| a.clone())
                .unwrap_or_else(|| previous_ids.first().cloned().unwrap_or_default());
            pending_focus.set(Some(target));
            previous.set(None);
            announcement.set("Previous order restored.".to_string());
            reorder_to(previous_ids, None, true);
        }
    };

    // The drag: measure the rows once, then reorder live as the pointer crosses
    // a neighbour's original box.
    let on_drag_start = {
        let ids = ids.clone();
        let mut drag_state = drag_state;
        EventHandler::new(move |(id, index, start_y): (String, usize, f64)| {
            if disabled {
                return;
            }
            drag_state.set(Some(DragState {
                id: id.clone(),
                start_y,
                original_index: index,
                target_index: index,
                start_order: ids.clone(),
                boxes: Vec::new(),
            }));
            let list = ids
                .iter()
                .map(|id| js_string(&format!("[data-sortable-row=\"{id}\"]")))
                .collect::<Vec<_>>()
                .join(",");
            spawn(async move {
                let script = format!(
                    "const selectors = [{list}];\n\
                     const out = [];\n\
                     for (const selector of selectors) {{\n\
                       const el = document.querySelector(selector);\n\
                       const rect = el ? el.getBoundingClientRect() : null;\n\
                       out.push(rect ? rect.top : 0, rect ? rect.height : 0);\n\
                     }}\n\
                     return out;",
                );
                let Some(values) = json::<Vec<f64>>(&script).await else {
                    return;
                };
                let boxes: Vec<(f64, f64)> =
                    values.chunks(2).map(|pair| (pair[0], pair[1])).collect();
                let mut state = drag_state;
                let mut guard = state.write();
                if let Some(open) = guard.as_mut() {
                    open.boxes = boxes;
                }
            });
        })
    };

    let on_drag_move = {
        let ids = ids.clone();
        let mut drag_state = drag_state;
        let mut reorder_to = reorder_to.clone();
        EventHandler::new(move |pointer_y: f64| {
            let Some(state) = drag_state() else {
                return;
            };
            if state.boxes.is_empty() {
                return;
            }
            let target = state
                .boxes
                .iter()
                .position(|(top, height)| pointer_y >= *top && pointer_y <= *top + *height)
                .unwrap_or_else(|| {
                    if pointer_y < state.boxes[0].0 {
                        0
                    } else {
                        state.boxes.len().saturating_sub(1)
                    }
                });
            if target == state.target_index {
                return;
            }
            let mut next = ids.clone();
            let Some(from) = next.iter().position(|candidate| *candidate == state.id) else {
                return;
            };
            let moved = next.remove(from);
            next.insert(target.min(next.len()), moved);
            drag_state.set(Some(DragState {
                target_index: target,
                ..state
            }));
            reorder_to(next, None, true);
        })
    };

    let on_drag_end = {
        let mut drag_state = drag_state;
        let mut previous = previous;
        let mut announcement = announcement;
        let items = items.clone();
        EventHandler::new(move |_: ()| {
            let Some(state) = drag_state() else {
                return;
            };
            drag_state.set(None);
            if state.target_index == state.original_index {
                return;
            }
            // The drag already committed the new order; undo restores the one it
            // started from.
            if previous().is_none() {
                previous.set(Some(state.start_order.clone()));
            }
            let position = state.target_index.min(items.len().saturating_sub(1)) + 1;
            let moved_label = items
                .iter()
                .find(|item| item.id == state.id)
                .map(|item| item.label.clone())
                .unwrap_or_default();
            announcement.set(format!(
                "{moved_label} moved to position {position} of {}.",
                items.len()
            ));
        })
    };

    let dragging_id = drag_state().map(|state| state.id.clone());
    let instructions_id = format!("sortable-instructions-{scope}");
    let root_class = cn!("w-full max-w-md", class);

    rsx! {
        div { class: root_class,
            p { id: "{instructions_id}", class: "sr-only",
                "Drag a handle to reorder. With a handle focused, use Up and Down to move, Home and End to move to the edges."
            }
            ul { aria_label: "{label}", class: "flex flex-col gap-2",
                for (index, item) in items.iter().enumerate() {
                    SortableRow {
                        key: "{item.id}",
                        id: item.id.clone(),
                        label: item.label.clone(),
                        index,
                        total: ids.len(),
                        disabled,
                        dragging: dragging_id.as_deref() == Some(item.id.as_str()),
                        drag_offset: drag_state()
                            .filter(|state| state.id == item.id)
                            .map(|state| state.layout_shift())
                            .unwrap_or(0.0),
                        item_class: item_class.clone(),
                        instructions_id: instructions_id.clone(),
                        on_drag_start,
                        on_drag_move,
                        on_drag_end,
                        on_move_key: {
                            let mut move_item = move_item.clone();
                            EventHandler::new(move |(id, direction): (String, i32)| move_item(id, direction))
                        },
                        {item.content.clone()}
                    }
                }
            }
            if show_undo {
                button {
                    r#type: "button",
                    disabled: disabled || !can_undo,
                    class: "mt-4 inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40",
                    onclick: {
                        let mut undo = undo.clone();
                        move |_| undo()
                    },
                    Undo { class: Some("size-3.5".to_string()) }
                    "Undo reorder"
                }
            }
            span { role: "status", aria_live: "polite", class: "sr-only", "{announcement()}" }
        }
    }
}

/// One row. Its index is the FLIP state, so a reorder animates the row from
/// where it was.
#[component]
fn SortableRow(
    id: String,
    label: String,
    index: usize,
    total: usize,
    disabled: bool,
    dragging: bool,
    /// The layout shift this row has taken since the drag started.
    drag_offset: f64,
    #[props(default)] item_class: Option<String>,
    instructions_id: String,
    on_drag_start: EventHandler<(String, usize, f64)>,
    on_drag_move: EventHandler<f64>,
    on_drag_end: EventHandler<()>,
    on_move_key: EventHandler<(String, i32)>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion();
    let flip = use_flip(
        index,
        FlipOptions::default().transition(AnimationConfig::spring(SPRING_LAYOUT)),
    );
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");
    use_effect(move || {
        if reduce() {
            return;
        }
        let target = if dragging { 1.025 } else { 1.0 };
        let _ = scale.animate_to(target, AnimationConfig::spring(SPRING_LAYOUT));
    });

    let row_class = cn!(
        "relative flex items-center gap-3 rounded-2xl border border-border bg-background p-3",
        if dragging { "shadow-lg" } else { "" },
        item_class,
    );
    // While dragging the row follows the finger, so it owns its transform; the
    // others take the FLIP pin.
    let style = if dragging {
        let scale_value = f64::from(*scale.current().read());
        format!(
            "transform:translateY({}px) scale({scale_value});z-index:10",
            -drag_offset
        )
    } else {
        flip.style_with("")
    };

    rsx! {
        li {
            "data-sortable-row": "{id}",
            "data-beui-flip": flip.scope(),
            aria_posinset: "{index + 1}",
            aria_setsize: "{total}",
            class: row_class,
            style,
            button {
                r#type: "button",
                "data-sortable-id": "{id}",
                disabled,
                aria_label: "Reorder {label}",
                aria_describedby: "{instructions_id}",
                class: "flex size-9 shrink-0 touch-none items-center justify-center rounded-lg text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-40",
                style: if disabled { String::new() } else { "cursor:grab".to_string() },
                onpointerdown: {
                    let handle_id = id.clone();
                    move |event: Event<PointerData>| {
                        if disabled {
                            return;
                        }
                        event.prevent_default();
                        // Capture so the drag survives the pointer leaving the handle.
                        let _ = capture_pointer(&event);
                        on_drag_start.call((handle_id.clone(), index, event.client_coordinates().y));
                    }
                },
                onpointermove: move |event: Event<PointerData>| {
                    if !disabled {
                        on_drag_move.call(event.client_coordinates().y);
                    }
                },
                onpointerup: move |event: Event<PointerData>| {
                    let _ = release_pointer(&event);
                    on_drag_end.call(());
                },
                onpointercancel: move |event: Event<PointerData>| {
                    let _ = release_pointer(&event);
                    on_drag_end.call(());
                },
                onkeydown: {
                    let key_id = id.clone();
                    move |event: Event<KeyboardData>| {
                        if disabled {
                            return;
                        }
                        let direction = match event.key() {
                            Key::ArrowUp => -1,
                            Key::ArrowDown => 1,
                            Key::Home => -2,
                            Key::End => 2,
                            _ => return,
                        };
                        event.prevent_default();
                        on_move_key.call((key_id.clone(), direction));
                    }
                },
                GripVertical { class: Some("size-4".to_string()) }
            }
            div { class: "min-w-0 flex-1", {children} }
        }
    }
}
