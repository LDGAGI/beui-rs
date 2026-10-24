//! `agents/todo-list.tsx` — the collapsible agent task list.
//!
//! DOM and classes follow the source: an 11-high trigger row (header glyph,
//! title, `completed/total`, chevron) over a [`AgentDisclosure`] viewport of
//! status rows. The source animates a roll on the counter and springs on the
//! status glyphs through motion; the port keeps the same shapes and encodes the
//! state changes as style-driven transitions .

use beui::prelude::*;

use crate::common::{use_unique_id, Icon, RollText};
use crate::disclosure::AgentDisclosure;
use crate::icons::IconName;

/// `TodoItemStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TodoItemStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

/// `TodoItem`.
#[derive(Clone, Debug, PartialEq)]
pub struct TodoItem {
    pub id: String,
    pub title: Element,
    pub status: TodoItemStatus,
    /// 0–100 for a determinate in-progress ring; `None` spins.
    pub progress: Option<f64>,
    pub detail: Option<Element>,
}

impl TodoItem {
    pub fn new(id: impl Into<String>, title: Element) -> Self {
        Self {
            id: id.into(),
            title,
            status: TodoItemStatus::Pending,
            progress: None,
            detail: None,
        }
    }

    pub fn status(mut self, status: TodoItemStatus) -> Self {
        self.status = status;
        self
    }

    pub fn progress(mut self, progress: f64) -> Self {
        self.progress = Some(progress);
        self
    }

    pub fn detail(mut self, detail: Element) -> Self {
        self.detail = Some(detail);
        self
    }
}

fn status_label(status: TodoItemStatus) -> &'static str {
    match status {
        TodoItemStatus::Pending => "Pending",
        TodoItemStatus::InProgress => "In progress",
        TodoItemStatus::Completed => "Completed",
        TodoItemStatus::Cancelled => "Cancelled",
    }
}

/// `TodoHeaderIcon`.
#[component]
fn TodoHeaderIcon(complete: bool) -> Element {
    let reduce = use_reduced_motion()();
    let duration = if reduce { 0 } else { 200 };
    let check_style = format!(
        "opacity:{};transform:scale({});transition:opacity {duration}ms,transform {duration}ms;",
        if complete { 1 } else { 0 },
        if complete { 1.0 } else { 0.72 },
    );
    let todo_style = format!(
        "opacity:{};transform:scale({});transition:opacity {duration}ms,transform {duration}ms;",
        if complete { 0 } else { 1 },
        if complete { 0.72 } else { 1.0 },
    );
    rsx! {
        span {
            "aria-hidden": "true",
            class: "relative grid size-6 shrink-0 place-items-center",
            svg {
                view_box: "0 0 24 24",
                class: "absolute size-5.5 overflow-visible text-emerald-500",
                style: "{check_style}",
                circle { cx: "12", cy: "12", r: "9", fill: "currentColor" }
                path {
                    d: "M7.5 12.25 10.5 15.25 16.75 8.75",
                    fill: "none",
                    stroke: "white",
                    "stroke-width": "2.25",
                    "stroke-linecap": "round",
                    "stroke-linejoin": "round",
                }
            }
            span {
                class: "absolute grid place-items-center text-muted-foreground",
                style: "{todo_style}",
                Icon { name: IconName::ListTodo, class: "size-4" }
            }
        }
    }
}

