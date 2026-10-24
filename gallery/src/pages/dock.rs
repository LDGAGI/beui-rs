//! `dock.preview.tsx` — one travelling pill behind six icons.

use beui::prelude::*;
use beui::{
    CalendarIcon, Dock, DockItem, DockSeparator, GithubIcon, HomeIcon, MailIcon, MusicIcon,
    SettingsIcon, SparklesIcon,
};

#[component]
pub fn DockPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Dock" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The active pill is a shared layout shape: it lives inside whichever item is active, so switching items hands it the box the old one was last measured at."
                }
            }
            Preview {}
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    Home,
    Mail,
    Calendar,
    Music,
    Discover,
    Settings,
}

impl Item {
    const ALL: [Item; 5] = [
        Item::Home,
        Item::Mail,
        Item::Calendar,
        Item::Music,
        Item::Discover,
    ];

    const fn label(self) -> &'static str {
        match self {
            Item::Home => "Home",
            Item::Mail => "Mail",
            Item::Calendar => "Calendar",
            Item::Music => "Music",
            Item::Discover => "Discover",
            Item::Settings => "Settings",
        }
    }
}

fn item_icon(item: Item) -> Element {
    match item {
        Item::Home => rsx! { HomeIcon { class: "h-5 w-5" } },
        Item::Mail => rsx! { MailIcon { class: "h-5 w-5" } },
        Item::Calendar => rsx! { CalendarIcon { class: "h-5 w-5" } },
        Item::Music => rsx! { MusicIcon { class: "h-5 w-5" } },
        Item::Discover => rsx! { SparklesIcon { class: "h-5 w-5" } },
        Item::Settings => rsx! { SettingsIcon { class: "h-5 w-5" } },
    }
}

#[component]
fn Preview() -> Element {
    let active = use_signal(|| Item::Home);
    let mut set_active = active;

    rsx! {
        div { class: "flex w-full justify-center",
            Dock {
                for item in Item::ALL {
                    DockItem {
                        key: "{item.label()}",
                        aria_label: item.label(),
                        active: active() == item,
                        onclick: move |_| set_active.set(item),
                        {item_icon(item)}
                    }
                }
                DockSeparator {}
                DockItem {
                    aria_label: "Settings",
                    active: active() == Item::Settings,
                    onclick: move |_| set_active.set(Item::Settings),
                    SettingsIcon { class: "h-5 w-5" }
                }
                DockItem { aria_label: "GitHub",
                    GithubIcon { class: "h-5 w-5" }
                }
            }
        }
    }
}
