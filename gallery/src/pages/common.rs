//! Small shared bits for the component pages.
//!
//! The beUI previews all open with the same header and the same section
//! heading, so the gallery uses one component for each rather than repeating
//! the markup on every page.

use beui::prelude::*;

/// The page header: eyebrow, title, lede.
#[component]
pub fn PageHeader(category: String, title: String, lede: String) -> Element {
    rsx! {
        header { class: "flex flex-col gap-3",
            p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                "{category}"
            }
            h1 { class: "text-balance text-3xl font-semibold tracking-tight", "{title}" }
            p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground", "{lede}" }
        }
    }
}

/// One labelled block of previews.
#[component]
pub fn Section(title: String, note: String, children: Element) -> Element {
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

/// The bordered stage the previews centre their component in.
#[component]
pub fn Stage(#[props(default)] class: Option<String>, children: Element) -> Element {
    rsx! {
        div {
            class: cn!(
                "flex flex-wrap items-center justify-center gap-3 rounded-xl border border-border bg-card p-8",
                class,
            ),
            {children}
        }
    }
}

/// The page body wrapper.
#[component]
pub fn PageBody(children: Element) -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16", {children} }
    }
}
