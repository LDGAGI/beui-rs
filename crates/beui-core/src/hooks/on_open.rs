//! `lib/hooks/use-on-open`.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::*;

/// Run `start` when `open` becomes true.
///
/// Opening a list starts a fresh session — an empty query, the highlight back at
/// the top — and that is a resolution, not a side effect: a passive effect runs
/// after the commit, so the new session would carry the last one's state for a
/// window in which a key press can land.
///
/// That window does not exist here. React needs the work to happen during render
/// because a commit can be deferred and played back later; Dioxus commits
/// synchronously and dispatches no events until it has, so an effect keyed to
/// `open` runs in exactly the same relationship to the next keystroke — and it
/// is the only place a hook may write state without dirtying the scope it is
/// running in.
///
/// Seeded with the value `open` already had on mount, so mounting an open list
/// does not fire `start`: the call is for a transition, not for a state.
///
/// `open` is a plain `bool` rather than a signal because the effect has to
/// re-run when it changes; [`use_reactive`] is what turns the value into a
/// dependency.
pub fn use_on_open(open: bool, mut start: impl FnMut() + 'static) {
    let was_open = use_hook(move || Rc::new(Cell::new(open)));

    use_effect(use_reactive((&open,), move |(open,)| {
        if open != was_open.get() {
            was_open.set(open);
            if open {
                start();
            }
        }
    }));
}
