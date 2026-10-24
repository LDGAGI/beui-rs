//! `ProjectFolder` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{ProjectFolder, ProjectFolderPreview};

const PREVIEWS: [(&str, &str, &str); 5] = [
    ("moss", "bg-emerald-200", "A"),
    ("clay", "bg-orange-200", "B"),
    ("sky", "bg-sky-200", "C"),
    ("lilac", "bg-violet-200", "D"),
    ("sand", "bg-amber-100", "E"),
];

fn previews() -> Vec<ProjectFolderPreview> {
    PREVIEWS
        .iter()
        .map(|(id, color, mark)| {
            ProjectFolderPreview::new(
                *id,
                rsx! {
                    span { class: "relative block h-full w-full {color}",
                        span { class: "absolute left-3 top-3 h-2 w-8 rounded-full bg-black/15" }
                        span { class: "absolute inset-x-3 top-8 h-px bg-black/10" }
                        span { class: "absolute inset-x-3 top-11 h-px bg-black/10" }
                        span { class: "absolute bottom-3 right-3 text-sm font-medium text-black/50", "{mark}" }
                    }
                },
            )
        })
        .collect()
}

#[component]
pub fn ProjectFolderPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · block"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ProjectFolder" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A folder whose previews fan out on hover and open into a full-screen overlay. The overlay renders in place (portaled subtrees lose Dioxus's delegated clicks) and its cards enter with a scale/fade rather than the source's layoutId morph."
                }
            }
            div { class: "flex min-h-80 w-full items-center justify-center px-6 py-10",
                ProjectFolder {
                    title: "Brand direction".to_string(),
                    description: "Updated recently".to_string(),
                    count: Some(5),
                    previews: previews(),
                    on_click: Some(EventHandler::new(|_| {})),
                }
            }
        }
    }
}
