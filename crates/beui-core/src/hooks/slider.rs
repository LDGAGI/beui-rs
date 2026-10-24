//! `lib/hooks/use-slider`.
//!
//! Shared value + input plumbing for slider designs: controlled/uncontrolled
//! value, step snapping, pointer-capture drag along a track and arrow-key
//! control. Visuals and motion live in the component; this only owns the number,
//! exactly as `lib/hooks/use-slider.ts` does.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::platform::{self, Rect};

/// `getBoundingClientRect` through the evaluator, for the renderers that cannot
/// mount-measure and as a refresh when a drag starts.
async fn measure_script(selector: &str) -> Option<Rect> {
    let script = format!(
        "const el = document.querySelector({});\n\
         if (!el) return null;\n\
         const r = el.getBoundingClientRect();\n\
         return {{ left: r.left, top: r.top, width: r.width, height: r.height }};",
        platform::js_string(selector)
    );
    platform::json::<Rect>(&script).await
}

/// `clamp(v, lo, hi)` from the source.
fn clamp(value: f64, lo: f64, hi: f64) -> f64 {
    value.min(hi).max(lo)
}

/// `Number(x.toFixed(6))` — six decimals, halves away from zero.
///
/// The source rounds through a decimal string on every step of the snap, so the
/// grid lands on numbers a user can read (`0.1`, not `0.30000000000000004`).
/// `f64::round` already rounds halves away from zero; the multiplication is what
/// makes it six decimals instead of integers.
fn round6(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

/// `Math.round(x)` — halves toward positive infinity.
///
/// Rust's `f64::round` rounds halves away from zero, which differs for negative
/// inputs; the snap can produce one (`next < min`), and the source's `Math.round`
/// is what the grid was designed against.
fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// Nearest legal value on `[min, max]` for the given step.
///
/// `max` counts as a candidate when the step does not divide the range, so a
/// pointer near the end does not snap back onto the last whole step.
// The source writes `!(max > min)` and `!(step > 0)` rather than `max <= min`
// and `step <= 0` so that a NaN bound falls into the degenerate branch instead
// of walking a NaN grid. Keeping the negated form is the point; the lint is
// about readability, not about this.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn snap_slider_value(next: f64, min: f64, max: f64, step: f64) -> f64 {
    // Neither case has a grid to walk. An empty range has exactly one legal
    // point, and a non-positive step only needs a clamp, which also keeps the
    // division below away from zero.
    if !(max > min) {
        return min;
    }
    if !(step > 0.0) {
        return clamp(next, min, max);
    }
    let whole = round6((max - min) / step).floor();
    let last_whole = round6(min + whole * step);
    let to_grid = clamp(js_round((next - min) / step) * step + min, min, last_whole);
    let snapped = if last_whole < max && (next - max).abs() <= (next - to_grid).abs() {
        max
    } else {
        to_grid
    };
    round6(snapped)
}

/// Everything a slider component configures.
#[derive(Clone, Copy, PartialEq)]
pub struct SliderConfig {
    /// Lower bound of the range. Default `0`.
    pub min: f64,
    /// Upper bound of the range. Default `100`.
    pub max: f64,
    /// Step between legal values. Default `1`.
    pub step: f64,
    /// The initial value when uncontrolled. Default `0`.
    pub default_value: f64,
    /// Input is ignored while set; the value is not clamped or reset.
    pub disabled: bool,
    /// Announced instead of the raw number — pass one when the value carries a
    /// unit or a suffix ("72.5 kg", "35%"); a bare number needs no value text.
    pub value_text: Option<Callback<f64, String>>,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            default_value: 0.0,
            disabled: false,
            value_text: None,
        }
    }
}

/// The slider's plumbing: the value, the drag state and the input handlers.
#[derive(Clone, Copy, PartialEq)]
pub struct Slider {
    internal: Signal<f64>,
    dragging: Signal<bool>,
    track: Signal<Option<Rect>>,
    controlled: Option<f64>,
    lo: f64,
    hi: f64,
    stride: f64,
    disabled: bool,
    value_text: Option<Callback<f64, String>>,
    on_value_change: Option<EventHandler<f64>>,
    id: u32,
}

