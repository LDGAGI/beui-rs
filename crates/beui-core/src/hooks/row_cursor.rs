//! `lib/hooks/use-row-cursor`.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

/// Where the keyboard or the pointer last moved to: the row's id, stamped with
/// the query it was placed under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowCursorState {
    pub id: String,
    pub query: String,
}

/// Which way a key moved the highlight.
///
/// The source takes `1 | -1`; this is the same pair with the direction named,
/// so a caller reads `RowStep::Next` for `ArrowDown` rather than guessing at a
/// sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowStep {
    /// One row down: the next entry.
    Next,
    /// One row up: the previous entry.
    Previous,
}

impl RowStep {
    const fn delta(self) -> i64 {
        match self {
            RowStep::Next => 1,
            RowStep::Previous => -1,
        }
    }
}

/// The cursor's row, or `None` once the query has moved on or the row has left.
pub fn index_of_cursor(
    rows: &[String],
    query: &str,
    cursor: Option<&RowCursorState>,
) -> Option<usize> {
    let cursor = cursor?;
    if cursor.query != query {
        return None;
    }
    rows.iter().position(|row| *row == cursor.id)
}

#[derive(Default)]
struct Latest {
    rows: Vec<String>,
    query: String,
}

/// The highlighted row of a list whose rows can change under it.
///
/// Which row is highlighted is resolved during render, never in a passive
/// effect: a passive effect runs after the commit, so a list that had just
/// changed would carry an `aria-activedescendant` naming a row that has left it,
/// and a key pressed in that window would commit the wrong row or nothing at
/// all.
///
/// The cursor holds the row's id, not its position. A position alone cannot tell
/// a list that shrank from one that swapped its rows for a different set of the
/// same length, and the second case is the one that silently hands Enter to a row
/// the user never chose. A cursor whose row has left the list returns the
/// highlight to the first row rather than to the nearest surviving one: the row
/// the user aimed at is gone, and the first row is where a new query already puts
/// the highlight.
///
/// The cursor is stamped with the query it was placed under and dropped when it
/// changes, so a caller cannot forget to clear it. Rows that come back under a
/// query that has moved on cannot revive it either. [`RowCursor::move_to`] with
/// `None` is for deliberate resets, such as reopening the list.
///
/// Two things differ from the React original, both for the same reason — Dioxus
/// has no render phase that can be discarded or replayed:
///
/// - the rows and the query are written into a plain `Rc<RefCell<_>>` during
///   render, where React needs a `useLayoutEffect` to keep a concurrent render
///   from stamping the cursor with a query the committed tree does not have;
/// - clearing a stale cursor mutates that same cell instead of calling
///   `setState`, so it cannot schedule another render from inside one. A
///   `revision` signal carries the notification instead, and each mutation bumps
///   it.
#[derive(Clone)]
pub struct RowCursor {
    cursor: Rc<RefCell<Option<RowCursorState>>>,
    latest: Rc<RefCell<Latest>>,
    revision: Signal<u32>,
}

impl RowCursor {
    /// The highlighted row, `0` when nothing is highlighted or the list is empty.
    ///
    /// Reads the revision signal, so calling this during render is what
    /// subscribes the component.
    pub fn active_index(&self) -> usize {
        let _ = (self.revision)();
        let found = {
            let latest = self.latest.borrow();
            let cursor = self.cursor.borrow();
            index_of_cursor(&latest.rows, &latest.query, cursor.as_ref())
        };
        match found {
            Some(index) => index,
            None => {
                // Cleared rather than ignored, so rows that come back under the
                // same query cannot revive a highlight the user stopped aiming
                // at.
                self.cursor.borrow_mut().take();
                0
            }
        }
    }

    /// The highlighted row's id, `None` when the list is empty.
    pub fn active_id(&self) -> Option<String> {
        let index = self.active_index();
        self.latest.borrow().rows.get(index).cloned()
    }

    /// Move the highlight to `id`, or clear it with `None`.
    pub fn move_to(&self, id: Option<&str>) {
        let query = self.latest.borrow().query.clone();
        *self.cursor.borrow_mut() = id.map(|id| RowCursorState {
            id: id.to_string(),
            query,
        });
        self.bump();
    }

    /// Step the highlight one row, clamped to the ends.
    pub fn move_active(&self, step: RowStep) {
        let (id, query) = {
            let latest = self.latest.borrow();
            let Some(last) = latest.rows.len().checked_sub(1) else {
                return;
            };
            // Steps from the row the cursor is really on, so two keys in a row
            // move two rows.
            let at = {
                let cursor = self.cursor.borrow();
                index_of_cursor(&latest.rows, &latest.query, cursor.as_ref()).unwrap_or(0)
            };
            let next = (at as i64 + step.delta()).clamp(0, last as i64) as usize;
            (latest.rows[next].clone(), latest.query.clone())
        };
        *self.cursor.borrow_mut() = Some(RowCursorState { id, query });
        self.bump();
    }

    fn bump(&self) {
        let mut revision = self.revision;
        revision += 1;
    }
}

/// See [`RowCursor`]. Call once per list; `rows` is the list's ids in order and
/// `query` is whatever the list is filtered by.
pub fn use_row_cursor(rows: Vec<String>, query: &str) -> RowCursor {
    let cursor = use_hook(|| Rc::new(RefCell::new(None)));
    let latest = use_hook(|| Rc::new(RefCell::new(Latest::default())));
    let revision = use_signal(|| 0u32);

    *latest.borrow_mut() = Latest {
        rows,
        query: query.to_string(),
    };

    RowCursor {
        cursor,
        latest,
        revision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<String> {
        ["a", "b", "c"].iter().map(|id| id.to_string()).collect()
    }

    fn cursor(id: &str, query: &str) -> RowCursorState {
        RowCursorState {
            id: id.to_string(),
            query: query.to_string(),
        }
    }

    #[test]
    fn cursor_resolves_to_its_row() {
        assert_eq!(
            index_of_cursor(&rows(), "q", Some(&cursor("b", "q"))),
            Some(1)
        );
    }

    #[test]
    fn a_cursor_from_another_query_does_not_resolve() {
        assert_eq!(
            index_of_cursor(&rows(), "q2", Some(&cursor("b", "q"))),
            None
        );
    }

    #[test]
    fn a_cursor_whose_row_left_does_not_resolve() {
        let shrunk = vec!["a".to_string()];
        assert_eq!(index_of_cursor(&shrunk, "q", Some(&cursor("c", "q"))), None);
    }

    #[test]
    fn no_cursor_resolves_to_nothing() {
        assert_eq!(index_of_cursor(&rows(), "q", None), None);
    }

    #[test]
    fn steps_clamp_at_both_ends() {
        // The maths `move_active` runs, without a runtime: `at` clamped to
        // `[0, last]` after the step.
        for (at, step, expected) in [
            (0usize, RowStep::Next, 1usize),
            (2, RowStep::Next, 2),
            (0, RowStep::Previous, 0),
            (2, RowStep::Previous, 1),
        ] {
            let last = 2i64;
            let next = (at as i64 + step.delta()).clamp(0, last) as usize;
            assert_eq!(next, expected);
        }
    }

    #[test]
    fn step_deltas_are_one_row() {
        assert_eq!(RowStep::Next.delta(), 1);
        assert_eq!(RowStep::Previous.delta(), -1);
    }
}
