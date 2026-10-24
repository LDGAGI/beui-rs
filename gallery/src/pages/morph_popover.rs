//! `popover-morph.preview.tsx` — a menu that unclips out of its trigger corner.

use beui::prelude::*;
use beui::{
    ChevronDownIcon, CopyIcon, MorphPopover, MorphPopoverAlign, MorphPopoverContent,
    MorphPopoverTrigger, PencilIcon, Share2Icon, Trash2Icon,
};

#[component]
pub fn MorphPopoverPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Morph popover" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The panel is laid out at full size and clipped to the corner nearest the trigger; the clip tweens to zero while the wrapper scales on the panel spring, so it grows out of the corner it was summoned from."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let open = use_signal(|| false);
    let mut set_open = open;
    let mut close = open;

    rsx! {
        section { class: "flex flex-col gap-4",
            h2 { class: "text-lg font-semibold tracking-tight", "Morph popover" }
            p { class: "max-w-3xl text-sm text-muted-foreground",
                "Click opens from the trigger's bottom-right corner; choosing an item closes it."
            }
            div { class: "flex justify-center py-8",
                MorphPopover {
                    open: open(),
                    on_open_change: move |next| set_open.set(next),
                    MorphPopoverTrigger { class: "inline-flex h-10 items-center gap-2 rounded-xl border border-border bg-background px-4 text-sm font-medium text-foreground outline-none transition-colors hover:border-border-strong focus-visible:ring-2 focus-visible:ring-ring",
                        "Options"
                        ChevronDownIcon { class: "h-4 w-4 text-muted-foreground" }
                    }
                    MorphPopoverContent { align: MorphPopoverAlign::Start, class: "w-48 p-1.5",
                        button {
                            r#type: "button",
                            onclick: move |_| close.set(false),
                            class: "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm text-foreground outline-none transition-colors hover:bg-muted focus-visible:bg-muted",
                            PencilIcon { class: "h-4 w-4 text-muted-foreground" }
                            "Edit"
                        }
                        button {
                            r#type: "button",
                            onclick: move |_| close.set(false),
                            class: "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm text-foreground outline-none transition-colors hover:bg-muted focus-visible:bg-muted",
                            CopyIcon { class: "h-4 w-4 text-muted-foreground" }
                            "Duplicate"
                        }
                        button {
                            r#type: "button",
                            onclick: move |_| close.set(false),
                            class: "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm text-foreground outline-none transition-colors hover:bg-muted focus-visible:bg-muted",
                            Share2Icon { class: "h-4 w-4 text-muted-foreground" }
                            "Share"
                        }
                        button {
                            r#type: "button",
                            onclick: move |_| close.set(false),
                            class: "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm text-foreground outline-none transition-colors hover:bg-muted focus-visible:bg-muted",
                            Trash2Icon { class: "h-4 w-4 text-muted-foreground" }
                            "Delete"
                        }
                    }
                }
            }
        }
    }
}
