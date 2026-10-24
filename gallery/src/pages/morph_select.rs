//! `select-morph.preview.tsx` — the trigger becomes the panel.

use beui::prelude::*;
use beui::{
    MorphSelect, MorphSelectContent, MorphSelectItem, MorphSelectTrigger, MorphSelectValue,
};

const FRAMEWORKS: [(&str, &str); 4] = [
    ("next", "Next.js"),
    ("remix", "Remix"),
    ("astro", "Astro"),
    ("vite", "Vite"),
];

#[component]
pub fn MorphSelectPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Morph select" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The trigger and the panel are two elements sharing one layoutId: the trigger is only rendered while closed, the panel only while open, and the panel starts from the trigger's measured box. Nothing detaches — the field grows into the menu and shrinks back."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let value = use_signal(|| Some("next".to_string()));
    let mut set_value = value;

    rsx! {
        section { class: "flex flex-col gap-4",
            h2 { class: "text-lg font-semibold tracking-tight", "Morph select" }
            p { class: "max-w-3xl text-sm text-muted-foreground",
                "Controlled value; click the field to morph it open, then pick an option."
            }
            div { class: "w-56 pt-2",
                MorphSelect {
                    value: value(),
                    on_value_change: move |next| set_value.set(Some(next)),
                    MorphSelectTrigger {
                        MorphSelectValue { placeholder: "Pick a framework" }
                    }
                    MorphSelectContent {
                        for (item_value, label) in FRAMEWORKS {
                            MorphSelectItem { key: "{item_value}", value: item_value, label, "{label}" }
                        }
                    }
                }
            }
        }
    }
}
