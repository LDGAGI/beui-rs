//! `agents/agent-disclosure.tsx` — the shared reveal every collapsible agent
//! surface uses.
//!
//! The source animates `opacity`, `clipPath` and `y`, and toggles `height`
//! between `0` and `openHeight` so the content leaves the layout when closed.
//! Framer-motion can animate `height: auto`; a plain CSS transition cannot, so
//! the port collapses through the grid-rows technique instead: the outer box is
//! a one-row grid whose track goes `0fr → 1fr` while `height` follows the
//! content, with the same 220 ms open / 140 ms close on `ease-out`. The visual
//! result is the source's — a clip from the top plus a 4px lift — and the extra
//! `min-height: 0` child is the grid item the track needs.
//!
//! `inert` is written only while closed and removed otherwise (see
//! [`beui_core::presence::Gate::inert_attr`] for why `inert="false"` would be
//! wrong).

use beui::prelude::*;

/// `AgentDisclosure`.
#[component]
pub fn AgentDisclosure(
    open: bool,
    #[props(default = "auto".to_string())] open_height: String,
    #[props(default)] id: Option<String>,
    #[props(default)] role: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] class: String,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let duration = if reduce {
        0
    } else if open {
        220
    } else {
        140
    };
    let rows = if open { "1fr" } else { "0fr" };
    let opacity = if open { "1" } else { "0" };
    let transform = if open {
        "translateY(0)"
    } else {
        "translateY(-4px)"
    };
    let pointer = if open { "" } else { "pointer-events:none;" };
    let height = if open && open_height != "auto" {
        format!("height:{open_height};")
    } else {
        String::new()
    };
    let transition = format!(
        "grid-template-rows {duration}ms {EASE_OUT_CSS}, opacity {duration}ms {EASE_OUT_CSS}, transform {duration}ms {EASE_OUT_CSS}"
    );

    rsx! {
        div {
            id,
            role,
            "aria-labelledby": aria_labelledby,
            "aria-hidden": if open { None } else { Some("true") },
            inert: if open { None } else { Some("true") },
            class: cn!("overflow-hidden", class),
            style: "display:grid; grid-template-rows:{rows}; opacity:{opacity}; transform:{transform}; transform-origin:top; {pointer}transition:{transition}",
            div { style: "min-height:0; overflow:hidden; {height}", {children} }
        }
    }
}
