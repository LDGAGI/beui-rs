//! The resource-tree sidebar, on screen: expand/collapse, selection, the row
//! menu (rename + the four moves), keyboard navigation and drag reordering.

use beui::prelude::*;
use beui_agents::ai_sidebar::{AISidebar, SidebarResource, SidebarResourceKind};

#[component]
pub fn Sidebar() -> Element {
    let items = vec![
        SidebarResource::folder("src", "src").children(vec![
            SidebarResource::new("lib", "lib.rs", SidebarResourceKind::File),
            SidebarResource::new("main", "main.rs", SidebarResourceKind::File),
        ]),
        SidebarResource::new("readme", "README.md", SidebarResourceKind::File),
        SidebarResource::new("api", "API Reference", SidebarResourceKind::Bookmark),
        SidebarResource::folder("archive", "archive")
            .disabled(true)
            .children(vec![SidebarResource::new(
                "old",
                "old-notes.md",
                SidebarResourceKind::File,
            )]),
    ];
    let expanded = vec!["src".to_string()];

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Agents"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "AI sidebar" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A resource tree: click a folder to expand, a file to select. Hover a row for its menu, or use the keyboard — arrows to move, F2 to rename, Alt+Shift+Arrow to reorder."
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "AISidebar" }
                div { class: "w-72 rounded-2xl border border-border bg-card p-2",
                    AISidebar { items, default_expanded_ids: expanded, default_active_id: Some("readme".to_string()) }
                }
            }
        }
    }
}
