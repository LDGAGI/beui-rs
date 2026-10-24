//! `BounceSidebar` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{BounceSidebar, BounceSidebarItem};

const DESTINATIONS: [(&str, &str); 5] = [
    ("overview", "Overview"),
    ("components", "Components"),
    ("motion", "Motion"),
    ("templates", "Templates"),
    ("changelog", "Changelog"),
];

#[component]
pub fn BounceSidebarPage() -> Element {
    let active = use_signal(|| "components".to_string());
    let items = DESTINATIONS
        .iter()
        .map(|(id, label)| BounceSidebarItem::new(*id, *label))
        .collect::<Vec<_>>();
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · navigation"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "BounceSidebar" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A selection dot that hops sideways between rows on a lightly underdamped spring, softened as the jump gets longer. The sideways arc is reproduced in the transform (offset-path is not portable across renderers)."
                }
            }
            div { class: "flex min-h-[360px] w-full items-center justify-center",
                BounceSidebar {
                    items,
                    value: Some(active()),
                    on_value_change: Some(EventHandler::new({
                        let mut active = active;
                        move |id: String| active.set(id)
                    })),
                    aria_label: "beUI sections",
                    class: "w-52",
                    list_class: "w-full",
                    item_class: "text-base",
                }
            }
        }
    }
}
