//! `layout` / `layoutId` — framed layout animation (FLIP).
//!
//! beUI animates layout with motion's `layout` (68 sites) and `layoutId` (40).
//! Both are the same algorithm, and it is not a CSS transition: the element's
//! box *jumps* to its new place in the DOM, and a transform that starts as the
//! inverse of that jump is animated back to identity, so the eye sees the
//! element travel while the layout is already settled.
//!
//! ```text
//! before           after the DOM change            after the pin
//! ┌──────┐         ┌──────┐                        ┌──────┐  ← pinned transform
//! │  A   │         │  A   │                        │  A   │     (visual: still A)
//! └──────┘         │  B   │                        │  B   │  ← layout: already B
//!                  └──────┘                        └──────┘  B is clipped away
//! ```
//!
//! The whole point is that the pin is applied *before the browser paints*, which
//! is why the two DOM operations live in `src/js/flip.js` and everything else
//! lives here — see that file for what has to happen in the same task.
//!
//! What this module owns:
//!
//! - [`use_flip`], which measures on `state` changes and on mount, computes the
//!   inverse box, and animates a single `0 → 1` progress value with
//!   [`dioxus_motion`] so the component re-renders with a shrinking transform.
//!   A progress value rather than the transform itself because the transform has
//!   to be *set* to the pin before the animation starts, and the animation API
//!   only animates from wherever it currently is.
//! - the registry that makes [`FlipOptions::layout_id`] work across elements:
//!   `layoutId` hands an identity from one DOM node to another (beUI renders its
//!   tab indicator inside the active trigger, so switching tabs unmounts the
//!   indicator in one button and mounts it in another), and the new node starts
//!   from the last box the old one was measured at.
//! - the interruption case: a flip that starts while one is still running begins
//!   from the element's *current visual* box — last layout box plus the
//!   remaining pin — so a fast second click continues the motion instead of
//!   snapping.
//!
//! Not ported yet, and worth knowing before a component leans on it:
//! `layout="position"` is [`FlipMode::Position`] (translate only, no scale, which
//! is what avoids distortion on an element with children), but motion's nested
//! projection (`layout` on a child of a scaled parent, which counter-scales the
//! child) is not implemented — a scaled parent still distorts its children.
//! beUI only relies on it inside `morphing-modal`'s cross-fade, which has its own
//! treatment in that port.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_motion::prelude::{AnimationConfig, AnimationManager, Time, TimeProvider};

use crate::ease::SPRING_LAYOUT;
use crate::platform::{self, Rect};
use crate::presence::Duration;

/// The listener body, ported next to the module that needs it.
const FLIP_JS: &str = include_str!("js/flip.js");

thread_local! {
    /// The last measured layout box per shared-layout id.
    ///
    /// An entry outlives the element that wrote it: that is the whole mechanism
    /// behind `layoutId`. The next element to claim the id reads the box its
    /// predecessor was last seen at and starts there.
    static LAYOUT_BOXES: RefCell<HashMap<String, Rect>> = RefCell::new(HashMap::new());
}

fn registry_get(id: &str) -> Option<Rect> {
    LAYOUT_BOXES.with(|boxes| boxes.borrow().get(id).copied())
}

fn registry_set(id: &str, rect: Rect) {
    LAYOUT_BOXES.with(|boxes| boxes.borrow_mut().insert(id.to_string(), rect));
}

/// Which parts of the box change animates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlipMode {
    /// Position and size — motion's `layout`.
    #[default]
    Both,
    /// Position only — motion's `layout="position"`, which leaves the element's
    /// own size alone and so cannot distort its children.
    Position,
}

/// Everything a `layout` element configures.
#[derive(Clone)]
pub struct FlipOptions {
    /// A shared-layout identity. Two elements that render the same id at
    /// different times are the same shape moving, which is motion's `layoutId`.
    pub layout_id: Option<String>,
    pub mode: FlipMode,
    /// The curve the pin decays along. beUI's layout animations are almost all
    /// [`SPRING_LAYOUT`](crate::ease::SPRING_LAYOUT).
    pub transition: AnimationConfig,
    /// A radius to counter-scale while the pin contains a scale, so a pill stays
    /// a pill mid-flight rather than a stretched capsule.
    pub border_radius: Option<f64>,
}