impl Slider {
    /// The value right now, clamped to the range.
    pub fn current(&self) -> f64 {
        let raw = match self.controlled {
            Some(value) => value,
            None => (self.internal)(),
        };
        clamp(raw, self.lo, self.hi)
    }

    /// The value as a percentage of the range, for positioning the fill.
    pub fn percent(&self) -> f64 {
        if self.hi > self.lo {
            ((self.current() - self.lo) / (self.hi - self.lo)) * 100.0
        } else {
            0.0
        }
    }

    /// Whether a drag is in progress.
    pub fn dragging(&self) -> bool {
        (self.dragging)()
    }

    /// The effective range and step, after inverted ranges and non-positive
    /// steps have been collapsed.
    pub fn range(&self) -> (f64, f64) {
        (self.lo, self.hi)
    }

    pub fn step(&self) -> f64 {
        self.stride
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }

    /// `aria-valuetext`, when a formatter was configured.
    pub fn aria_value_text(&self) -> Option<String> {
        let format = self.value_text?;
        Some(format.call(self.current()))
    }

    /// The `data-beui-slider` value the track must carry.
    ///
    /// It is the handle for the pointer-capture fallback and for the mount-time
    /// measurement off the web renderer, and it is what the verification script
    /// addresses the track by.
    pub fn scope(&self) -> String {
        self.id.to_string()
    }

    /// The selector for the track element.
    pub fn track_selector(&self) -> String {
        format!("[data-beui-slider=\"{}\"]", self.id)
    }

    /// The selector for the focusable handle.
    pub fn handle_selector(&self) -> String {
        format!("[data-beui-slider-handle=\"{}\"]", self.id)
    }

    /// Snap and commit `next`.
    pub fn commit(&self, next: f64) {
        let clean = snap_slider_value(next, self.lo, self.hi, self.stride);
        if self.controlled.is_none() {
            let mut internal = self.internal;
            internal.set(clean);
        }
        if let Some(on_value_change) = self.on_value_change {
            on_value_change.call(clean);
        }
    }

    /// Snap and commit the value at `x`, a viewport coordinate.
    pub fn commit_from_x(&self, x: f64) {
        let Some(rect) = (self.track)() else {
            return;
        };
        self.commit(self.lo + rect.ratio_x(x) * (self.hi - self.lo));
    }

    /// `onmounted` on the track element: record where it is on screen.
    ///
    /// Every renderer implements `MountedData`, so this is the exact
    /// `getBoundingClientRect` the source reads — and it costs no script. Takes
    /// the `Rc` the event hands out, because the measurement has to be awaited
    /// and `MountedData` is not cloneable.
    pub fn measure(&self, data: Rc<MountedData>) {
        let mut track = self.track;
        spawn(async move {
            if let Some(rect) = platform::client_rect(&data).await {
                track.set(Some(rect));
            }
        });
    }

    /// Re-measure through a script.
    fn remeasure(&self) {
        let mut track = self.track;
        let selector = self.track_selector();
        spawn(async move {
            if let Some(rect) = measure_script(&selector).await {
                track.set(Some(rect));
            }
        });
    }

