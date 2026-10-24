//! `lib/hooks/use-hover-gesture`.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::touch::is_hovering_pointer;

/// Pairs a surface's enter with its leave, per pointer.
///
/// [`is_hovering_pointer`] answers the question the *enter* asks — is this
/// pointer resting on the surface or pressing it — and both boundary cases go
/// wrong if the leave is asked the same question again:
///
/// - A pen with no hover never rests. It arrives in contact, taps, and the spec
///   then requires its boundary events after `pointerup`, so the leave carries
///   `buttons: 0` and reads as a mouse gliding off. Hover teardown then undid
///   the tap — the panel the pen had just opened closed under it.
/// - A mouse pressed on the surface and dragged off leaves with `buttons`
///   non-empty. Skipping teardown there strands the surface open: the release
///   happens outside, and no second leave ever comes.
///
/// So the state a hover holds is released by the pointer that took it, whatever
/// the buttons say at the boundary, and a pointer that arrived in contact never
/// took it in the first place. Contact is the exception tracked here, not hover:
/// a leave from a pointer this surface never saw enter — mounted under the
/// cursor, say — still counts, since the alternative is state with no way out.
#[derive(Clone, Default)]
pub struct HoverGesture {
    contact: Rc<RefCell<HashSet<i32>>>,
}

impl HoverGesture {
    /// True when this enter starts a hover: the pointer arrived resting, not pressing.
    pub fn enter(&self, event: &PointerData) -> bool {
        self.enter_parts(event.pointer_id(), is_hovering_pointer(event))
    }

    /// True when this leave ends a hover that entered as one.
    pub fn leave(&self, event: &PointerData) -> bool {
        self.leave_parts(event.pointer_id(), event.pointer_type() != "touch")
    }

    fn enter_parts(&self, pointer_id: i32, hovering: bool) -> bool {
        if hovering {
            self.contact.borrow_mut().remove(&pointer_id);
            return true;
        }
        self.contact.borrow_mut().insert(pointer_id);
        false
    }

    fn leave_parts(&self, pointer_id: i32, not_touch: bool) -> bool {
        let arrived_in_contact = self.contact.borrow_mut().remove(&pointer_id);
        !arrived_in_contact && not_touch
    }
}

/// See [`HoverGesture`]. Call once per surface and use it in `onpointerenter` /
/// `onpointerleave`.
pub fn use_hover_gesture() -> HoverGesture {
    use_hook(HoverGesture::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `enter` / `leave` only ever read `(pointer_id, is_hovering,
    /// pointer_type)` off the event, so the contact bookkeeping — which is the
    /// whole point of the hook — is testable without a DOM.
    #[test]
    fn mouse_enter_then_leave_closes() {
        let gesture = HoverGesture::default();
        assert!(gesture.enter_parts(1, true));
        assert!(gesture.leave_parts(1, true));
    }

    #[test]
    fn pen_that_arrived_in_contact_does_not_close_on_its_leave() {
        // The leave of a contact-first pen reads as a mouse gliding off, but
        // the enter took no hover, so there is nothing to release.
        let gesture = HoverGesture::default();
        assert!(!gesture.enter_parts(2, false));
        assert!(!gesture.leave_parts(2, true));
    }

    #[test]
    fn mouse_dragged_off_while_pressed_still_closes() {
        let gesture = HoverGesture::default();
        assert!(gesture.enter_parts(3, true));
        assert!(gesture.leave_parts(3, true));
    }

    #[test]
    fn touch_never_closes() {
        let gesture = HoverGesture::default();
        assert!(!gesture.leave_parts(4, false));
    }

    #[test]
    fn leave_without_a_matching_enter_still_closes() {
        // Mounted under the cursor: state with no way out would be worse.
        let gesture = HoverGesture::default();
        assert!(gesture.leave_parts(5, true));
    }
}
