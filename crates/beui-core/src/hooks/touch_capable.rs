//! `lib/hooks/use-touch-capable`.

use dioxus::prelude::*;

use super::media_query::use_js_boolean;

/// Returns true on devices that can be touched, whatever else they claim.
///
/// This is not the inverse of `use_hover_capable`: iPadOS Safari browses
/// desktop-class and answers `(hover: hover) and (pointer: fine)` with true
/// while a finger is the only input there is, so anything that treats
/// hover-capable as "no touch here" strands every iPad. Gate the *touch path* of
/// an interaction on this hook and leave hover-only polish on
/// `use_hover_capable`, so a component that opens on hover also opens on tap.
///
/// There is no single query behind the answer — `any-pointer: coarse` or a
/// non-zero `maxTouchPoints`, whichever speaks first — so the first render is
/// `false` on every platform and settles one tick later, which is what the
/// source's `useState(false) + useEffect` does too.
pub fn use_touch_capable() -> Signal<bool> {
    use_js_boolean(
        None,
        "matchMedia('(any-pointer: coarse)').matches || navigator.maxTouchPoints > 0",
    )
}
