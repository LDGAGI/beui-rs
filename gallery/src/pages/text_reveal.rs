//! `TextReveal` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::TextReveal;

#[component]
pub fn TextRevealPage() -> Element {
    let mut key = use_signal(|| 0u32);
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · text"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "TextReveal" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Words (or characters) arriving one at a time on a spring, with opacity and blur on EASE_OUT tweens."
                }
            }
            div { class: "flex w-full flex-col items-center gap-8 text-center",
                div { key: "{key()}", class: "flex flex-col gap-2",
                    TextReveal {
                        r#as: beui::TextTag::H2,
                        text: vec!["Motion that feels".to_string(), "considered.".to_string()],
                        class: "text-balance text-4xl font-semibold leading-[0.95] tracking-[-0.04em] text-foreground sm:text-5xl",
                    }
                    TextReveal {
                        text: vec!["Word by word, with a soft blur.".to_string()],
                        delay: 0.9,
                        stagger: 0.05,
                        blur: 6.0,
                        y_offset: "20%".to_string(),
                        class: "text-sm text-muted-foreground",
                    }
                }
                button {
                    r#type: "button",
                    class: "press inline-flex h-9 items-center rounded-full border border-border bg-card px-4 text-xs font-medium text-foreground hover:border-(--color-border-strong)",
                    onclick: move |_| key += 1,
                    "Replay"
                }
            }
        }
    }
}
