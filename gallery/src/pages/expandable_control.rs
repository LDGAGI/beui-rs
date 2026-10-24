//! `ExpandableControl` — the beUI preview, same copy and structure.

use beui::icons::{Bell, X};
use beui::prelude::*;
use beui::{ExpandableButton, ExpandableChip};

#[component]
pub fn ExpandableControlPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · control"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ExpandableControl" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A pill button and a chip whose labels expand on CONTINUITY_SPRING. Motion's nested projection is a known gap, so the label's own width springs from 0 to its measured natural width instead."
                }
            }
            div { class: "flex flex-col items-center gap-6",
                div { class: "flex items-center gap-2",
                    ExpandableButton {
                        icon: rsx! { Bell { class: Some("size-4".to_string()) } },
                        label: rsx! { "Notifications" },
                    }
                }
                div { class: "flex items-center gap-2",
                    ExpandableChip {
                        label: rsx! { "React" },
                        action_icon: rsx! { X { class: Some("size-3.5".to_string()) } },
                        action_label: "Remove React",
                    }
                }
            }
        }
    }
}