/// `TodoStatusIcon`.
#[component]
fn TodoStatusIcon(status: TodoItemStatus, progress: Option<f64>) -> Element {
    let reduce = use_reduced_motion()();
    let normalized = progress
        .map(|p| p.clamp(0.0, 100.0) / 100.0)
        .unwrap_or(0.68);
    let in_progress = status == TodoItemStatus::InProgress;
    let completed = status == TodoItemStatus::Completed;
    let cancelled = status == TodoItemStatus::Cancelled;
    let spin = in_progress && progress.is_none() && !reduce;

    let disc_style = format!(
        "fill-opacity:{};{}",
        if completed { 0.06 } else { 0.0 },
        if in_progress { "opacity:0.2;" } else { "" },
    );
    let arc_style = format!(
        "transform-origin:12px 12px;transform:rotate(-90deg);opacity:{};",
        if in_progress { 1 } else { 0 },
    );
    let check_style = format!("opacity:{};", if completed { 1 } else { 0 });
    let cross_style = format!("opacity:{};", if cancelled { 1 } else { 0 });

    rsx! {
        svg {
            "aria-hidden": "true",
            view_box: "0 0 24 24",
            class: cn!(
                "mx-0.5 size-5 shrink-0 overflow-visible text-muted-foreground",
                if in_progress { "text-foreground" } else { "" },
                if cancelled { "text-rose-600 dark:text-rose-400" } else { "" },
            ),
            circle {
                cx: "12",
                cy: "12",
                r: "9",
                fill: "currentColor",
                stroke: "currentColor",
                "stroke-width": "1.5",
                "stroke-dasharray": if status == TodoItemStatus::Pending { "2 3" } else { "" },
                "stroke-linecap": "round",
                style: "{disc_style}",
            }
            circle {
                cx: "12",
                cy: "12",
                r: "9",
                "pathLength": "1",
                fill: "none",
                stroke: "currentColor",
                "stroke-width": "2",
                "stroke-linecap": "round",
                "stroke-dasharray": "{normalized} 1",
                style: "{arc_style}",
                class: if spin { "animate-spin" } else { "" },
            }
            path {
                d: "M7.5 12.25 10.5 15.25 16.75 8.75",
                fill: "none",
                stroke: "currentColor",
                "stroke-width": "2",
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                style: "{check_style}",
            }
            path {
                d: "M8.5 8.5 15.5 15.5M15.5 8.5 8.5 15.5",
                fill: "none",
                stroke: "currentColor",
                "stroke-width": "2",
                "stroke-linecap": "round",
                style: "{cross_style}",
            }
        }
    }
}

