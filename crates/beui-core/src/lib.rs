//! beUI shared primitives.
//!
//! Shape of the crate, mirroring `lib/` and `lib/hooks/` in the beUI source:
//!
//! | beUI | here |
//! |---|---|
//! | `lib/utils.ts` `cn()` | [`cn!`] + [`Class`] |
//! | `lib/ease.ts` | [`ease`] |
//! | `lib/touch.ts` | [`touch`] |
//! | `lib/presence-gate.tsx` + `AnimatePresence` | [`presence`] |
//! | `layout` / `layoutId` | [`flip`] |
//! | `drag` / `dragConstraints` / `dragElastic` / `dragMomentum` | [`drag`] |
//! | `createPortal(…, document.body)` | [`portal`] |
//! | `lib/hooks/*` | [`hooks`] |
//! | DOM reads (`matchMedia`, `localStorage`, `<html>` class) | [`platform`] |
//!
//! `platform` is the one place that has to differ between renderers: the web
//! renderer reaches the DOM through `web-sys`, while the desktop/mobile
//! renderers only have `document::eval` (their DOM lives inside a WebView that
//! the Rust side cannot touch directly). Everything above this module is
//! renderer-agnostic.

pub mod class;
pub mod drag;
pub mod ease;
pub mod flip;
pub mod hooks;
pub mod platform;
pub mod portal;
pub mod presence;
pub mod touch;

pub use class::Class;
pub use drag::{
    use_drag, Axes, Drag, DragAxis, DragConstraints, DragElastic, DragEnd, DragOptions, DragRange,
};
pub use flip::{use_flip, Flip, FlipMode, FlipOptions};
pub use hooks::{
    snap_slider_value, use_dismiss, use_hover_capable, use_hover_gesture, use_media_query,
    use_modal_scope, use_on_open, use_reduced_motion, use_row_cursor, use_slider, use_tap_gesture,
    use_touch_capable, DismissBehavior, DismissOptions, HoverGesture, OverlayScope, RowCursor,
    RowCursorState, RowStep, Slider, SliderConfig, TapGesture, TapRecord,
};
pub use portal::{use_portal, PORTAL_ATTRIBUTE};
pub use presence::{
    use_gate, use_is_present, use_presence, use_presence_style, AnimatePresence, Gate,
    PresenceConfig, PresenceGate, PresenceLayout, PresenceMode,
};
