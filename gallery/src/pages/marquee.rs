//! `Marquee` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::Marquee;

const LOGOS: [&str; 8] = [
    "Vercel", "Linear", "Stripe", "Figma", "GitHub", "Notion", "Loom", "Raycast",
];

#[component]
pub fn MarqueePage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Marquee" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Two duplicated tracks drifting under one CSS keyframe, with the leading and trailing edges masked. Pauses on hover."
                }
            }
            section { class: "w-full",
                Marquee { speed: 25.0,
                    for logo in LOGOS {
                        div {
                            key: "{logo}",
                            class: "mx-4 flex h-12 items-center justify-center rounded-lg border border-border bg-card px-6 text-sm font-medium text-foreground",
                            "{logo}"
                        }
                    }
                }
            }
        }
    }
}