impl Default for FlipOptions {
    fn default() -> Self {
        Self {
            layout_id: None,
            mode: FlipMode::Both,
            transition: AnimationConfig::spring(SPRING_LAYOUT),
            border_radius: None,
        }
    }
}

impl FlipOptions {
    pub fn layout_id(mut self, id: impl Into<String>) -> Self {
        self.layout_id = Some(id.into());
        self
    }

    pub fn mode(mut self, mode: FlipMode) -> Self {
        self.mode = mode;
        self
    }

    /// Translate only — motion's `layout="position"`.
    pub fn position_only(self) -> Self {
        self.mode(FlipMode::Position)
    }

    pub fn transition(mut self, transition: AnimationConfig) -> Self {
        self.transition = transition;
        self
    }

    pub fn border_radius(mut self, radius: f64) -> Self {
        self.border_radius = Some(radius);
        self
    }
}

/// The inverse of a layout change, in the element's own coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pin {
    dx: f64,
    dy: f64,
    sx: f64,
    sy: f64,
}

impl Pin {
    const IDENTITY: Pin = Pin {
        dx: 0.0,
        dy: 0.0,
        sx: 1.0,
        sy: 1.0,
    };

    fn is_identity(self) -> bool {
        self == Pin::IDENTITY
    }

    /// Whether what is left of the pin is invisible.
    ///
    /// Motion snaps to the target when the remaining travel stops being visible
    /// rather than integrating the spring's asymptotic tail, and so does this:
    /// the predicate decides that a flip is over, and it is the same one
    /// [`Flip::transform`] uses to stop emitting a transform.
    ///
    /// Relative to the travel, with a sub-pixel floor, because that is the scale
    /// the numbers actually live at: a spring is allowed to stop 0.2% short
    /// (measured: the tabs spring stops with 0.2px of a 93px slide left), and
    /// 0.2% of a 900px move is still under 2px.
    fn is_visually_settled(self, progress: f64) -> bool {
        let settle =
            |value: f64, travel: f64, floor: f64| value.abs() < floor.max(travel.abs() * 0.005);
        let pin = self.decay(progress);
        settle(pin.dx, self.dx, 0.5)
            && settle(pin.dy, self.dy, 0.5)
            && settle(pin.sx - 1.0, self.sx - 1.0, 0.002)
            && settle(pin.sy - 1.0, self.sy - 1.0, 0.002)
    }

    /// What is left of the pin after `progress` of the animation.
    ///
    /// A spring is linear in its initial displacement and so is a bezier tween,
    /// which is why animating one scalar `0 → 1` and scaling the pin by it is
    /// the same curve as animating the pin itself — and why the value the pin
    /// starts from can simply be replaced on an interruption.
    fn decay(self, progress: f64) -> Pin {
        let remaining = 1.0 - progress;
        Pin {
            dx: self.dx * remaining,
            dy: self.dy * remaining,
            sx: 1.0 + (self.sx - 1.0) * remaining,
            sy: 1.0 + (self.sy - 1.0) * remaining,
        }
    }
}

/// What the script reports back.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
struct Measurement {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    dx: f64,
    dy: f64,
    sx: f64,
    sy: f64,
    moved: bool,
}

impl Measurement {
    fn rect(self) -> Rect {
        Rect {
            left: self.left,
            top: self.top,
            width: self.width,
            height: self.height,
        }
    }

    fn pin(self) -> Pin {
        Pin {
            dx: self.dx,
            dy: self.dy,
            sx: self.sx,
            sy: self.sy,
        }
    }
}

/// The hook's handle.
#[derive(Clone)]
pub struct Flip {
    scope: u32,
    /// `0 → 1`, one unit of "pin still to decay".
    progress: dioxus_motion::prelude::MotionHandle<f32>,
    pin: Signal<Pin>,
    layout: Signal<Option<Rect>>,
    transition: AnimationConfig,
    layout_id: Option<Rc<str>>,
    mode: FlipMode,
    border_radius: Option<f64>,
}

impl Flip {
    /// The `data-beui-flip` value the element must carry.
    pub fn scope(&self) -> String {
        self.scope.to_string()
    }

