//! Presence — beUI's `AnimatePresence` + `PresenceGate` layer.
//!
//! beUI keeps an overlay mounted until its exit animation finishes, and every
//! layer that takes pointer events while it is open reads `isPresent` so that
//! interaction releases in the same commit the exit starts rather than when it
//! ends. Both halves come from motion: `AnimatePresence` retains a keyed child
//! that has left the tree and unmounts it when the child says its exit work is
//! done, `useIsPresent()` says which side of that boundary a child is on, and
//! `lib/presence-gate.tsx` is the three-line component that hands the boolean
//! down to the layers.
//!
//! [`dioxus_motion::presence`] is the port of that machinery, including the
//! parts that only exist to keep the retained subtree alive and animating (it
//! re-renders the child's captured `VNode`, which is why children must be keyed
//! and why the retained element keeps the props it had when it was removed —
//! the same trade framer-motion makes). This module is the layer on top:
//!
//! - it re-exports what a ported component needs, so a component file has one
//!   import site instead of reaching into `dioxus_motion` directly;
//! - it turns `isPresent` into the [`Gate`] the source spreads onto a layer.
//!
//! The one thing that does not carry over is the render prop. `PresenceGate`
//! takes `children: Element` in React because `useIsPresent()` must be called
//! *inside* the `AnimatePresence` subtree, so the value has to be read one
//! component further down and passed back up through a function. Dioxus has the
//! same constraint — a component outside the subtree cannot read the context —
//! but it can solve it two ways instead of one: [`use_gate`] reads the presence
//! directly in any component that is mounted inside a presence child, and
//! [`PresenceGate`] provides the same value through a context for components
//! that are nested further down. Ported overlays use whichever of the two
//! keeps the DOM identical to the source.

use dioxus::prelude::*;

pub use dioxus_motion::animations::style::MotionStyle;
pub use dioxus_motion::manager::MotionHandle;
pub use dioxus_motion::presence::{
    use_is_present, use_presence, use_presence_data, use_presence_motion,
    use_presence_motion_completion, use_presence_style, AnimatePresence, PresenceAnchorX,
    PresenceAnchorY, PresenceConfig, PresenceCustom, PresenceHandle, PresenceLayout, PresenceMode,
};
pub use dioxus_motion::{motion_style, presence_style, AnimationManager, Duration};

/// The current CSS declaration list of a presence style handle.
///
/// A presence animation is written to the element's `style` attribute every
/// frame, so the handle is read during render and handed to `style` — the same
/// reactive-attribute path every other animation in the port takes, which is
/// what keeps it working on the WebView renderers.
pub fn presence_css(handle: &MotionHandle<MotionStyle>) -> String {
    handle.get_value().to_css()
}

/// `PresenceGateRenderProps["gate"]`: what a layer does with the presence it was
/// read from.
///
/// The source spreads both fields onto the layer — `inert` and
/// `style.pointerEvents` — and warns about the asymmetry between them:
///
/// - `inert` goes on *every* layer. It drops the subtree from focus order, from
///   tab order and from the accessibility tree, so an exiting dialog is not a
///   dialog that can still be typed into.
/// - `pointer-events` goes only on a layer that *takes* pointer events while
///   open. A wrapper that only centres the panel already has
///   `pointer-events-none`, and writing `auto` over it would take the wrapper's
///   own opt-out away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gate {
    pub is_present: bool,
}

impl Gate {
    /// `isPresent`: false from the commit that starts the exit onward.
    pub fn is_present(&self) -> bool {
        self.is_present
    }

    /// `gate.inert` — whether the subtree is inert.
    pub fn inert(&self) -> bool {
        !self.is_present
    }

    /// The `inert` attribute value for `rsx!`.
    ///
    /// `Some("true")` while exiting, `None` while present — never `false`.
    /// `inert` is a boolean attribute: it is true when it is *present*, and
    /// `inert="false"` is still present, so writing the boolean straight through
    /// would lock the layer in exactly the state that is supposed to release it.
    /// (`element.inert` reads `true` for `inert="false"` in every engine.) An
    /// `Option` value is what removes the attribute instead of writing it.
    pub fn inert_attr(&self) -> Option<&'static str> {
        self.inert().then_some("true")
    }

    /// `gate.style.pointerEvents`.
    pub fn pointer_events(&self) -> &'static str {
        if self.is_present {
            "auto"
        } else {
            "none"
        }
    }

    /// `gate.style` as a CSS declaration list, for an element with no style of
    /// its own.
    pub fn style(&self) -> &'static str {
        if self.is_present {
            "pointer-events:auto"
        } else {
            "pointer-events:none"
        }
    }

    /// `{...gate}` for an element that already has an inline style: the layer's
    /// own declarations first, the gate's last, so the gate wins — which is what
    /// a later `style` object in JSX does.
    pub fn style_with(&self, base: &str) -> String {
        let base = base.trim().trim_end_matches(';');
        if base.is_empty() {
            return self.style().to_string();
        }
        format!("{base};{}", self.style())
    }
}

impl From<bool> for Gate {
    fn from(is_present: bool) -> Self {
        Self { is_present }
    }
}

/// The `gate` the source's render prop would hand down, read straight from the
/// nearest `AnimatePresence` boundary.
///
/// Call it in a component mounted inside a presence child. Outside one it reads
/// as present, which is what `useIsPresent()` does in the source: a component
/// that is not in a presence subtree has nothing to exit from.
pub fn use_gate() -> Gate {
    Gate::from(use_is_present())
}

/// Provide the gate to descendant components.
///
/// The Dioxus counterpart of beUI's `PresenceGate`: the keyed child of
/// `AnimatePresence`, with the gate reachable by anything mounted inside it via
/// [`use_gate`]. Components whose markup is built inline in the same component
/// that owns the `AnimatePresence` do not need it — they cannot read the context
/// from out there either way — so they read [`use_gate`] in an inner component
/// instead.
#[component]
pub fn PresenceGate(children: Element) -> Element {
    let gate = use_gate();
    use_context_provider(|| gate);
    rsx! {
        {children}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_follows_presence() {
        let present = Gate::from(true);
        assert!(present.is_present());
        assert!(!present.inert());
        assert_eq!(present.pointer_events(), "auto");
        assert_eq!(present.style(), "pointer-events:auto");

        assert_eq!(present.inert_attr(), None);
        assert_eq!(Gate::from(false).inert_attr(), Some("true"));

        let exiting = Gate::from(false);
        assert!(!exiting.is_present());
        assert!(exiting.inert());
        assert_eq!(exiting.pointer_events(), "none");
        assert_eq!(exiting.style(), "pointer-events:none");
    }

    #[test]
    fn gate_appends_to_a_layer_style_and_wins() {
        let exiting = Gate::from(false);
        assert_eq!(
            exiting.style_with("transform:translateX(0px)"),
            "transform:translateX(0px);pointer-events:none"
        );
        // A trailing semicolon must not double up.
        assert_eq!(
            exiting.style_with("opacity: 0.5;"),
            "opacity: 0.5;pointer-events:none"
        );
        // An empty layer style is just the gate.
        assert_eq!(exiting.style_with(""), "pointer-events:none");
        assert_eq!(exiting.style_with("   "), "pointer-events:none");
    }
}
