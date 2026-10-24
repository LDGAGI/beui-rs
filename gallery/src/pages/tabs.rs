//! `tabs.preview.tsx` — the four shapes, including the overflowing list.

use beui::prelude::*;
use beui::{Tabs, TabsContent, TabsList, TabsTrigger, TabsVariant};

#[component]
pub fn TabsPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Tabs" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "One indicator, handed between tabs. It lives inside the active trigger, so switching tabs is an unmount in one button and a mount in another — the layout box is already the new tab's, only the transform is behind. The active label hands its colour over at the indicator's edge."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Section(title: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "flex flex-col gap-2",
            span { class: "text-[10px] font-semibold uppercase tracking-wider text-muted-foreground",
                "{title}"
            }
            {children}
        }
    }
}

#[component]
fn Preview() -> Element {
    const OVERFLOW: [&str; 6] = [
        "Overview",
        "Activity",
        "Analytics",
        "Members",
        "Billing",
        "Settings",
    ];

    rsx! {
        div { class: "flex w-full max-w-md flex-col gap-8",
            Section { title: "Pill",
                Tabs { default_value: "overview", variant: TabsVariant::Pill,
                    TabsList {
                        TabsTrigger { value: "overview", "Overview" }
                        TabsTrigger { value: "activity", "Activity" }
                        TabsTrigger { value: "settings", "Settings" }
                    }
                    TabsContent { value: "overview", class: "text-sm text-muted-foreground", "High-level summary." }
                    TabsContent { value: "activity", class: "text-sm text-muted-foreground", "Recent events." }
                    TabsContent { value: "settings", class: "text-sm text-muted-foreground", "Preferences." }
                }
            }
            Section { title: "Overflow",
                Tabs { default_value: "Overview", class: "w-full max-w-xs",
                    TabsList {
                        for label in OVERFLOW {
                            TabsTrigger { key: "{label}", value: label, "{label}" }
                        }
                    }
                }
            }
            Section { title: "Segment",
                Tabs { default_value: "day", variant: TabsVariant::Segment,
                    TabsList {
                        TabsTrigger { value: "day", "Day" }
                        TabsTrigger { value: "week", "Week" }
                        TabsTrigger { value: "month", "Month" }
                    }
                }
            }
            Section { title: "Underline",
                Tabs { default_value: "all", variant: TabsVariant::Underline,
                    TabsList {
                        TabsTrigger { value: "all", "All" }
                        TabsTrigger { value: "open", "Open" }
                        TabsTrigger { value: "closed", "Closed" }
                    }
                }
            }
        }
    }
}