    /// The selector the script finds the element by.
    pub fn selector(&self) -> String {
        format!("[data-beui-flip=\"{}\"]", self.scope)
    }

    /// The transform declarations the element has to carry, pinned or at rest.
    ///
    /// Never empty once a flip has run, and that is not a style preference: the
    /// web interpreter's `style` writer keeps imperative inline-style writes
    /// alive by *restoring* any property the new attribute value does not
    /// mention. The script that pins an element writes `style.transform`
    /// directly, so a later `style` that says nothing about transform would see
    /// the last transform restored — the element would keep its frozen spring
    /// value for the rest of its life. `transform: none` is how a flip says it
    /// is finished, and it is what removes the transform, since `none` is not a
    /// containing block and not a stacking context.
    pub fn transform(&self) -> String {
        if (self.layout)().is_none() {
            // Nothing has been measured yet: leave the element exactly as the
            // component wrote it.
            //
            // This has to be a *tracked* read even though the value is only used
            // as a gate. `transform` is the only place the render touches the
            // flip's state, so an untracked read here — `peek` — would leave the
            // component subscribed to nothing at all: the measurement would land,
            // the pin would be set, the animation would run, and the element
            // would keep the transform the script pinned it with, forever.
            return String::new();
        }
        let pin = (self.pin)();
        let progress = f64::from(self.progress_value().clamp(0.0, 1.0));
        let mut css = String::from("transform-origin:top left;");
        if pin.is_identity() || pin.is_visually_settled(progress) {
            css.push_str("transform:none");
            return css;
        }
        let pin = pin.decay(progress);
        css.push_str(&format!(
            "transform:translateX({}px) translateY({}px) scaleX({}) scaleY({})",
            pin.dx, pin.dy, pin.sx, pin.sy
        ));
        // motion counter-scales the radius so a scaled pill still reads as a
        // pill. One elliptical radius rather than motion's per-corner
        // correction; a clamped `9999px / …` is a pill at any scale, which is
        // the case beUI actually uses.
        if let Some(radius) = self.border_radius {
            if (pin.sx - 1.0).abs() > 0.0005 || (pin.sy - 1.0).abs() > 0.0005 {
                css.push_str(&format!(
                    ";border-radius:{}px / {}px",
                    radius / pin.sx,
                    radius / pin.sy
                ));
            }
        }
        css
    }

    /// The element's own style with [`Flip::transform`] appended.
    pub fn style_with(&self, base: &str) -> String {
        let transform = self.transform();
        let base = base.trim().trim_end_matches(';');
        match (base.is_empty(), transform.is_empty()) {
            (true, true) => String::new(),
            (true, false) => transform,
            (false, true) => base.to_string(),
            (false, false) => format!("{base};{transform}"),
        }
    }

    /// Whether the element has been measured at least once.
    pub fn measured(&self) -> bool {
        (self.layout)().is_some()
    }

    /// Whether the element still has a visible pin to decay.
    pub fn is_settling(&self) -> bool {
        let pin = *self.pin.peek();
        !pin.is_identity() && !pin.is_visually_settled(f64::from(self.progress_value()))
    }

    /// The progress value, as a tracked read.
    ///
    /// This is what makes the element animate at all: reading the motion during
    /// render is what re-renders the component once per frame with a smaller
    /// transform. Reading it *untracked* here would leave the transform at its
    /// pinned value forever — the animation would run, in a store nobody reads.
    fn progress_value(&self) -> f32 {
        use dioxus::prelude::ReadableExt;
        *self.progress.current().read()
    }

    /// The same value without subscribing, for the commit effect.
    ///
    /// The effect works out where the element currently *looks*; subscribing
    /// there would re-run it on every frame of every animation.
    fn progress_value_untracked(&self) -> f32 {
        use dioxus::prelude::ReadableExt;
        *self.progress.current().peek()
    }

    /// Where the element looks right now: its layout box with the remaining pin
    /// applied.
    fn visual_rect(&self, layout: Rect) -> Rect {
        let pin = *self.pin.peek();
        if pin.is_identity() {
            return layout;
        }
        let pin = pin.decay(f64::from(self.progress_value_untracked().clamp(0.0, 1.0)));
        Rect {
            left: layout.left + pin.dx,
            top: layout.top + pin.dy,
            width: layout.width * pin.sx,
            height: layout.height * pin.sy,
        }
    }

