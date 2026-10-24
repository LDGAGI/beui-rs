//! `animated-sidebar` — the beUI preview, one-for-one.

use beui::icons::{
    Building2Icon, ChevronRightIcon, ChevronsUpDownIcon, CircleUserRoundIcon, CommandIcon,
    InboxIcon, LayoutGridIcon, ListTodoIcon, NotebookTabsIcon, PanelLeftIcon, SearchIcon,
    SparklesIcon, TargetIcon, WorkflowIcon, XIcon,
};
use beui::prelude::*;
use beui::{
    AnimatedSidebar, AnimatedSidebarClose, AnimatedSidebarContent, AnimatedSidebarFooter,
    AnimatedSidebarGroup, AnimatedSidebarGroupContent, AnimatedSidebarGroupLabel,
    AnimatedSidebarHeader, AnimatedSidebarInset, AnimatedSidebarMenu, AnimatedSidebarMenuButton,
    AnimatedSidebarMenuItem, AnimatedSidebarMenuSub, AnimatedSidebarMenuSubButton,
    AnimatedSidebarMenuSubItem, AnimatedSidebarProvider, AnimatedSidebarRail,
    AnimatedSidebarTrigger,
};

/// `destinations` from the preview.
const DESTINATIONS: [(&str, &[&str]); 7] = [
    ("People", &["All people", "Recent activity", "Segments"]),
    ("Companies", &[]),
    ("Opportunities", &["Pipeline", "Forecast", "Closed deals"]),
    ("Tasks", &[]),
    ("Notes", &[]),
    ("Workflows", &["Automations", "Runs", "Templates"]),
    ("Dashboard", &[]),
];

fn destination_icon(label: &str) -> Element {
    match label {
        "People" => rsx! { CircleUserRoundIcon { class: "size-4" } },
        "Companies" => rsx! { Building2Icon { class: "size-4" } },
        "Opportunities" => rsx! { TargetIcon { class: "size-4" } },
        "Tasks" => rsx! { ListTodoIcon { class: "size-4" } },
        "Notes" => rsx! { NotebookTabsIcon { class: "size-4" } },
        "Workflows" => rsx! { WorkflowIcon { class: "size-4" } },
        _ => rsx! { LayoutGridIcon { class: "size-4" } },
    }
}

