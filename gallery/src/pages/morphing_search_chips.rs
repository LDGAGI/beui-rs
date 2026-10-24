//! `MorphingSearchChips` — the beUI preview: a search bar floating over a tile
//! backdrop, with chips, recents and suggestions.

use beui::prelude::*;
use beui::MorphingSearchChips;

const TILES: [&str; 6] = [
    "from-rose-400/40 to-orange-300/30",
    "from-sky-400/40 to-indigo-400/30",
    "from-emerald-400/40 to-teal-300/30",
    "from-amber-300/40 to-rose-300/30",
    "from-violet-400/40 to-fuchsia-300/30",
    "from-teal-300/40 to-sky-300/30",
];

const SUGGESTIONS: [&str; 8] = [
    "dashboards",
    "data tables",
    "dark mode",
    "empty states",
    "onboarding",
    "pricing pages",
    "settings",
    "sign in",
];

#[component]
pub fn MorphingSearchChipsPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "MorphingSearchChips" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A search field with filter chips whose results panel grows out of the bar's edge: a small pill tucked into the bar springs out to the size of what it holds, and the content waits for the shape to arrive."
                }
            }
            div { class: "relative flex h-[440px] w-full max-w-2xl flex-col justify-end overflow-hidden rounded-3xl border border-border bg-muted/40 p-4",
                div { "aria-hidden": "true", class: "absolute inset-4 grid grid-cols-3 gap-3 opacity-80",
                    for tile in TILES {
                        div { key: "{tile}", class: "rounded-2xl bg-gradient-to-br {tile}" }
                    }
                }
                MorphingSearchChips {
                    class: Some("mx-auto".to_string()),
                    placeholder: "Search inspiration".to_string(),
                    label: "Search inspiration".to_string(),
                    default_chips: vec!["onboarding".to_string()],
                    default_recents: vec![
                        "dashboards".to_string(),
                        "empty states".to_string(),
                        "pricing pages".to_string(),
                    ],
                    storage_key: Some("beui:morphing-search-chips:recents".to_string()),
                    max_chips: Some(4),
                    suggestions: SUGGESTIONS.iter().map(|term| term.to_string()).collect(),
                }
            }
        }
    }
}
