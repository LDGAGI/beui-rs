//! `command-palette` — Cmd-K search over a grouped command list.
//!
//! beUI has no `command-palette.preview.tsx`; this page opens the palette from a
//! button and shows grouping, keyboard navigation, and filtering.

use beui::prelude::*;
use beui::{CommandItem, CommandPalette};

#[component]
pub fn CommandPalettePage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Command palette" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Opened via Cmd/Ctrl-K many times a day, so the entrance reads as instant: a tight spring in, an even faster exit. Grouping reorders the list, so the highlight, the ids, Enter, and the scroll all read one flattened row array."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let open = use_signal(|| false);
    let selected = use_signal(|| "—".to_string());
    let mut set_open = open;
    let mut set_selected = selected;

    let on_select =
        move |id: &'static str| EventHandler::new(move |_| set_selected.set(id.to_string()));

    let items = vec![
        CommandItem::new("home", "Go home", on_select("home"))
            .group("Navigation")
            .hint("⌘H"),
        CommandItem::new("inbox", "Go to inbox", on_select("inbox"))
            .group("Navigation")
            .keywords("mail messages"),
        CommandItem::new("new-file", "New file", on_select("new-file"))
            .group("Actions")
            .hint("⌘N"),
        CommandItem::new("search", "Search workspace", on_select("search"))
            .group("Actions")
            .keywords("find"),
        CommandItem::new("theme", "Toggle theme", on_select("theme"))
            .group("Actions")
            .keywords("dark light appearance"),
        CommandItem::new("sign-out", "Sign out", on_select("sign-out")).group("Account"),
    ];

    rsx! {
        section { class: "flex flex-col gap-4",
            h2 { class: "text-lg font-semibold tracking-tight", "Command palette" }
            p { class: "max-w-3xl text-sm text-muted-foreground",
                "Press ⌘K (or Ctrl-K), or click the button. Type to filter, arrow keys to move, Enter to run."
            }
            div { class: "flex flex-wrap items-center gap-3 py-4",
                button {
                    r#type: "button",
                    "data-command-open": "true",
                    class: "press inline-flex h-9 items-center gap-2 rounded-lg border border-border px-4 text-sm font-medium",
                    onclick: move |_| set_open.set(true),
                    "Open palette"
                    kbd { class: "rounded border border-border px-1.5 py-0.5 text-[10px] text-muted-foreground", "⌘K" }
                }
                span { class: "text-sm text-muted-foreground",
                    "selected: "
                    strong { class: "text-foreground", "{selected()}" }
                }
            }
            CommandPalette {
                items: items.clone(),
                open: Some(open()),
                on_open_change: move |next| set_open.set(next),
            }
        }
    }
}
