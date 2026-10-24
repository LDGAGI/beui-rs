//! `bouncy-accordion.preview.tsx` — six connected rows, one open.

use beui::prelude::*;
use beui::{BouncyAccordion, BouncyAccordionItem};

const ITEMS: [(&str, &str, &str); 6] = [
    (
        "brief",
        "Release Brief",
        "Collect launch notes, owners, and risks in one compact handoff before the release window opens.",
    ),
    (
        "launch",
        "Launch Checklist",
        "Verify copy, links, analytics, rollback steps, and final approvals without leaving the queue.",
    ),
    (
        "campaign",
        "Campaign Notes",
        "Keep channel-specific notes close to the task while preserving a calm collapsed list.",
    ),
    (
        "calendar",
        "Rollout Calendar",
        "Plan announcements, staging checks, reminders, and quiet periods around the same timeline.",
    ),
    (
        "ship",
        "Ship Build",
        "Track the current artifact, deploy status, and final sign-off before marking the release complete.",
    ),
    (
        "archive",
        "Archive Assets",
        "Move final copy, images, and source files into the campaign folder once the rollout is done.",
    ),
];

#[component]
pub fn BouncyAccordionPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Bouncy accordion" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The open row is a rounded card and its neighbours drop the corners that touch it, so a run reads as one group. Every row's position is a layout animation, so a row pushed by a neighbour's expansion starts where it was and springs to its new place."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let items = ITEMS
        .iter()
        .map(|(id, title, description)| {
            BouncyAccordionItem::new(*id, *title).description(*description)
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "flex min-h-96 w-full items-center justify-center",
            div { class: "h-[480px] w-full max-w-sm",
                BouncyAccordion {
                    items,
                    default_value: Some("calendar".to_string()),
                }
            }
        }
    }
}
