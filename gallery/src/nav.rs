//! The gallery's left navigation.
//!
//! The item list is derived at runtime from the two lists that already exist —
//! [`crate::PAGES`] (every page that exists, which is the truth about what is
//! navigable) and the source registry (names and categories) — so the nav
//! cannot offer a page that does not exist. The old registry-driven rail could:
//! 37 of its 107 entries had no page at all, and 43 existing pages were missing
//! from it.
//!
//! Deviation from the gallery's old chrome: there is no site header, no right
//! rail and no second nav bar. This sidebar is the only navigation, so the
//! header's spaces, its search palette, its mobile sheet, its theme control and
//! the `Customize` drawer are gone with it.
//!
//! Groups are always expanded, the way shadcn's docs sidebar is: with ~115
//! entries the filter box is the way to find one, not collapsing.

use crate::registry::REGISTRY;
use crate::PAGES;
use beui::prelude::*;
use std::collections::HashMap;

/// The group for pages the source registry does not describe: the gallery's own
/// observation pages (`foundation`, `hooks`, `presence`, …), behaviour demos and
/// variant/state pages (`table-editable`, `not-found-*`, `text-*`, …).
const GALLERY_GROUP: &str = "Gallery";

pub struct NavEntry {
    pub slug: &'static str,
    pub label: String,
}

pub struct NavGroup {
    pub title: &'static str,
    pub entries: Vec<NavEntry>,
}

/// Every navigable page, in the registry's category order with the gallery's
/// own pages last. Rebuilt per render: ~115 string allocations, which is far
/// cheaper than the memo machinery it would take to cache.
pub fn groups() -> Vec<NavGroup> {
    let mut named: HashMap<&'static str, (&'static str, &'static str)> = HashMap::new();
    for category in REGISTRY {
        for component in category.components {
            named.insert(component.slug, (category.name, component.name));
        }
    }

    let mut grouped: Vec<NavGroup> = REGISTRY
        .iter()
        .map(|category| NavGroup {
            title: category.name,
            entries: Vec::new(),
        })
        .collect();
    let mut gallery = NavGroup {
        title: GALLERY_GROUP,
        entries: Vec::new(),
    };

    for (slug, label) in PAGES.iter().copied() {
        match named.get(slug) {
            Some((category, name)) => {
                if let Some(group) = grouped.iter_mut().find(|group| group.title == *category) {
                    group.entries.push(NavEntry {
                        slug,
                        label: (*name).to_string(),
                    });
                }
            }
            None => gallery.entries.push(NavEntry {
                slug,
                label: space_out(label),
            }),
        }
    }

    grouped.retain(|group| !group.entries.is_empty());
    if !gallery.entries.is_empty() {
        grouped.push(gallery);
    }
    grouped
}

/// `TextShimmer` → `Text Shimmer`. The gallery labels its non-registry pages in
/// Rust-ish CamelCase (`PAGES`), which reads badly in a nav.
fn space_out(label: &str) -> String {
    let mut out = String::with_capacity(label.len() + 4);
    for (index, character) in label.char_indices() {
        if character.is_uppercase() && index > 0 {
            let previous = label[..index].chars().next_back().unwrap_or(' ');
            if previous.is_lowercase() || previous.is_ascii_digit() {
                out.push(' ');
            }
        }
        out.push(character);
    }
    out
}

