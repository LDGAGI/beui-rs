//! `tooltip.preview.tsx` — four placements with arrows.

use beui::prelude::*;
use beui::{
    ArrowDownIcon, ArrowLeftIcon, ArrowRightIcon, ArrowUpIcon, Tooltip, TooltipAlign,
    TooltipContent, TooltipSide, TooltipTrigger,
};

const PLACEMENTS: [(&str, TooltipSide, &str, &str); 4] = [
    ("Top", TooltipSide::Top, "col-start-2 row-start-1", "up"),
    ("Left", TooltipSide::Left, "col-start-1 row-start-2", "left"),
    (
        "Right",
        TooltipSide::Right,
        "col-start-3 row-start-2",
        "right",
    ),
    (
        "Bottom",
        TooltipSide::Bottom,
        "col-start-2 row-start-3",
        "down",
    ),
];

fn placement_icon(kind: &str) -> Element {
    match kind {
        "up" => rsx! { ArrowUpIcon { class: "size-3.5 text-muted-foreground" } },
        "left" => rsx! { ArrowLeftIcon { class: "size-3.5 text-muted-foreground" } },
        "right" => rsx! { ArrowRightIcon { class: "size-3.5 text-muted-foreground" } },
        _ => rsx! { ArrowDownIcon { class: "size-3.5 text-muted-foreground" } },
    }
}

#[component]
pub fn TooltipPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Tooltip" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Positioning is geometry, not animation: the label is measured against its anchor and placed with the same offset / flip / shift / arrow / hide middleware the source configures, then writes one transform per frame. Hover or focus; Escape dismisses."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    rsx! {
        section { class: "flex w-full flex-col items-center gap-8 py-6",
            div { class: "grid grid-cols-3 gap-x-3 gap-y-5",
                for (label, side, position, kind) in PLACEMENTS {
                    Tooltip { key: "{label}", side, align: TooltipAlign::Center, arrow: true,
                        TooltipTrigger { class: cn!(position, "inline-flex h-10 w-20 items-center justify-center gap-1.5 rounded-lg border border-border bg-card text-xs font-medium text-foreground transition-colors hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring"),
                            {placement_icon(kind)}
                            "{label}"
                        }
                        TooltipContent { text: "More context" }
                    }
                }
            }
            p { class: "text-center text-xs text-muted-foreground",
                "Hover or focus. Press Esc to dismiss."
            }
        }
    }
}
