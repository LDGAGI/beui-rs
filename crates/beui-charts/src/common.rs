//! Shared helpers for the charts components.
//!
//! The charts draw the same Lucide glyphs the agents do; the path table is
//! duplicated from `beui-agents` (the two crates are siblings and neither should
//! depend on the other — see the module notes). [`Icon`] renders them in the
//! shared `<svg>` shell.

use beui_core::platform;
use dioxus::prelude::*;

use crate::icons::IconName;

/// Draw a Lucide icon: the shared `<svg>` shell plus the glyph's path data.
#[component]
pub fn Icon(
    name: IconName,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let inner = name.inner();
    rsx! {
        svg {
            class: "{class}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            dangerous_inner_html: "{inner}",
            ..attributes,
        }
    }
}

/// A stable DOM id for this component instance.
pub fn use_unique_id(prefix: &str) -> String {
    use_hook(|| format!("{prefix}-{}", platform::next_scope_id()))
}
