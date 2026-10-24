//! `TextScramble` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::TextScramble;

const PHRASES: [&str; 3] = [
    "Inspecting the repository",
    "Running the checks",
    "Preparing the update",
];

#[component]
pub fn TextScramblePage() -> Element {
    let mut index = use_signal(|| 0usize);
    let text = PHRASES[index() % PHRASES.len()].to_string();
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · text"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "TextScramble" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Characters resolve left to right out of a glyph soup. Reduced motion shows the text straight away."
                }
            }
            div { class: "flex w-full flex-col items-center gap-8 text-center",
                TextScramble {
                    text,
                    class: "font-mono text-xl font-medium text-foreground",
                }
                button {
                    r#type: "button",
                    class: "press inline-flex h-9 items-center rounded-full border border-border bg-card px-4 text-xs font-medium text-foreground hover:border-(--color-border-strong)",
                    onclick: move |_| index += 1,
                    "Next phrase"
                }
            }
        }
    }
}
