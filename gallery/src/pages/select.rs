//! `select.preview.tsx` — the field, its list, and the fold it makes.

use beui::prelude::*;
use beui::{Select, SelectContent, SelectItem, SelectTrigger, SelectValue};

const FRAMEWORKS: [(&str, &str); 4] = [
    ("next", "Next.js"),
    ("remix", "Remix"),
    ("astro", "Astro"),
    ("vite", "Vite"),
];

#[component]
pub fn SelectPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Select" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The panel is absolutely positioned inside the field, so two open selects paint over each other. On open it measures the room below and flips above when there is more there, and the gap on the side facing the trigger opens as the near corners round."
                }
            }
            Preview {}
            FlipDemo {}
        }
    }
}

#[component]
fn Panel(title: String, note: String, children: Element) -> Element {
    rsx! {
        section { class: "flex flex-col gap-4",
            div { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold tracking-tight", "{title}" }
                p { class: "max-w-3xl text-sm text-muted-foreground", "{note}" }
            }
            {children}
        }
    }
}

#[component]
fn Preview() -> Element {
    let value = use_signal(|| Some("next".to_string()));
    let mut set_value = value;

    rsx! {
        Panel {
            title: "Select",
            note: "Controlled value, four options, and a check beside the selected one.",
            div { class: "w-56",
                Select {
                    value: value(),
                    on_value_change: move |next| set_value.set(Some(next)),
                    SelectTrigger {
                        SelectValue { placeholder: "Pick a framework" }
                    }
                    SelectContent {
                        for (value, label) in FRAMEWORKS {
                            SelectItem { key: "{value}", value, label, "{label}" }
                        }
                    }
                }
            }
        }
    }
}

/// A short page puts the trigger near the viewport floor, so the panel flips up.
#[component]
fn FlipDemo() -> Element {
    let value = use_signal(|| None::<String>);
    let mut set_value = value;

    rsx! {
        Panel {
            title: "Placement",
            note: "Opens upward when the room below is smaller than the panel and there is more above.",
            div { class: "w-56",
                Select {
                    value: value(),
                    on_value_change: move |next| set_value.set(Some(next)),
                    SelectTrigger {
                        SelectValue { placeholder: "Pick one" }
                    }
                    SelectContent {
                        for index in 0..8 {
                            SelectItem {
                                key: "item-{index}",
                                value: "item-{index}",
                                label: "Option {index + 1}",
                                "Option {index + 1}"
                            }
                        }
                    }
                }
            }
        }
    }
}
