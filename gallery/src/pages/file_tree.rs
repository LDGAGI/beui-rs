//! `FileTree` — the beUI preview, same copy and structure.

use beui::icons::{Braces, FileCode2, FileJson2, FileText, Palette};
use beui::prelude::*;
use beui::{FileTree, FileTreeItem};

fn tree() -> Vec<FileTreeItem> {
    vec![
        FileTreeItem::folder("app", "app").children(vec![
            FileTreeItem::folder("components", "components").children(vec![
                FileTreeItem::file("file-tree", "file-tree.tsx").icon(rsx! {
                    Braces { class: Some("size-4 text-sky-500".to_string()) }
                }),
                FileTreeItem::file("button", "button.tsx").icon(rsx! {
                    Braces { class: Some("size-4 text-sky-500".to_string()) }
                }),
            ]),
            FileTreeItem::file("page", "page.tsx").icon(rsx! {
                FileCode2 { class: Some("size-4 text-sky-500".to_string()) }
            }),
            FileTreeItem::file("styles", "globals.css").icon(rsx! {
                Palette { class: Some("size-4 text-violet-500".to_string()) }
            }),
        ]),
        FileTreeItem::folder("public", "public").children(vec![
            FileTreeItem::file("logo", "logo.svg"),
            FileTreeItem::file("grid", "grid.svg"),
        ]),
        FileTreeItem::file("package", "package.json").icon(rsx! {
            FileJson2 { class: Some("size-4 text-amber-500".to_string()) }
        }),
        FileTreeItem::file("readme", "README.md").icon(rsx! {
            FileText { class: Some("size-4 text-muted-foreground".to_string()) }
        }),
    ]
}

#[component]
pub fn FileTreePage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · block"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "FileTree" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A keyboard-navigable tree: Up/Down/Home/End, Right to open or step in, Left to close or step out, Enter/Space to select. The hover pill is SharedLayoutBg; selection is a plain class."
                }
            }
            div { class: "flex min-h-[420px] w-full items-start justify-center px-4 pt-10",
                div { class: "w-full max-w-xs p-2",
                    FileTree {
                        items: tree(),
                        default_value: Some("file-tree".to_string()),
                        default_expanded_ids: vec!["app".to_string(), "components".to_string()],
                        aria_label: "Project files",
                    }
                }
            }
        }
    }
}