    /// Measure, pin and animate.
    ///
    /// Fired on mount and on every change of the caller's `state`; the measure
    /// and the pin themselves happen in [`FLIP_JS`].
    pub fn commit(&self) {
        // Every read here is untracked on purpose. `commit` runs inside an
        // effect, and it also *writes* both of these signals: a tracked read
        // would subscribe the effect to its own output, and the effect would
        // re-run for as long as it kept writing — an eval, a render and a
        // measure per iteration, with no animation in it at all.
        //
        // Where to start from: this element's own last box, or — for the first
        // commit of a `layoutId` that someone else used to own — the box that
        // owner was last measured at.
        let previous = (*self.layout.peek())
            .or_else(|| self.layout_id.as_deref().and_then(registry_get))
            .map(|rect| self.visual_rect(rect));

        let from = previous.map_or_else(
            || "null".to_string(),
            |rect| serde_json::to_string(&rect).unwrap_or_else(|_| "null".to_string()),
        );
        let script = FLIP_JS
            .replace("__BEUI_SELECTOR__", &platform::js_string(&self.selector()))
            .replace("__BEUI_FROM__", &from)
            .replace(
                "__BEUI_SCALE__",
                if self.mode == FlipMode::Both {
                    "true"
                } else {
                    "false"
                },
            );

        let Some(mut eval) = platform::eval(&script) else {
            return;
        };

        let mut layout = self.layout;
        let mut pin = self.pin;
        let mut progress = self.progress;
        let transition = self.transition.clone();
        let layout_id = self.layout_id.clone();

        spawn(async move {
            let Ok(Some(measurement)) = eval.recv::<Option<Measurement>>().await else {
                return;
            };
            let rect = measurement.rect();
            layout.set(Some(rect));
            if let Some(id) = &layout_id {
                registry_set(id, rect);
            }
            if measurement.moved {
                // The animation API only animates from the current value, so the
                // pin has to be the *starting* value: `reset` puts progress back
                // to 0 and the tween or spring runs it to 1, which the render
                // turns into `pin * (1 - progress)`.
                //
                // Restart, then pin, in that order: the settle effect below
                // clears the pin when the animation is not running, and a moment
                // with the new pin and no animation would look settled.
                progress.reset();
                let _ = progress.animate_to(1.0, transition);
                pin.set(measurement.pin());
                // Watchdog. A spring stops when its own convergence test says
                // so, and that test can stop it a visible fraction of a
                // millimetre short — measured on the tabs spring: 0.2px to 1.2px
                // of a 93px slide, and 1.1px of a 208px growth — after which
                // `running` is false and the value stops changing, so nothing
                // will ever notify a watcher again. Motion snaps the value onto
                // the target when an animation completes; this does the same,
                // and does not depend on the library saying anything.
                spawn(async move {
                    let mut previous = f32::NAN;
                    let mut still = 0u32;
                    for _ in 0..40 {
                        if Time::delay(Duration::from_millis(50)).await.is_err() {
                            return;
                        }
                        let current = {
                            use dioxus::prelude::ReadableExt;
                            *progress.current().peek()
                        };
                        let pin_now = *pin.peek();
                        if pin_now.is_identity() {
                            return;
                        }
                        if pin_now.is_visually_settled(f64::from(current)) {
                            pin.set(Pin::IDENTITY);
                            let mut handle = progress;
                            handle.stop();
                            return;
                        }
                        // Frozen: the driver is still nominally running but the
                        // value has stopped moving.
                        if (current - previous).abs() < 1.0e-4 {
                            still += 1;
                            if still >= 4 {
                                pin.set(Pin::IDENTITY);
                                let mut handle = progress;
                                handle.stop();
                                return;
                            }
                        } else {
                            still = 0;
                        }
                        previous = current;
                    }
                });
            } else if !(pin.peek().is_identity()) {
                pin.set(Pin::IDENTITY);
            }
        });
    }
}

