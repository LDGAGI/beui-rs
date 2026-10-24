//! `agents/citations.tsx` — inline citation markers, the source stack/list and
//! the `Citations` disclosure.
//!
//! The source resolves a favicon with `useFavicon` (which drops the image once
//! `decode()` rejects, because an SSR-started image can fail before a handler
//! attaches). Here the image is client-rendered, so a plain `onerror` is enough;
//! [`CitationFavicon`] swaps to the globe glyph for a failed or missing favicon.

use beui::prelude::*;

use crate::common::{use_unique_id, Icon};
use crate::disclosure::AgentDisclosure;
use crate::icons::IconName;

/// `CitationItem`.
#[derive(Clone, Debug, PartialEq)]
pub struct CitationItem {
    pub id: String,
    pub title: Element,
    pub domain: Option<Element>,
    pub url: Option<String>,
    /// Drawn instead of the URL favicon, e.g. a document icon for non-web sources.
    pub icon: Option<Element>,
}

impl CitationItem {
    /// A source with just a title (and optional URL), the common case.
    pub fn new(id: impl Into<String>, title: Element) -> Self {
        Self {
            id: id.into(),
            title,
            domain: None,
            url: None,
            icon: None,
        }
    }

    pub fn domain(mut self, domain: Element) -> Self {
        self.domain = Some(domain);
        self
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

/// `new URL("/favicon.ico", value).toString()`, `None` when `value` is not a URL.
fn favicon_url(value: &str) -> Option<String> {
    let scheme_end = value.find("://")?;
    let after = &value[scheme_end + 3..];
    let authority_end = after.find(['/', '?', '#']).unwrap_or(after.len());
    let authority = &after[..authority_end];
    if authority.is_empty() {
        return None;
    }
    Some(format!(
        "{}://{}/favicon.ico",
        &value[..scheme_end],
        authority
    ))
}

/// `citationTargetId(prefix, citationId)` — keep only `[a-zA-Z0-9_-]`.
fn citation_target_id(prefix: &str, citation_id: &str) -> String {
    let safe: String = citation_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("{prefix}-{safe}")
}

/// `Citation` — the raised numeric marker inside prose.
#[component]
pub fn Citation(
    citation_id: String,
    index: usize,
    id_prefix: String,
    #[props(default)] label: Option<String>,
    #[props(default)] class: String,
) -> Element {
    let href = format!("#{}", citation_target_id(&id_prefix, &citation_id));
    rsx! {
        a {
            href: "{href}",
            "aria-label": label.unwrap_or_else(|| format!("View citation {index}")),
            class: cn!(
                "mx-0.5 inline-flex min-w-4 -translate-y-0.5 items-center justify-center rounded-md bg-muted/60 px-1 py-0.5 text-[10px] font-semibold leading-none text-muted-foreground no-underline outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            "{index}"
        }
    }
}

/// `CitationFavicon`.
#[component]
pub fn CitationFavicon(
    #[props(default)] url: Option<String>,
    #[props(default)] icon: Option<Element>,
    #[props(default)] class: String,
) -> Element {
    let mut failed = use_signal(|| false);
    let src = if icon.is_none() {
        url.as_deref().and_then(favicon_url)
    } else {
        None
    };
    let show_image = src.is_some() && !failed();
    let src_value = src.clone().unwrap_or_default();

    rsx! {
        span {
            "aria-hidden": "true",
            class: cn!(
                "grid size-5 shrink-0 place-items-center text-muted-foreground [&>svg]:size-3.5",
                class,
            ),
            if let Some(icon) = icon {
                {icon}
            } else if show_image {
                img {
                    src: "{src_value}",
                    alt: "",
                    width: 16,
                    height: 16,
                    "referrerpolicy": "no-referrer",
                    class: "size-4 rounded-sm object-contain",
                    onerror: move |_| failed.set(true),
                }
            } else {
                Icon { name: IconName::Globe2, class: "size-3.5" }
            }
        }
    }
}

/// `CitationStack` — the overlapping favicon row used as a disclosure trigger.
#[component]
pub fn CitationStack(
    citations: Vec<CitationItem>,
    #[props(default = 3)] limit: usize,
    #[props(default)] class: String,
) -> Element {
    rsx! {
        span {
            "aria-hidden": "true",
            class: cn!("flex -space-x-1.5", class),
            for citation in citations.into_iter().take(limit) {
                CitationFavicon {
                    key: "{citation.id}",
                    url: citation.url.clone(),
                    icon: citation.icon.clone(),
                    class: "size-6 rounded-full bg-background ring-2 ring-background",
                }
            }
        }
    }
}

fn citation_row(citation: CitationItem, index: usize, id_prefix: &str) -> Element {
    let id = citation_target_id(id_prefix, &citation.id);
    let class = "group/citation flex items-center gap-2 rounded-md px-1.5 py-1 outline-none focus-visible:ring-2 focus-visible:ring-ring";
    let content = rsx! {
        CitationFavicon { url: citation.url.clone(), icon: citation.icon.clone() }
        span { class: "flex min-w-0 flex-1 flex-wrap items-baseline gap-x-2 gap-y-0.5",
            span { class: "truncate text-sm font-medium text-foreground/80 transition-colors group-hover/citation:text-foreground",
                {citation.title.clone()}
            }
            if let Some(domain) = citation.domain.clone() {
                span { class: "min-w-0 truncate text-xs text-muted-foreground/60", {domain} }
            }
        }
        span { class: "flex shrink-0 items-center gap-1.5",
            span { class: "grid size-5 place-items-center rounded-md bg-foreground/[0.05] text-[10px] font-semibold tabular-nums text-muted-foreground",
                "{index}"
            }
            if citation.url.is_some() {
                Icon {
                    name: IconName::ExternalLink,
                    class: "size-3.5 text-muted-foreground/40 transition-colors group-hover/citation:text-muted-foreground",
                }
            }
        }
    };

    match citation.url {
        Some(url) => rsx! {
            a {
                id: "{id}",
                href: "{url}",
                target: "_blank",
                rel: "noreferrer noopener",
                class: "{class}",
                {content}
            }
        },
        None => rsx! {
            div { id: "{id}", class: "{class}", {content} }
        },
    }
}

/// `CitationList`.
#[component]
pub fn CitationList(
    citations: Vec<CitationItem>,
    #[props(default)] id_prefix: Option<String>,
    #[props(default)] class: String,
) -> Element {
    let base = use_unique_id("citation-list");
    let prefix = id_prefix.unwrap_or(base);
    rsx! {
        div { class: cn!("grid gap-0.5", class),
            for (index, citation) in citations.into_iter().enumerate() {
                {citation_row(citation, index + 1, &prefix)}
            }
        }
    }
}

/// `Citations` — the `Sources` disclosure.
#[component]
pub fn Citations(
    citations: Vec<CitationItem>,
    #[props(default = "Sources".to_string())] title: String,
    #[props(default)] open: Option<bool>,
    #[props(default)] default_open: bool,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(default)] id_prefix: Option<String>,
    #[props(default)] class: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let base = use_unique_id("citation");
    let content_id = format!("{base}-content");
    let prefix = id_prefix.unwrap_or(base);
    let mut internal_open = use_signal(|| default_open);
    let current_open = open.unwrap_or(internal_open());

    let mut set_open = move |next: bool| {
        if open.is_none() {
            internal_open.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    };

    let count = citations.len();
    let rotate = if current_open { 180 } else { 0 };
    let duration = if reduce { 0 } else { 200 };
    rsx! {
        div { class: cn!("w-full text-sm", class),
            button {
                r#type: "button",
                "aria-expanded": if current_open { "true" } else { "false" },
                "aria-controls": "{content_id}",
                onclick: move |_| set_open(!current_open),
                class: "group -ml-1 flex min-h-8 items-center gap-2 rounded-lg px-1 text-left text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                Icon { name: IconName::BookOpenText, class: "size-4" }
                span { class: "font-medium", "{title}" }
                span { class: "rounded-full bg-muted px-1.5 py-0.5 text-[10px] font-semibold tabular-nums",
                    "{count}"
                }
                span {
                    "aria-hidden": "true",
                    class: "text-muted-foreground/60",
                    style: "transform: rotate({rotate}deg); transition: transform {duration}ms cubic-bezier(0.34, 1.56, 0.64, 1);",
                    Icon { name: IconName::ChevronDown, class: "size-3.5" }
                }
            }

            AgentDisclosure { id: content_id.clone(), open: current_open,
                CitationList { citations, id_prefix: Some(prefix), class: "mt-1" }
            }
        }
    }
}
