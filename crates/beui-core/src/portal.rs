//! `portal` — the overlay mount point.
//!
//! Every beUI overlay is a portal: 19 files call `createPortal(…, document.body)`
//! (popover, context menu, select/combobox/multi-select content, command palette,
//! tooltips, the modal family, the toast stack, the lightbox, the table menus).
//! Two reasons, and both are about what an overlay has to escape:
//!
//! - **Ancestors.** A `position: fixed` element is positioned against the nearest
//!   ancestor with a transform, a filter or `will-change` — not the viewport — and
//!   it is clipped by any ancestor with `overflow: hidden`. A card with a hover
//!   lift (a transform) is enough to break every popover inside it.
//! - **Stacking.** A parent with its own `z-index` creates a stacking context, and
//!   no child can paint above a sibling of that parent however large its own
//!   `z-index` is.
//!
//! In React the portal is a first-class thing. Dioxus has no portal, so this is
//! one built out of the renderer's own behaviour: the component puts
//! `data-beui-portal` on the element it wants moved and [`Portal::attach`] moves
//! it under `<body>` on mount. See `src/js/portal.js` for why the move is safe —
//! the short version is that Dioxus's mutations are node-relative, so a moved
//! subtree keeps being updated where it is.
//!
//! The element that moves is the element the component already renders, so the
//! DOM ends up *identical* to the source's: an overlay as a direct child of
//! `body`, carrying its own classes. Nothing is wrapped, and nothing is
//! re-parented visually.
//!
//! # The z-order convention
//!
//! beUI does not have a shared z-index scale; each overlay copies the class its
//! design calls for, and the numbers cluster like this (counts from the source,
//! `components/motion` + `components/agents`):
//!
//! | z | where | examples |
//! |---|---|---|
//! | `z-10` / `z-20` | inside a component's own stacking context | card content, toast content |
//! | `z-30` / `z-40` | sticky chrome, sidebar rail, stage overlays | animated sidebar, marquee fade |
//! | `z-50` | the overlay layer proper (12 uses) | drawer, modal, select, command palette, toast stack |
//! | `z-[60]` … `z-[101]` | a second overlay over the first (6 + 1 + 2 uses) | nested menus, lightbox controls |
//! | `z-[9999]` | pointer-positioned hosts (5 uses) | popover, context menu |
//!
//! The rule the port follows is therefore: **copy the class from the source**, and
//! portal only the layer that has to escape. A portaled element outside a
//! stacking context stacks by its own `z-index` exactly as it does in the source,
//! and two portaled layers stack against each other in the same order.
//!
//! Nothing here sets a z-index: a portal that decided one would be inventing a
//! number the source does not have.

use dioxus::prelude::*;

use crate::platform;

const PORTAL_JS: &str = include_str!("js/portal.js");

/// The attribute a portalable element carries.
pub const PORTAL_ATTRIBUTE: &str = "data-beui-portal";

/// A handle for moving one element under `<body>`.
///
/// ```ignore
/// let portal = use_portal();
/// rsx! {
///     div {
///         "data-beui-portal": portal.scope(),
///         onmounted: move |_| portal.attach(),
///         class: "fixed inset-0 z-50",
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Portal {
    id: u32,
}

impl Portal {
    /// The `data-beui-portal` value the element must carry.
    pub fn scope(&self) -> String {
        self.id.to_string()
    }

    /// The selector the script finds the element by.
    pub fn selector(&self) -> String {
        format!("[{PORTAL_ATTRIBUTE}=\"{}\"]", self.id)
    }

    /// The script that moves the element, as a function of the selector.
    ///
    /// Public so a test can check what would run without a browser.
    pub fn script(&self) -> String {
        PORTAL_JS.replace("__BEUI_SELECTOR__", &platform::js_string(&self.selector()))
    }

    /// Move the element under `<body>`.
    ///
    /// Wire this to `onmounted`: it fires once per *element*, so a layer that is
    /// unmounted and mounted again is moved again. An effect would only fire once
    /// per component and would leave the second element inline.
    pub fn attach(&self) {
        platform::fire(&self.script());
    }
}

/// See [`Portal`]. Call it in the component that owns the overlay element.
pub fn use_portal() -> Portal {
    Portal {
        id: use_hook(platform::next_scope_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_targets_the_elements_attribute() {
        // The selector is built from the same constant the element is given, so
        // the two cannot drift.
        let script = super::PORTAL_JS.replace(
            "__BEUI_SELECTOR__",
            &platform::js_string(&format!("[{PORTAL_ATTRIBUTE}=\"7\"]")),
        );
        assert!(script.contains(r#"[data-beui-portal=\"7\"]"#));
        assert!(script.contains("document.body.appendChild(element)"));
        // The move is idempotent: an element already under `body` is left alone,
        // which matters because the mounted event can fire again after Dioxus
        // re-creates the element.
        assert!(script.contains("element.parentElement !== document.body"));
    }

    #[test]
    fn the_scope_is_the_attribute_value_and_the_selector_finds_it() {
        let portal = Portal { id: 42 };
        assert_eq!(portal.scope(), "42");
        assert_eq!(portal.selector(), "[data-beui-portal=\"42\"]");
        assert!(portal.script().contains("document.body.appendChild"));
    }
}
