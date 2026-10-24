//! `MorphingSearch` — the beUI preview: a full trigger plus an icon-only one.

use beui::icons::{BookOpen, Bot, FolderOpen, LayoutGrid, Palette};
use beui::prelude::*;
use beui::{MorphingSearch, MorphingSearchItem};

fn items() -> Vec<MorphingSearchItem> {
    vec![
        MorphingSearchItem::new("project-folder", "Project Folder")
            .description("Block · Files and previews")
            .keywords(&["files", "overlay"])
            .icon(rsx! { FolderOpen { class: None } }),
        MorphingSearchItem::new("motion-components", "Motion components")
            .description("Collection · Interaction primitives")
            .keywords(&["animation", "components"])
            .icon(rsx! { LayoutGrid { class: None } }),
        MorphingSearchItem::new("agent-interfaces", "Agent interfaces")
            .description("Collection · AI building blocks")
            .keywords(&["ai", "chat"])
            .icon(rsx! { Bot { class: None } }),
        MorphingSearchItem::new("installation", "Installation guide")
            .description("Documentation · Add your first component")
            .keywords(&["setup", "shadcn"])
            .icon(rsx! { BookOpen { class: None } }),
        MorphingSearchItem::new("design-tokens", "Design tokens")
            .description("Documentation · Color, type, and motion")
            .keywords(&["theme", "styles"])
            .icon(rsx! { Palette { class: None } }),
    ]
}

#[component]
pub fn MorphingSearchPage() -> Element {
    let mut picked = use_signal(String::new);

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "MorphingSearch" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A search field that morphs into a command panel anchored over it: the trigger hands its shape to the panel through a shared layout id, the clip-path unfolds while the shell springs, and the results arrive a beat later."
                }
                p { class: "text-xs text-muted-foreground",
                    "Picked: " span { class: "font-mono text-foreground", "{picked()}" }
                }
            }
            div { class: "flex min-h-60 w-full max-w-[22rem] items-center gap-3",
                MorphingSearch {
                    items: items(),
                    placeholder: "Find components".to_string(),
                    on_select: Some(EventHandler::new(move |id: String| picked.set(id))),
                }
                MorphingSearch {
                    items: items(),
                    placeholder: "Find components".to_string(),
                    shortcut: Some(String::new()),
                    icon_only: true,
                    on_select: Some(EventHandler::new(move |id: String| picked.set(id))),
                }
            }
        }
    }
}
