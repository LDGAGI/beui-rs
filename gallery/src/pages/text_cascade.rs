//! `TextCascade` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::TextCascade;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

const PHRASES: [&str; 3] = ["Install skills", "Open settings", "Ship updates"];

#[component]
pub fn TextCascadePage() -> Element {
    let mut phrase = use_signal(|| 0usize);
    use_future(move || async move {
        loop {
            if Time::delay(Duration::from_millis(2400)).await.is_err() {
                return;
            }
            phrase += 1;
        }
    });

    let text = PHRASES[phrase() % PHRASES.len()].to_string();
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · text"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "TextCascade" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Letter-by-letter slot roll for standalone text — the old letters drop away as the new ones land, left to right."
                }
            }
            div { class: "flex w-full justify-center",
                p { class: "text-lg font-medium text-foreground",
                    TextCascade { text }
                }
            }
        }
    }
}
