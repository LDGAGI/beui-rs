//! `lib/hooks/use-reduced-motion` — the JS half of beUI's reduced-motion track.
//!
//! beUI handles reduced motion twice over. `globals.css` collapses every CSS
//! animation and transition to `0.01ms` under `@media (prefers-reduced-motion:
//! reduce)`, and every component that animates through JS reads
//! `useReducedMotion()` — 140 files in the source — so the springs can be
//! replaced by a direct jump. The CSS rule is already in
//! [`beui_theme::globals_css`]; this is the other half, and a component that
//! skips it will still animate for a user who asked for no motion.

use dioxus::prelude::*;

use super::media_query::use_media_query;

/// `useReducedMotion()`: true when the user asked the platform for less motion.
///
/// The value is read on every render, so a component re-renders when the
/// preference changes. Read it once at the top of a component and branch on it
/// around every spring and tween:
///
/// ```ignore
/// let reduce = use_reduced_motion();
/// let config = if reduce() { AnimationConfig::tween_ms(0) } else { AnimationConfig::spring(SPRING_PRESS) };
/// ```
pub fn use_reduced_motion() -> Signal<bool> {
    use_media_query("(prefers-reduced-motion: reduce)")
}
