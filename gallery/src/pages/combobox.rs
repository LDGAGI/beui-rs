//! `combobox.preview.tsx` — a searchable workspace picker.

use beui::prelude::*;
use beui::{
    Combobox, ComboboxAlign, ComboboxContent, ComboboxEmpty, ComboboxGroup, ComboboxInput,
    ComboboxItem, ComboboxLabel, ComboboxList, ComboboxSeparator, ComboboxTrigger, FolderIcon,
    HomeIcon, MailIcon, SettingsIcon,
};

const WORKSPACES: [(&str, &str, &str, &str, &str); 4] = [
    (
        "studio",
        "Design studio",
        "12 projects",
        "Recent",
        "bg-amber-400/20 text-amber-700",
    ),
    (
        "product",
        "Product team",
        "8 projects",
        "Recent",
        "bg-sky-400/20 text-sky-700",
    ),
    (
        "playground",
        "Playground",
        "24 experiments",
        "Workspaces",
        "bg-emerald-400/20 text-emerald-700",
    ),
    (
        "archive",
        "Component archive",
        "41 components",
        "Workspaces",
        "bg-rose-400/20 text-rose-700",
    ),
];

fn workspace_icon(value: &str) -> Element {
    match value {
        "studio" => rsx! { SettingsIcon { class: "size-3.5" } },
        "product" => rsx! { HomeIcon { class: "size-3.5" } },
        "playground" => rsx! { FolderIcon { class: "size-3.5" } },
        _ => rsx! { MailIcon { class: "size-3.5" } },
    }
}

fn workspace_color(value: &str) -> &'static str {
    WORKSPACES
        .iter()
        .find(|(id, ..)| *id == value)
        .map(|(_, _, _, _, color)| *color)
        .unwrap_or("bg-muted text-foreground")
}

#[component]
pub fn ComboboxPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Combobox" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A searchable listbox. The panel is portalled and measured against the trigger, its height, opacity and gap spring open, and it flips sides when there is more room above. The active option is resolved during render, never in an effect."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let value = use_signal(|| Some("studio".to_string()));
    let mut set_value = value;

    rsx! {
        section { class: "w-full max-w-72",
            p { class: "mb-2 text-xs font-medium text-muted-foreground", "Workspace" }
            Combobox {
                value: value(),
                on_value_change: move |next| set_value.set(Some(next)),
                ComboboxTrigger { class: "h-12 rounded-2xl px-2.5",
                    ComboboxInput { aria_label: "Search workspaces", placeholder: "Search workspaces…" }
                }
                ComboboxContent { align: ComboboxAlign::Start, class: "w-72 rounded-2xl",
                    ComboboxList { aria_label: "Workspaces", class: "p-2",
                        ComboboxEmpty { message: "No workspaces found." }
                        for (group_index, group) in ["Recent", "Workspaces"].iter().enumerate() {
                            ComboboxGroup { key: "{group}",
                                if group_index > 0 {
                                    ComboboxSeparator {}
                                }
                                ComboboxLabel { "{group}" }
                                for (id, label, detail, item_group, _color) in WORKSPACES {
                                    if item_group == *group {
                                        ComboboxItem {
                                            key: "{id}",
                                            value: id,
                                            text_value: label,
                                            keywords: vec![detail.to_string(), item_group.to_string()],
                                            class: "py-2",
                                            span { class: "flex min-w-0 items-center gap-2.5",
                                                span {
                                                    class: cn!("grid size-7 shrink-0 place-items-center rounded-lg", workspace_color(id)),
                                                    {workspace_icon(id)}
                                                }
                                                span { class: "min-w-0",
                                                    span { class: "block truncate font-medium text-foreground", "{label}" }
                                                    span { class: "block truncate text-xs text-muted-foreground", "{detail}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
