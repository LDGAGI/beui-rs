//! `lib/hooks/use-hover-capable`.

use dioxus::prelude::*;

use super::media_query::use_js_boolean;

/// Returns true only on devices that have a true hover (mouse / trackpad).
///
/// Touch devices fire phantom `:hover` on tap that sticks until tap-elsewhere —
/// gate hover-only effects (scale lifts, magnetic pulls) behind this.
pub fn use_hover_capable() -> Signal<bool> {
    use_js_boolean(
        Some("(hover: hover) and (pointer: fine)"),
        "matchMedia('(hover: hover) and (pointer: fine)').matches",
    )
}