    /// `onpointerdown` on the track: start the drag, focus the handle and commit.
    ///
    /// Takes the whole event because it cancels the default action, and the
    /// default action is the problem: `pointerdown` fires before the
    /// compatibility `mousedown`, and `mousedown` is what moves focus to
    /// `<body>` when the target is not itself focusable — undoing the handle
    /// focus below. Cancelling `pointerdown` suppresses the compatibility mouse
    /// events entirely, which also stops the drag from painting a selection. The
    /// source focuses the handle the same way and inherits the same race; the
    /// observable behaviour it documents is the one kept here.
    pub fn on_pointer_down(&self, event: &Event<PointerData>) {
        if self.disabled {
            return;
        }
        event.prevent_default();
        // Start the drag first: capture is a convenience, and a browser that
        // refuses it must not take the drag down with it.
        let mut dragging = self.dragging;
        dragging.set(true);
        if !platform::capture_pointer(event) {
            platform::capture_pointer_selector(&self.track_selector(), event.pointer_id());
        }
        // A click on the track should land keyboard focus on the handle.
        platform::focus_selector(&self.handle_selector(), true);

        let x = event.client_coordinates().x;
        if self.track.peek().is_some() {
            // Committed against the measurement already in hand; the next drag
            // gets a fresh one.
            self.commit_from_x(x);
            self.remeasure();
        } else {
            // Nothing measured yet: measure once, then commit from it.
            let mut track = self.track;
            let lo = self.lo;
            let hi = self.hi;
            let stride = self.stride;
            let controlled = self.controlled;
            let mut internal = self.internal;
            let on_value_change = self.on_value_change;
            let selector = self.track_selector();
            spawn(async move {
                let Some(rect) = measure_script(&selector).await else {
                    return;
                };
                track.set(Some(rect));
                let clean = snap_slider_value(lo + rect.ratio_x(x) * (hi - lo), lo, hi, stride);
                if controlled.is_none() {
                    internal.set(clean);
                }
                if let Some(on_value_change) = on_value_change {
                    on_value_change.call(clean);
                }
            });
        }
    }

    /// `onpointermove` on the track.
    pub fn on_pointer_move(&self, event: &Event<PointerData>) {
        if !self.dragging() || self.disabled {
            return;
        }
        self.commit_from_x(event.client_coordinates().x);
    }

    /// `onpointerup` / `onpointercancel` / `onlostpointercapture` on the track.
    pub fn end_drag(&self, event: &Event<PointerData>) {
        if !platform::release_pointer(event) {
            platform::release_pointer_selector(&self.track_selector(), event.pointer_id());
        }
        let mut dragging = self.dragging;
        dragging.set(false);
    }

    /// `onkeydown` on the handle.
    ///
    /// Takes the whole event rather than its data because the handled keys have
    /// to call `prevent_default` — otherwise an arrow key scrolls the page as
    /// well as moving the value.
    pub fn on_key_down(&self, event: &Event<KeyboardData>) {
        if self.disabled {
            return;
        }
        let current = self.current();
        let stride = self.stride;
        let next = match event.key() {
            Key::ArrowRight | Key::ArrowUp => current + stride,
            Key::ArrowLeft | Key::ArrowDown => current - stride,
            Key::PageUp => current + stride * 10.0,
            Key::PageDown => current - stride * 10.0,
            Key::Home => self.lo,
            Key::End => self.hi,
            _ => return,
        };
        event.prevent_default();
        self.commit(next);
    }
}

