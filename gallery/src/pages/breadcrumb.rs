//! `breadcrumb.preview.tsx` — a path that folds its middle away as depth grows.

use beui::prelude::*;
use beui::{
    ArrowUpRightIcon, Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbPage,
    BreadcrumbSeparator, FolderIcon, HomeIcon,
};

const PATH: [&str; 7] = [
    "Workspace",
    "Projects",
    "Website",
    "Design system",
    "Components",
    "Navigation",
    "Breadcrumb",
];

#[component]
pub fn BreadcrumbDemo() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Breadcrumb" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A long path folds its middle into an ellipsis; the items are popLayout presence children, so a route change animates the leaving item out while the siblings move at once."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let depth = use_signal(|| 5usize);
    let mut set_depth = depth;

    rsx! {
        div { class: "w-full max-w-lg space-y-8 px-4",
            Breadcrumb { class: "min-h-[4.25rem] sm:min-h-8",
                BreadcrumbList { max_items: 3,
                    for (index, label) in PATH.iter().enumerate().take(depth() + 1) {
                        BreadcrumbItem { key: "{label}",
                            if index > 0 {
                                BreadcrumbSeparator {}
                            }
                            if index == depth() {
                                BreadcrumbPage { tabindex: -1,
                                    if index == 0 {
                                        HomeIcon { class: "size-3.5" }
                                    }
                                    "{label}"
                                }
                            } else {
                                BreadcrumbLink {
                                    href: "#{label.to_lowercase()}",
                                    onclick: move |event: MouseEvent| {
                                        event.prevent_default();
                                        set_depth.set(index);
                                    },
                                    if index == 0 {
                                        HomeIcon { class: "size-3.5" }
                                    }
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "min-h-20 border-t border-border/60 pt-5",
                if depth() < PATH.len() - 1 {
                    button {
                        r#type: "button",
                        class: "flex w-full items-center gap-3 rounded-lg border border-border/60 px-4 py-3 text-start text-sm transition-colors hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                        onclick: move |_| set_depth.set((depth() + 1).min(PATH.len() - 1)),
                        FolderIcon { class: "size-4 text-muted-foreground" }
                        span { class: "flex-1", "{PATH[depth() + 1]}" }
                        ArrowUpRightIcon { class: "size-3.5 text-muted-foreground" }
                    }
                } else {
                    p { class: "py-3 text-center text-sm text-muted-foreground",
                        "Choose a parent path to go back."
                    }
                }
            }
        }
    }
}
