//! `context-menu.preview.tsx` — a folder tile with a right-click menu.

use beui::prelude::*;
use beui::{
    ContextMenu, ContextMenuCheckboxItem, ContextMenuContent, ContextMenuItem, ContextMenuLabel,
    ContextMenuSeparator, ContextMenuShortcut, ContextMenuTrigger, CopyIcon, DownloadIcon, EyeIcon,
    PencilIcon, Trash2Icon,
};

#[component]
pub fn ContextMenuPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Context menu" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The menu opens at the invocation point, clamped inside the viewport, and unfolds out of that point: the clip morphs from a tiny inset around the cursor to the full rounded rect. Right-click, press Shift+F10, or hold on touch."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let message = use_signal(|| None::<&'static str>);
    let offline = use_signal(|| false);
    let mut set_message = message;
    let mut set_offline = offline;

    rsx! {
        section { class: "flex min-h-[360px] w-full items-center justify-center",
            ContextMenu {
                ContextMenuTrigger {
                    button {
                        r#type: "button",
                        class: "group flex flex-col items-center outline-none",
                        div { class: "relative h-24 w-32 transition-transform duration-150 group-active:scale-[0.98] group-focus-visible:rounded-2xl group-focus-visible:ring-2 group-focus-visible:ring-foreground/20 group-focus-visible:ring-offset-4 group-focus-visible:ring-offset-background",
                            div { class: "absolute left-1 top-1 h-7 w-14 rounded-t-[10px] bg-[#d4a84f] dark:bg-[#a77d2f]" }
                            div { class: "absolute inset-x-0 bottom-0 top-5 rounded-[14px] bg-[#e7bb61] shadow-[0_14px_24px_-16px_rgba(90,58,8,0.75)] dark:bg-[#bd8d36]" }
                            div { class: "absolute inset-x-0 bottom-0 top-9 rounded-[14px] bg-[#efc86f] dark:bg-[#cb9a41]" }
                            div { class: "absolute inset-x-5 bottom-4 h-px bg-black/10 dark:bg-white/10" }
                        }
                        span { class: "mt-4 text-sm font-medium text-foreground", "Right click on me" }
                        div { class: "mt-1 h-4",
                            if let Some(text) = message() {
                                span { class: "text-[10px] text-muted-foreground", "{text}" }
                            } else {
                                span { class: "text-[10px] text-muted-foreground", "or long-press · Shift + F10" }
                            }
                        }
                    }
                }
                ContextMenuContent { aria_label: "Folder actions", class: "w-60",
                    ContextMenuLabel { "Project files" }
                    ContextMenuItem { text_value: "Open", on_select: move |_| set_message.set(Some("Folder opened")),
                        EyeIcon { class: "h-4 w-4" }
                        "Open"
                        ContextMenuShortcut { "↵" }
                    }
                    ContextMenuItem { text_value: "Rename", on_select: move |_| set_message.set(Some("Ready to rename")),
                        PencilIcon { class: "h-4 w-4" }
                        "Rename"
                        ContextMenuShortcut { "R" }
                    }
                    ContextMenuItem { text_value: "Duplicate", on_select: move |_| set_message.set(Some("Folder duplicated")),
                        CopyIcon { class: "h-4 w-4" }
                        "Duplicate"
                        ContextMenuShortcut { "⌘D" }
                    }
                    ContextMenuItem { text_value: "Download", on_select: move |_| set_message.set(Some("Download started")),
                        DownloadIcon { class: "h-4 w-4" }
                        "Download"
                    }
                    ContextMenuSeparator {}
                    ContextMenuCheckboxItem {
                        text_value: "Keep offline",
                        checked: offline(),
                        close_on_select: false,
                        on_checked_change: move |checked| {
                            set_offline.set(checked);
                            set_message.set(Some(if checked { "Available offline" } else { "Online only" }));
                        },
                        "Keep offline"
                    }
                    ContextMenuSeparator {}
                    ContextMenuItem { text_value: "Move to trash", destructive: true, on_select: move |_| set_message.set(Some("Moved to trash")),
                        Trash2Icon { class: "h-4 w-4" }
                        "Move to trash"
                        ContextMenuShortcut { "⌘⌫" }
                    }
                }
            }
        }
    }
}
