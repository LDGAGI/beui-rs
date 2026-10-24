//! `ChromaticTextReveal` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::ChromaticTextReveal;

#[component]
pub fn ChromaticTextRevealPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · text"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ChromaticTextReveal" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A clipped text gradient sweeping across one word at a time. The sweep is a CSS custom property animated from -14% to 114%."
                }
            }
            div { class: "@container flex w-full justify-center",
                ChromaticTextReveal {
                    prefix: "Motion that feels".to_string(),
                    words: vec!["natural.".to_string(), "intentional.".to_string(), "alive.".to_string()],
                    start_on_view: false,
                    class: "shrink-0 font-medium tracking-[-0.04em] text-foreground [font-size:clamp(1.25rem,7.8cqw,3rem)]",
                }
            }
        }
    }
}