/// See [`Slider`].
///
/// The source's `aria-*` and `role` props stay with the component: it owns the
/// markup, and Dioxus builds attributes from `rsx!` rather than from a returned
/// props object. What the component must set, transcribed from the source:
///
/// ```ignore
/// // track
/// div {
///     "data-beui-slider": slider.scope(),
///     onmounted: move |data| slider.measure(data),
///     onpointerdown: move |e| slider.on_pointer_down(&e),
///     onpointermove: move |e| slider.on_pointer_move(&e),
///     onpointerup: move |e| slider.end_drag(&e),
///     onpointercancel: move |e| slider.end_drag(&e),
///     onlostpointercapture: move |e| slider.end_drag(&e),
///     // handle
///     button {
///         "data-beui-slider-handle": slider.scope(),
///         role: "slider",
///         tabindex: if slider.disabled() { -1 } else { 0 },
///         "aria-valuemin": "{min}",
///         "aria-valuemax": "{max}",
///         "aria-valuenow": "{slider.current()}",
///         "aria-valuetext": slider.aria_value_text(),
///         onkeydown: move |e| slider.on_key_down(&e),  // e: Event<KeyboardData>
///     }
/// }
/// ```
pub fn use_slider(
    config: SliderConfig,
    value: Option<f64>,
    on_value_change: Option<EventHandler<f64>>,
) -> Slider {
    let internal = use_signal(|| config.default_value);
    let dragging = use_signal(|| false);
    let track = use_signal(|| None::<Rect>);
    let id = use_hook(platform::next_scope_id);

    // Collapse inverted or empty ranges and non-positive steps here, so that
    // percent, ticks and the keyboard maths never divide by zero or walk a NaN
    // grid.
    let lo = config.min;
    let hi = if config.max > config.min {
        config.max
    } else {
        config.min
    };
    let stride = if config.step > 0.0 { config.step } else { 1.0 };

    Slider {
        internal,
        dragging,
        track,
        controlled: value,
        lo,
        hi,
        stride,
        disabled: config.disabled,
        value_text: config.value_text,
        on_value_change,
        id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_up_the_range() {
        assert_eq!(snap_slider_value(37.0, 0.0, 100.0, 1.0), 37.0);
        assert_eq!(snap_slider_value(37.4, 0.0, 100.0, 1.0), 37.0);
        assert_eq!(snap_slider_value(37.5, 0.0, 100.0, 1.0), 38.0);
    }

    #[test]
    fn max_is_a_candidate_when_the_step_does_not_divide_the_range() {
        // 0..10 step 3: the grid is 0,3,6,9 and 10 is reachable by hand.
        assert_eq!(snap_slider_value(10.0, 0.0, 10.0, 3.0), 10.0);
        assert_eq!(snap_slider_value(9.4, 0.0, 10.0, 3.0), 9.0);
        // 9 is nearer than 10, so it still snaps to the grid.
        assert_eq!(snap_slider_value(8.8, 0.0, 10.0, 3.0), 9.0);
    }

    #[test]
    fn out_of_range_values_clamp() {
        assert_eq!(snap_slider_value(-50.0, 0.0, 100.0, 1.0), 0.0);
        assert_eq!(snap_slider_value(500.0, 0.0, 100.0, 1.0), 100.0);
    }

    #[test]
    fn an_empty_range_has_one_legal_point() {
        assert_eq!(snap_slider_value(5.0, 7.0, 7.0, 1.0), 7.0);
        // An inverted range collapses onto the minimum, as the source's
        // `!(max > min)` guard does.
        assert_eq!(snap_slider_value(5.0, 7.0, 3.0, 1.0), 7.0);
    }

    #[test]
    fn a_non_positive_step_only_clamps() {
        assert_eq!(snap_slider_value(37.4, 0.0, 100.0, 0.0), 37.4);
        assert_eq!(snap_slider_value(-3.0, 0.0, 100.0, -2.0), 0.0);
    }

    #[test]
    fn fractional_steps_land_on_readable_numbers() {
        assert_eq!(snap_slider_value(0.30000000000000004, 0.0, 1.0, 0.1), 0.3);
        assert_eq!(snap_slider_value(0.55, 0.0, 1.0, 0.1), 0.6);
        assert_eq!(snap_slider_value(0.31, 0.0, 1.0, 0.05), 0.3);
    }

    #[test]
    fn an_offset_grid_keeps_its_phase() {
        // 10..20 step 3: 10, 13, 16, 19 and the max 20.
        assert_eq!(snap_slider_value(14.0, 10.0, 20.0, 3.0), 13.0);
        assert_eq!(snap_slider_value(15.0, 10.0, 20.0, 3.0), 16.0);
        assert_eq!(snap_slider_value(19.9, 10.0, 20.0, 3.0), 20.0);
    }

    #[test]
    fn js_round_matches_math_round() {
        assert_eq!(js_round(0.5), 1.0);
        assert_eq!(js_round(-0.5), 0.0);
        assert_eq!(js_round(-1.5), -1.0);
        assert_eq!(js_round(2.4), 2.0);
    }

    #[test]
    fn rect_ratio_clamps_and_survives_a_zero_width_track() {
        let rect = Rect {
            left: 100.0,
            top: 0.0,
            width: 200.0,
            height: 8.0,
        };
        assert_eq!(rect.ratio_x(100.0), 0.0);
        assert_eq!(rect.ratio_x(200.0), 0.5);
        assert_eq!(rect.ratio_x(300.0), 1.0);
        // Past either end.
        assert_eq!(rect.ratio_x(0.0), 0.0);
        assert_eq!(rect.ratio_x(900.0), 1.0);
        let collapsed = Rect {
            left: 10.0,
            width: 0.0,
            ..Rect::default()
        };
        assert_eq!(collapsed.ratio_x(50.0), 0.0);
    }
}
