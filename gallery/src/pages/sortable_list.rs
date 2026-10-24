//! `SortableList` / `SortableStack` — the beUI preview: studio priorities.

use beui::icons::{Circle, FileText, Layers, Palette, Rocket};
use beui::prelude::*;
use beui::{SortableItem, SortableList};

struct Task {
    id: &'static str,
    title: &'static str,
    detail: &'static str,
    color: &'static str,
}

const TASKS: [Task; 4] = [
    Task {
        id: "research",
        title: "Collect references",
        detail: "Find the right direction",
        color: "bg-amber-500/10 text-amber-600 dark:text-amber-400",
    },
    Task {
        id: "design",
        title: "Explore the visual system",
        detail: "Type, color, and composition",
        color: "bg-rose-500/10 text-rose-600 dark:text-rose-400",
    },
    Task {
        id: "build",
        title: "Build the first prototype",
        detail: "Make the interaction tangible",
        color: "bg-blue-500/10 text-blue-600 dark:text-blue-400",
    },
    Task {
        id: "ship",
        title: "Ship something useful",
        detail: "Polish the details and release",
        color: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400",
    },
];

fn task_icon(id: &str) -> Element {
    let class = Some("size-4.5".to_string());
    match id {
        "research" => rsx! { FileText { class } },
        "design" => rsx! { Palette { class } },
        "build" => rsx! { Layers { class } },
        _ => rsx! { Rocket { class } },
    }
}

#[component]
pub fn SortableListPage() -> Element {
    let mut order = use_signal(|| {
        TASKS
            .iter()
            .map(|task| task.id.to_string())
            .collect::<Vec<_>>()
    });

    let items: Vec<SortableItem> = order()
        .iter()
        .enumerate()
        .filter_map(|(index, id)| {
            let task = TASKS.iter().find(|task| task.id == id)?;
            let content = rsx! {
                div { class: "flex items-center gap-3",
                    span { class: "flex size-10 shrink-0 items-center justify-center rounded-xl {task.color}",
                        {task_icon(task.id)}
                    }
                    div { class: "min-w-0 flex-1",
                        p { class: "text-sm font-medium", "{task.title}" }
                        p { class: "mt-0.5 text-xs text-muted-foreground", "{task.detail}" }
                    }
                    span { class: "text-[10px] tabular-nums text-muted-foreground",
                        "{index + 1:02}"
                    }
                    Circle { class: Some("size-3.5 text-border".to_string()) }
                }
            };
            Some(SortableItem::new(task.id, task.title, content))
        })
        .collect();

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "SortableList" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A reorderable list. Drag a handle to reorder live, use Up and Down on a focused handle, and undo restores the order the last move replaced."
                }
            }
            div { class: "flex min-h-40 w-full justify-center",
                div { class: "w-full max-w-md", "data-order": "{order().join(\",\")}",
                    div { class: "mb-5 flex items-end justify-between gap-4 px-1",
                        div {
                            p { class: "mb-1 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground",
                                "In the studio"
                            }
                            h3 { class: "text-lg font-medium tracking-tight", "Make room for what matters." }
                        }
                        span { class: "shrink-0 text-xs tabular-nums text-muted-foreground",
                            "{order().len():02} tasks"
                        }
                    }
                    SortableList {
                        items,
                        label: "Studio priorities".to_string(),
                        on_reorder: EventHandler::new(move |next: Vec<String>| order.set(next)),
                    }
                }
            }
        }
    }
}