/// `TodoList`.
#[component]
pub fn TodoList(
    items: Vec<TodoItem>,
    #[props(default)] title: Option<Element>,
    #[props(default)] open: Option<bool>,
    #[props(default = true)] default_open: bool,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = true)] collapse_on_complete: bool,
    #[props(default = 248)] max_height: i32,
    #[props(default)] class: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let base = use_unique_id("todo");
    let trigger_id = format!("{base}-trigger");
    let content_id = format!("{base}-content");
    let viewport = format!("{base}-viewport");
    let mut internal_open = use_signal(|| default_open);

    let completed = items
        .iter()
        .filter(|item| item.status == TodoItemStatus::Completed)
        .count();
    let item_count = items.len();
    let all_complete = item_count > 0 && completed == item_count;
    let current_open = open.unwrap_or(internal_open());

    let mut set_open = move |next: bool| {
        if open.is_none() {
            internal_open.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    };

    // Collapse once every task completes, reopen when work returns.
    //
    // `previous` is a `Cell`, not a signal: an effect that reads and writes a
    // signal re-runs itself forever (Dioxus `set` notifies even for an equal
    // value), which is a hung main thread. The source's `useRef` has the same
    // "no re-render" property.
    let previous_complete = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(all_complete)));
    use_effect(use_reactive((&all_complete,), move |(all_complete,)| {
        let previous = previous_complete.get();
        let target = if previous && !all_complete {
            Some(true)
        } else if !previous && all_complete && collapse_on_complete {
            Some(false)
        } else {
            None
        };
        if let Some(target) = target {
            if open.is_none() {
                internal_open.set(target);
            }
            if let Some(handler) = on_open_change {
                handler.call(target);
            }
        }
        previous_complete.set(all_complete);
    }));

    // Keep the newest task in view as items stream in.
    let scroll_selector = format!("[data-beui-todo-viewport=\"{viewport}\"]");
    use_effect(use_reactive((&item_count,), move |_| {
        crate::common::scroll_to_bottom(&scroll_selector, !reduce);
    }));

    let rotate = if current_open { 180 } else { 0 };
    let chevron_duration = if reduce { 0 } else { 200 };
    let strike_duration = if reduce { 0 } else { 280 };

    rsx! {
        section {
            "aria-label": "Agent task list",
            class: cn!("w-full overflow-hidden rounded-2xl border border-border/70", class),
            button {
                id: "{trigger_id}",
                r#type: "button",
                "aria-expanded": if current_open { "true" } else { "false" },
                "aria-controls": "{content_id}",
                onclick: move |_| set_open(!current_open),
                class: "group flex h-11 w-full items-center gap-2.5 rounded-2xl px-3.5 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring",
                TodoHeaderIcon { complete: all_complete }
                h3 { class: "min-w-0 flex-1 truncate text-sm font-medium text-foreground/90",
                    {title.unwrap_or_else(|| rsx! { "To-dos" })}
                }
                span {
                    class: cn!(
                        "shrink-0 text-xs font-medium tabular-nums text-muted-foreground",
                        if all_complete { "text-emerald-600 dark:text-emerald-400" } else { "" },
                    ),
                    span { class: "sr-only", "{completed} of {item_count} tasks completed" }
                    span { "aria-hidden": "true", class: "inline-flex",
                        RollText { value: "{completed}" }
                        span { "/" }
                        span { "{item_count}" }
                    }
                }
                span {
                    "aria-hidden": "true",
                    class: "text-muted-foreground/50 group-hover:text-muted-foreground",
                    style: "transform:rotate({rotate}deg);transition:transform {chevron_duration}ms cubic-bezier(0.34,1.56,0.64,1);",
                    Icon { name: IconName::ChevronDown, class: "size-3.5" }
                }
            }

            AgentDisclosure {
                id: content_id.clone(),
                role: "region",
                aria_labelledby: trigger_id.clone(),
                open: current_open,
                div {
                    "data-beui-todo-viewport": "{viewport}",
                    class: "scrollbar-hide overflow-y-auto px-2 pb-2",
                    style: "max-height:{max_height}px",
                    if item_count > 0 {
                        ol { "aria-live": "polite", class: "space-y-0",
                            for item in items.iter() {
                                {
                                    let done = item.status == TodoItemStatus::Completed;
                                    let strike_style = format!(
                                        "transform:scaleX({});opacity:{};transition:transform {strike_duration}ms cubic-bezier(0.16,1,0.3,1) 60ms,opacity {strike_duration}ms;",
                                        if done { 1 } else { 0 },
                                        if done { 1 } else { 0 },
                                    );
                                    let title_class = cn!(
                                        "min-w-0 flex-1 truncate text-sm leading-5",
                                        match item.status {
                                            TodoItemStatus::Pending => "text-muted-foreground/65",
                                            TodoItemStatus::InProgress => "text-foreground",
                                            TodoItemStatus::Completed => "text-muted-foreground/60",
                                            TodoItemStatus::Cancelled => "text-muted-foreground/55",
                                        },
                                    );
                                    rsx! {
                                        li {
                                            key: "{item.id}",
                                            class: "flex min-h-9 items-center gap-2.5 rounded-xl px-1.5 py-1",
                                            TodoStatusIcon { status: item.status, progress: item.progress }
                                            span { class: "sr-only", "{status_label(item.status)}: " }
                                            span { class: "{title_class}",
                                                span { class: "relative inline-block max-w-full",
                                                    {item.title.clone()}
                                                    span {
                                                        "aria-hidden": "true",
                                                        class: "absolute inset-x-0 top-1/2 h-px origin-left bg-current",
                                                        style: "{strike_style}",
                                                    }
                                                }
                                            }
                                            if let Some(detail) = item.detail.clone() {
                                                span { class: "shrink-0 text-sm text-muted-foreground/55", {detail} }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        p { class: "px-1.5 py-2 text-sm text-muted-foreground", "No tasks yet" }
                    }
                }
            }
        }
    }
}
