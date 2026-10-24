//! `multi-select.preview.tsx` — pick a set of teams.

use beui::prelude::*;
use beui::{
    CircleIcon, MultiSelect, MultiSelectContent, MultiSelectEmpty, MultiSelectGroup,
    MultiSelectInput, MultiSelectItem, MultiSelectLabel, MultiSelectList, MultiSelectTrigger,
    MultiSelectValue,
};

const PRODUCT_TEAMS: [(&str, &str, &str); 4] = [
    ("design", "Design", "fill-rose-500 text-rose-500"),
    ("engineering", "Engineering", "fill-sky-500 text-sky-500"),
    ("product", "Product", "fill-amber-500 text-amber-500"),
    ("research", "Research", "fill-violet-500 text-violet-500"),
];

const BUSINESS_TEAMS: [(&str, &str, &str); 2] = [
    (
        "marketing",
        "Marketing",
        "fill-emerald-500 text-emerald-500",
    ),
    ("operations", "Operations", "fill-slate-500 text-slate-500"),
];

#[component]
pub fn MultiSelectPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Multi select" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The same searchable panel as the combobox, but the value is a set: selecting toggles, the trigger shows removable chips, and Backspace on an empty query removes the last one."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    rsx! {
        section { class: "flex min-h-[420px] w-full items-start justify-center px-4 pt-24",
            div { class: "w-full max-w-sm",
                MultiSelect { default_value: vec!["design".to_string(), "engineering".to_string()],
                    MultiSelectTrigger {
                        MultiSelectValue { placeholder: "Choose teams" }
                        MultiSelectInput { aria_label: "Search teams" }
                    }
                    MultiSelectContent {
                        MultiSelectList { aria_label: "Teams",
                            MultiSelectGroup {
                                MultiSelectLabel { "Product teams" }
                                for (value, label, color) in PRODUCT_TEAMS {
                                    MultiSelectItem { key: "{value}", value, text_value: label,
                                        span { class: "flex items-center gap-2.5",
                                            CircleIcon { class: cn!("size-2.5", color) }
                                            "{label}"
                                        }
                                    }
                                }
                            }
                            MultiSelectGroup {
                                MultiSelectLabel { "Business teams" }
                                for (value, label, color) in BUSINESS_TEAMS {
                                    MultiSelectItem { key: "{value}", value, text_value: label,
                                        span { class: "flex items-center gap-2.5",
                                            CircleIcon { class: cn!("size-2.5", color) }
                                            "{label}"
                                        }
                                    }
                                }
                            }
                            MultiSelectEmpty { message: "No teams found." }
                        }
                    }
                }
            }
        }
    }
}