#[component]
pub fn AnimatedSidebarPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-6xl flex-col gap-10 px-6 py-12",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                    "Animated sidebar"
                }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The rail folds down to an icon rail; ⌘/Ctrl+B or the rail toggles it. Below 768px the same panel becomes a modal sheet."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let mut active = use_signal(|| "People".to_string());
    let mut open_section = use_signal(|| None::<String>);
    let active_now = active();

    rsx! {
        div { class: "w-full px-0 py-2 sm:p-3",
            AnimatedSidebarProvider { class: "h-[720px] min-h-0 overflow-hidden rounded-2xl border border-foreground/[0.08] bg-background",
                AnimatedSidebar {
                    aria_label: "Solace workspace",
                    collapsible: beui::AnimatedSidebarCollapsible::Icon,
                    class: "min-h-0",
                    panel_class: "h-full border-foreground/[0.08]",
                    AnimatedSidebarHeader { class: "p-3 pb-2",
                        div { class: "flex min-h-11 items-center gap-3 overflow-hidden px-2",
                            div { class: "grid size-7 shrink-0 place-items-center rounded-lg bg-foreground text-background",
                                CommandIcon { class: "size-4" }
                            }
                            button {
                                r#type: "button",
                                class: "flex min-w-0 flex-1 items-center gap-2 rounded-lg text-left outline-none focus-visible:ring-2 focus-visible:ring-ring group-data-[state=collapsed]/sidebar:hidden",
                                span { class: "truncate text-sm font-semibold text-foreground", "Acme Inc" }
                                ChevronsUpDownIcon { class: "size-3.5 shrink-0 text-muted-foreground" }
                            }
                            AnimatedSidebarClose { class: "ml-auto text-muted-foreground hover:bg-muted md:hidden",
                                XIcon { class: "size-4" }
                            }
                        }
                    }
                    AnimatedSidebarContent { class: "px-2 pt-1",
                        AnimatedSidebarGroup { class: "pb-2",
                            AnimatedSidebarGroupContent {
                                AnimatedSidebarMenu {
                                    AnimatedSidebarMenuItem {
                                        AnimatedSidebarMenuButton {
                                            label: "Search",
                                            icon: rsx! { SearchIcon { class: "size-4" } },
                                            is_active: active_now == "Search",
                                            on_select: move |_| {
                                                active.set("Search".to_string());
                                                open_section.set(None);
                                            },
                                            "Search"
                                        }
                                    }
                                    AnimatedSidebarMenuItem {
                                        AnimatedSidebarMenuButton {
                                            label: "AI Assistant",
                                            icon: rsx! { SparklesIcon { class: "size-4" } },
                                            is_active: active_now == "AI Assistant",
                                            on_select: move |_| {
                                                active.set("AI Assistant".to_string());
                                                open_section.set(None);
                                            },
                                            "AI Assistant"
                                        }
                                    }
                                    AnimatedSidebarMenuItem {
                                        AnimatedSidebarMenuButton {
                                            label: "Inbox",
                                            icon: rsx! { InboxIcon { class: "size-4" } },
                                            badge: rsx! { "4" },
                                            is_active: active_now == "Inbox",
                                            on_select: move |_| {
                                                active.set("Inbox".to_string());
                                                open_section.set(None);
                                            },
                                            "Inbox"
                                        }
                                    }
                                }
                            }
                        }
                        AnimatedSidebarGroup { class: "pt-1",
                            AnimatedSidebarGroupLabel { "Workspaces" }
                            AnimatedSidebarGroupContent {
                                AnimatedSidebarMenu {
                                    for (label, children) in DESTINATIONS {
                                        {
                                            let label = label.to_string();
                                            let has_children = !children.is_empty();
                                            let label_for_active = label.clone();
                                            let label_for_expanded = label.clone();
                                            let label_for_select = label.clone();
                                            let child_labels: Vec<String> =
                                                children.iter().map(|child| (*child).to_string()).collect();
                                            let submenu_open = has_children
                                                && open_section() == Some(label.clone())
                                                && active_now != label;
                                            rsx! {
                                                AnimatedSidebarMenuItem {
                                                    AnimatedSidebarMenuButton {
                                                        label: label_for_active.clone(),
                                                        icon: destination_icon(&label_for_active),
                                                        is_active: active_now == label_for_active
                                                            || child_labels.contains(&active_now),
                                                        aria_expanded: if has_children {
                                                            Some(open_section() == Some(label_for_expanded.clone()))
                                                        } else {
                                                            None
                                                        },
                                                        on_select: move |_| {
                                                            if has_children {
                                                                let next = if open_section() == Some(label_for_select.clone()) {
                                                                    None
                                                                } else {
                                                                    Some(label_for_select.clone())
                                                                };
                                                                open_section.set(next);
                                                            } else {
                                                                active.set(label_for_select.clone());
                                                                open_section.set(None);
                                                            }
                                                        },
                                                        "{label}"
                                                    }
                                                    if has_children {
                                                        AnimatedSidebarMenuSub { open: submenu_open,
                                                            for child in child_labels.iter().cloned() {
                                                                {
                                                                    let child_for_active = child.clone();
                                                                    let child_for_select = child.clone();
                                                                    rsx! {
                                                                        AnimatedSidebarMenuSubItem {
                                                                            AnimatedSidebarMenuSubButton {
                                                                                is_active: active_now == child_for_active,
                                                                                on_select: move |_| {
                                                                                    active.set(child_for_select.clone());
                                                                                },
                                                                                "{child}"
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
                        }
                    }
                    AnimatedSidebarFooter { class: "gap-3 border-none p-3",
                        button {
                            r#type: "button",
                            class: "flex min-h-11 w-full items-center gap-3 overflow-hidden rounded-xl p-1 text-left outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring",
                            span { class: "grid size-9 shrink-0 place-items-center rounded-full bg-[#d5ff66] text-xs font-semibold text-[#172000]",
                                "AS"
                            }
                            span { class: "min-w-0 flex-1 group-data-[state=collapsed]/sidebar:hidden",
                                span { class: "block truncate text-sm font-medium text-foreground",
                                    "Ava Stone"
                                }
                                span { class: "block truncate text-xs text-muted-foreground",
                                    "ava@acme.com"
                                }
                            }
                            ChevronRightIcon { class: "size-4 shrink-0 text-muted-foreground group-data-[state=collapsed]/sidebar:hidden" }
                        }
                    }
                    AnimatedSidebarRail {}
                }
                AnimatedSidebarInset { class: "min-h-0 bg-background",
                    header { class: "flex h-16 shrink-0 items-center gap-3 border-border border-b px-4",
                        AnimatedSidebarTrigger { class: "text-muted-foreground transition-colors hover:bg-muted hover:text-foreground",
                            PanelLeftIcon { class: "size-4" }
                        }
                        div { class: "h-5 w-px bg-border" }
                        p { class: "text-sm font-medium text-foreground", "{active}" }
                    }
                    div { class: "flex min-h-0 flex-1 flex-col justify-between overflow-hidden p-5 sm:p-7",
                        div {
                            p { class: "text-xs font-medium text-muted-foreground",
                                "Wednesday, July 29"
                            }
                            h3 { class: "mt-2 max-w-md text-xl font-semibold tracking-tight text-foreground sm:text-2xl",
                                "Good morning, Ava."
                            }
                            p { class: "mt-2 max-w-md text-sm leading-6 text-muted-foreground",
                                "Your workspace stays in place while the navigation folds down to a focused icon rail."
                            }
                        }
                        div { class: "flex items-end justify-between border-border border-t pt-4",
                            div {
                                p { class: "text-[10px] uppercase tracking-[0.16em] text-muted-foreground",
                                    "Active view"
                                }
                                p { class: "mt-1 text-sm font-medium text-foreground", "{active}" }
                            }
                            p { class: "hidden text-xs text-muted-foreground sm:block",
                                "Press ⌘B to fold the rail"
                            }
                        }
                    }
                }
            }
        }
    }
}