/// Animate an element's layout changes — motion's `layout` and `layoutId`.
///
/// `state` is whatever changes when the layout changes: the selected tab, an
/// expanded flag, a size. The hook cannot see the layout itself, so this is how
/// it learns that there is something to measure — the same thing motion learns
/// from React re-rendering the element.
///
/// The returned handle's [`Flip::transform`] has to be part of the element's
/// style, and the element has to carry `data-beui-flip`:
///
/// ```ignore
/// let flip = use_flip(selected, FlipOptions::default().layout_id("indicator").border_radius(9999.0));
/// rsx! {
///     span {
///         "data-beui-flip": flip.scope(),
///         style: flip.style_with("inset:0"),
///     }
/// }
/// ```
pub fn use_flip<T: Clone + PartialEq + 'static>(state: T, options: FlipOptions) -> Flip {
    let scope = use_hook(platform::next_scope_id);
    let progress =
        dioxus_motion::prelude::use_motion(0.0f32).expect("0.0 is a finite animation value");
    let pin = use_signal(|| Pin::IDENTITY);
    let layout = use_signal(|| None::<Rect>);

    let flip = Flip {
        scope,
        progress,
        pin,
        layout,
        transition: options.transition,
        layout_id: options.layout_id.map(Rc::from),
        mode: options.mode,
        border_radius: options.border_radius,
    };

    // Mount, and every `state` change after it.
    let commit = flip.clone();
    use_effect(use_reactive((&state,), move |_| commit.commit()));

    // Drop the pin once the animation has settled.
    //
    // A spring stops near its target, not on it, so without this the element
    // would keep a sub-pixel transform for the rest of its life — invisible, but
    // a transform is a containing block for fixed-position descendants and a
    // stacking context, and the idle element in the source has neither. Watches
    // `running` so it fires on the frame the animation stops rather than on a
    // progress threshold that a damping ratio could miss.
    let settle = flip.clone();
    use_effect(move || {
        let running = *settle.progress.running().read();
        let progress = f64::from(settle.progress_value());
        let pin = *settle.pin.peek();
        if pin.is_identity() {
            return;
        }
        if running && !pin.is_visually_settled(progress) {
            return;
        }
        let mut handle = settle.pin;
        handle.set(Pin::IDENTITY);
        if running {
            // The remaining travel is invisible, so let the spring go rather
            // than re-rendering the tail of it for another second.
            let mut progress = settle.progress;
            progress.stop();
        }
    });

    flip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pin_decays_to_identity() {
        let pin = Pin {
            dx: 120.0,
            dy: -40.0,
            sx: 2.0,
            sy: 0.5,
        };
        assert_eq!(pin.decay(1.0), Pin::IDENTITY);
        let half = pin.decay(0.5);
        assert_eq!(half.dx, 60.0);
        assert_eq!(half.dy, -20.0);
        assert_eq!(half.sx, 1.5);
        assert_eq!(half.sy, 0.75);
        assert_eq!(pin.decay(0.0), pin);
    }

    #[test]
    fn an_interrupted_flip_starts_from_where_it_looks() {
        // Layout box at 100, pin of +200 with half of it left to decay: the
        // element looks like it is at 200, and that is where the next flip has
        // to start.
        let layout = Rect {
            left: 100.0,
            top: 10.0,
            width: 50.0,
            height: 20.0,
        };
        let pin = Pin {
            dx: 200.0,
            dy: 0.0,
            sx: 1.0,
            sy: 1.0,
        };
        let visual = pin.decay(0.5);
        assert_eq!(layout.left + visual.dx, 200.0);
        // Size follows the scale the same way.
        let stretched = Pin {
            dx: 0.0,
            dy: 0.0,
            sx: 2.0,
            sy: 1.0,
        }
        .decay(0.5);
        // Halfway between the layout width (50) and the pinned visual width
        // (100): the element was pinned at twice its new size.
        assert_eq!(layout.width * stretched.sx, 75.0);
    }

    #[test]
    fn pin_identity_is_the_default() {
        assert!(Pin::IDENTITY.is_identity());
        assert!(Pin::IDENTITY.decay(0.5).is_identity());
        assert!(!Pin {
            dx: 0.5,
            ..Pin::IDENTITY
        }
        .is_identity());
    }
}