/// The sidebar, plus the small-viewport drawer that carries the same list.
#[component]
pub fn NavSidebar(active: String) -> Element {
    let query = use_signal(String::new);
    let mut drawer = use_signal(|| false);

    rsx! {
        // Small viewports: the sidebar is a panel behind one button.
        div { class: "sticky top-0 z-30 flex items-center gap-2 border-b border-border bg-background/95 px-4 py-2 backdrop-blur md:hidden",
            span { class: "text-sm font-semibold tracking-tight", "beUI · Dioxus" }
            button {
                r#type: "button",
                "data-nav-toggle": "true",
                "aria-expanded": if drawer() { "true" } else { "false" },
                class: "ml-auto rounded-lg border border-border px-2.5 py-1 text-xs text-muted-foreground",
                onclick: move |_| drawer.toggle(),
                if drawer() { "Close" } else { "Browse" }
            }
        }

        aside {
            "data-nav-sidebar": "true",
            class: "sticky top-0 hidden h-dvh w-64 shrink-0 flex-col border-r border-border md:flex",
            div { class: "shrink-0 px-4 pb-2 pt-4",
                div { class: "text-sm font-semibold tracking-tight", "beUI · Dioxus" }
            }
            NavPanel {
                active: active.clone(),
                query,
                on_pick: EventHandler::new(move |_: ()| drawer.set(false)),
            }
        }

        if drawer() {
            div {
                "data-nav-drawer": "true",
                class: "fixed inset-0 z-50 flex flex-col bg-background md:hidden",
                div { class: "flex shrink-0 items-center gap-2 border-b border-border px-4 py-2",
                    span { class: "text-sm font-semibold tracking-tight", "beUI · Dioxus" }
                    button {
                        r#type: "button",
                        "data-nav-close": "true",
                        class: "ml-auto rounded-lg border border-border px-2.5 py-1 text-xs text-muted-foreground",
                        onclick: move |_| drawer.set(false),
                        "Close"
                    }
                }
                NavPanel {
                    active: active.clone(),
                    query,
                    on_pick: EventHandler::new(move |_: ()| drawer.set(false)),
                }
            }
        }
    }
}

/// The filter box and the groups. Rendered twice (sidebar and drawer) against
/// one filter signal, so typing in either filters both.
#[component]
fn NavPanel(active: String, query: Signal<String>, on_pick: EventHandler<()>) -> Element {
    let navigator = navigator();
    let needle = query().trim().to_lowercase();

    let groups: Vec<NavGroup> = groups()
        .into_iter()
        .map(|group| NavGroup {
            title: group.title,
            entries: group
                .entries
                .into_iter()
                .filter(|entry| {
                    needle.is_empty()
                        || entry.label.to_lowercase().contains(&needle)
                        || entry.slug.contains(&needle)
                })
                .collect(),
        })
        .filter(|group| !group.entries.is_empty())
        .collect();
    let matched: usize = groups.iter().map(|group| group.entries.len()).sum();

    rsx! {
        div { class: "shrink-0 px-3 pb-2",
            input {
                r#type: "search",
                "data-nav-filter": "true",
                placeholder: "Filter components…",
                value: "{query}",
                class: "w-full rounded-lg border border-border bg-card/40 px-3 py-1.5 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
                oninput: move |event| query.set(event.value()),
            }
        }
        nav {
            class: "min-h-0 flex-1 overflow-y-auto px-3 pb-6",
            if matched == 0 {
                p { class: "px-1 py-4 text-sm text-muted-foreground", "Nothing matches that." }
            }
            for group in groups {
                div { key: "{group.title}", "data-nav-group": "{group.title}", class: "pt-3",
                    div { class: "flex items-baseline justify-between px-1 pb-1",
                        span { class: "text-xs font-medium uppercase tracking-wider text-muted-foreground",
                            "{group.title}"
                        }
                        span { class: "text-xs tabular-nums text-muted-foreground", "{group.entries.len()}" }
                    }
                    for entry in group.entries {
                        a {
                            key: "{entry.slug}",
                            href: "/{entry.slug}",
                            "data-page-link": "{entry.slug}",
                            "aria-current": if active == entry.slug { "page" } else { "false" },
                            class: cn!(
                                "block truncate rounded-lg px-2 py-1.5 text-sm transition-colors",
                                if active == entry.slug { "bg-primary text-primary-foreground" } else { "text-muted-foreground hover:bg-card hover:text-foreground" },
                            ),
                            onclick: move |event: Event<MouseData>| {
                                // The href is the real URL; a plain click stays in the SPA.
                                event.prevent_default();
                                navigator.push(crate::Route::Page { slug: entry.slug.to_string() });
                                on_pick.call(());
                            },
                            "{entry.label}"
                        }
                    }
                }
            }
        }
    }
}
