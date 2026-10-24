//! Touch / pointer primitives — a port of `lib/touch.ts`.
//!
//! iOS and iPadOS run their own gestures on top of the page — the long-press
//! selection callout and the selection it drags in with it — and they win: once
//! the platform claims a touch it cancels ours mid-gesture, so a press-and-hold
//! or a drag simply dies. Surfaces that own their gesture have to opt out.
//!
//! What the two classes below cover, precisely:
//!
//! - `-webkit-touch-callout: none` stops iOS's long-press callout. WebKit-only:
//!   it is not a property other engines have, so it is inert everywhere else.
//! - `user-select: none` stops the long-press selection on every engine,
//!   Android included, and stops a drag from painting a selection under the
//!   cursor. It is inherited, so it reaches every descendant — which is why the
//!   two classes differ only in whether they apply it unconditionally.
//!
//! What neither covers:
//!
//! - Chrome for Android's long-press menu on a link or an image. No CSS
//!   suppresses it; a gesture surface that wraps one needs its own
//!   `oncontextmenu` with `prevent_default()`.
//! - The native drag of an `<img>` or `<a>` descendant. `-webkit-user-drag` is
//!   not inherited and plain divs and buttons are not drag sources, so setting
//!   it on the surface does nothing — the child itself needs `draggable: false`.

use dioxus::prelude::{PointerData, PointerInteraction};

/// Classes for a surface that *is* the control: a thumb, a drum, a stage, a
/// handle, a hold button. Selection is suppressed on every input, because a
/// drag that highlights the control's own label is wrong on a mouse too.
/// Compose with `touch-none` when the surface also owns the scroll axis — leave
/// it off when the page must still scroll from there.
pub const TOUCH_GESTURE_CLASS: &str = "select-none [-webkit-touch-callout:none]";

/// The same opt-out for a gesture surface that wraps content the consumer owns:
/// a scroller, a context-menu trigger, a sheet header, a list row. Selection is
/// suppressed only where the platform runs its own press gestures — a coarse
/// pointer — so a mouse user can still select and copy that content.
///
/// `pointer: coarse` describes the *primary* pointer and nothing else, so a
/// hybrid machine reads it wrong in both directions: a tablet with a mouse
/// plugged in keeps touch as primary and loses mouse selection, and a laptop
/// with a touchscreen keeps the mouse as primary and leaves selection live
/// under a finger. No media query can answer per interaction — the query is
/// about the device, and the question is about the gesture in progress. The
/// default stays here because it is right on the machines that are one thing or
/// the other, and losing a selection is a nuisance; where the miss costs a
/// *gesture* instead, the surface pairs it with [`hold_selection_style`] on the
/// press.
pub const TOUCH_GESTURE_CONTENT_CLASS: &str =
    "[-webkit-touch-callout:none] pointer-coarse:select-none";

/// The inline `style` that suppresses selection for as long as a gesture runs,
/// whatever the primary pointer of the machine happens to be.
///
/// `holdSelection()` in the source mutates `element.style` directly and returns
/// a release closure. In Dioxus the element's `style` attribute is reactive, so
/// the equivalent is to append this fragment while the gesture is live and drop
/// it when the gesture ends — inline, which is why it wins over
/// [`TOUCH_GESTURE_CONTENT_CLASS`].
///
/// For the press gestures a native selection would otherwise steal — a
/// long-press that opens a menu. Elsewhere prefer the classes: a surface that
/// takes selection away for the whole session is a surface whose text nobody
/// can copy.
pub const HOLD_SELECTION_STYLE: &str = "user-select:none;-webkit-user-select:none";

/// Build a `style` value with [`HOLD_SELECTION_STYLE`] appended when `held`.
pub fn with_hold_selection(style: &str, held: bool) -> String {
    if !held {
        return style.to_string();
    }
    if style.is_empty() {
        return HOLD_SELECTION_STYLE.to_string();
    }
    let style = style.strip_suffix(';').unwrap_or(style);
    format!("{style};{HOLD_SELECTION_STYLE}")
}

/// Whether this event came from a pointer that is *hovering*: not a touch, and
/// not currently pressed.
///
/// Which input the user is holding right now is not something a device
/// capability can answer — a touchscreen laptop hovers and taps, and iPadOS
/// reports a fine hovering pointer for a finger — so both paths stay live and
/// each handler branches on the event it was given.
///
/// A pen resting on the glass is making contact, not hovering: `buttons` is the
/// tell, and it sends a pen tap down the same route a finger takes.
///
/// This answers what an *enter* asks. A leave is the other half of a pair and
/// has to be read against the enter that started it — [`use_hover_gesture`]
/// does that, and hover surfaces should use it rather than asking this question
/// twice.
///
/// [`use_hover_gesture`]: crate::hooks::use_hover_gesture
pub fn is_hovering_pointer(event: &PointerData) -> bool {
    event.pointer_type() != "touch" && event.held_buttons().is_empty()
}

/// The two fields [`is_hovering_pointer`] reads, for callers holding a raw
/// pointer event payload rather than a Dioxus [`PointerData`].
pub fn is_hovering_pointer_parts(pointer_type: &str, held_buttons_empty: bool) -> bool {
    pointer_type != "touch" && held_buttons_empty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gesture_class_strings_are_verbatim() {
        assert_eq!(
            TOUCH_GESTURE_CLASS,
            "select-none [-webkit-touch-callout:none]"
        );
        assert_eq!(
            TOUCH_GESTURE_CONTENT_CLASS,
            "[-webkit-touch-callout:none] pointer-coarse:select-none"
        );
    }

    #[test]
    fn hover_pointer_ignores_touch_and_pressed_pointers() {
        assert!(is_hovering_pointer_parts("mouse", true));
        assert!(is_hovering_pointer_parts("pen", true));
        assert!(!is_hovering_pointer_parts("touch", true));
        // A pen (or a mouse) in contact is not hovering.
        assert!(!is_hovering_pointer_parts("pen", false));
        assert!(!is_hovering_pointer_parts("mouse", false));
    }

    #[test]
    fn hold_selection_appends_without_double_semicolons() {
        assert_eq!(with_hold_selection("", false), "");
        assert_eq!(with_hold_selection("color:red", false), "color:red");
        assert_eq!(
            with_hold_selection("color:red;", true),
            "color:red;user-select:none;-webkit-user-select:none"
        );
        assert_eq!(with_hold_selection("", true), HOLD_SELECTION_STYLE);
    }
}
