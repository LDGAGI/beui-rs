//! `lib/hooks/use-tap-gesture`.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

/// What a pointerdown recorded, read back by the click that ends its gesture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapRecord<S> {
    /// Which input started the gesture.
    pub pointer_type: String,
    /// What the surface was showing when it started.
    pub state: S,
}

/// The pointer gesture behind a click, recorded where the click cannot report
/// it.
///
/// A `click` carries no `pointerType` in the engines that matter, so the
/// `pointerdown` before it is the only thing that says which input activated the
/// control — and whether one did at all, since keyboard activation synthesizes a
/// click with no pointer behind it.
///
/// State goes in with the record because a click reports that no better: a
/// browser that focuses a control on contact can open the very panel the tap was
/// meant to open, and reading "is it open" at click time then undoes it. What
/// the gesture started against is what it acts on.
///
/// The record is spent by one click and dropped by everything else, because a
/// record that outlives its gesture is worse than none:
///
/// - A scroll or an OS gesture takes the touch away — `pointercancel`, no click
///   ever — and the finger would sit in the record until some later click.
/// - That later click is often `Enter` on a keyboard, which arrives with no
///   pointerdown of its own and would inherit the abandoned finger. A keydown is
///   the start of a keyboard activation and never part of a tap, so it drops the
///   record too.
///
/// Both ends have to be wired by the surface: `drop_record` on `onpointercancel`
/// and on `onkeydown`.
#[derive(Clone)]
pub struct TapGesture<S> {
    record: Rc<RefCell<Option<TapRecord<S>>>>,
}

impl<S> Default for TapGesture<S> {
    fn default() -> Self {
        Self {
            record: Rc::new(RefCell::new(None)),
        }
    }
}

impl<S: Clone> TapGesture<S> {
    /// Record the gesture a pointerdown starts, with the state it starts in.
    pub fn start(&self, event: &PointerData, state: S) {
        *self.record.borrow_mut() = Some(TapRecord {
            pointer_type: event.pointer_type(),
            state,
        });
    }

    /// Read the record and clear it. `None` when no pointer is behind this click.
    pub fn take(&self) -> Option<TapRecord<S>> {
        self.record.borrow_mut().take()
    }

    /// Drop the record: this gesture will never spend it on a click.
    pub fn drop_record(&self) {
        *self.record.borrow_mut() = None;
    }
}

/// See [`TapGesture`].
pub fn use_tap_gesture<S: Clone + 'static>() -> TapGesture<S> {
    use_hook(TapGesture::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_is_spent_by_one_take() {
        let gesture: TapGesture<u8> = TapGesture::default();
        gesture.record.borrow_mut().replace(TapRecord {
            pointer_type: "touch".into(),
            state: 7,
        });
        let taken = gesture.take().expect("record");
        assert_eq!(taken.pointer_type, "touch");
        assert_eq!(taken.state, 7);
        assert!(gesture.take().is_none());
    }

    #[test]
    fn drop_clears_an_abandoned_gesture() {
        let gesture: TapGesture<()> = TapGesture::default();
        gesture.record.borrow_mut().replace(TapRecord {
            pointer_type: "touch".into(),
            state: (),
        });
        gesture.drop_record();
        assert!(gesture.take().is_none());
    }
}
