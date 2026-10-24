//! `lib/hooks/` — one module per hook, same names, same behaviour.
//!
//! Each hook returns a `Signal` rather than a plain value where the source uses
//! `useState`: reading the signal is what subscribes the calling component, so
//! `let hoverable = use_hover_capable();` inside `rsx!` re-renders exactly like
//! the React state did.

mod dismiss;
mod hover_capable;
mod hover_gesture;
mod media_query;
mod modal_scope;
mod on_open;
mod reduced_motion;
mod row_cursor;
mod slider;
mod tap_gesture;
mod touch_capable;

pub use dismiss::{use_dismiss, DismissBehavior, DismissOptions, OverlayScope};
pub use hover_capable::use_hover_capable;
pub use hover_gesture::{use_hover_gesture, HoverGesture};
pub use media_query::{use_js_boolean, use_media_query};
pub use modal_scope::use_modal_scope;
pub use on_open::use_on_open;
pub use reduced_motion::use_reduced_motion;
pub use row_cursor::{index_of_cursor, use_row_cursor, RowCursor, RowCursorState, RowStep};
pub use slider::{snap_slider_value, use_slider, Slider, SliderConfig};
pub use tap_gesture::{use_tap_gesture, TapGesture, TapRecord};
pub use touch_capable::use_touch_capable;
