//! `drag` — motion's drag gesture.
//!
//! beUI drags six things: a sheet down to dismiss, a toast sideways to throw it
//! away, a slider ruler, a slide-to-confirm handle, a swipeable row and a
//! lightbox. They all use the same three pieces, and each piece is a line-by-line
//! port rather than an approximation:
//!
//! - **Velocity**, from `MotionValue.getVelocity()` in motion 12.43.0: the
//!   difference between this frame's value and the previous frame's, divided by
//!   the time between the two frames, and zero if the value has not moved for
//!   [`MAX_VELOCITY_DELTA`]. This is the number the toast's `|v| > 520`
//!   threshold and every release animation are given.
//! - **Constraints and elasticity**, from `gestures/drag/utils/constraints.mjs`:
//!   inside the range the offset is the pointer's, and beyond it the excess is
//!   *mixed* toward the bound by the elastic factor — `0.4` for the sheet's
//!   downward direction, `0.02` upward, `0.18` for the toast's two.
//! - **The release**, from `VisualElementDragControls.startAnimation`: momentum
//!   is an inertia throw (`power 0.8`, `timeConstant 750`, `restDelta 1`), and
//!   anything that would end outside the constraints hands over to the boundary
//!   spring (`bounceStiffness 200`, `bounceDamping 40` — or `1e6`/`1e7` when
//!   elasticity is off, which is motion's way of making it instant).
//!
//! Two things are simplified, and both are noted where they bite:
//!
//! 1. motion's `dragConstraints` can be another element, and are then solved in
//!    layout-box space. Every beUI use passes a plain offset range
//!    (`{left: 0, right: 0}`, `{left: -width, right: width}`), so this takes the
//!    offset range directly — [`DragConstraints`] is that range.
//! 2. the inertia generator's crossing into the boundary spring is decided up
//!    front (throw outside the range → spring) instead of mid-flight. For the
//!    constrained cases beUI has, the crossing happens on the first frame.
//!
//! Why the offset is not a `dioxus-motion` value: a drag has to *set* its value
//! to the pointer on every move, and `MotionHandle` has no setter — it only
//! animates from wherever it already is. So the drag offset is a plain signal,
//! and the release animation runs on a motion whose value is a *delta* from where
//! the drag left off ([`Axes`] + [`Phase`]).

use dioxus::prelude::*;
use dioxus_motion::prelude::{
    AnimationConfig, AnimationManager, Spring, TimeProvider, Transform, Tween,
};
use dioxus_motion::{Duration, Time};

use crate::platform;

/// `MAX_VELOCITY_DELTA` from `MotionValue.getVelocity()`: a value that has not
/// moved for longer than this has no velocity, not a stale one.
pub const MAX_VELOCITY_DELTA: f64 = 30.0;

/// `velocityPerSecond` from `motion-utils`.
pub fn velocity_per_second(velocity: f64, frame_duration: f64) -> f64 {
    if frame_duration == 0.0 {
        0.0
    } else {
        velocity * (1000.0 / frame_duration)
    }
}

/// Which axis (or axes) an element can be dragged along.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DragAxis {
    /// motion's `drag="x"`.
    X,
    /// motion's `drag="y"`.
    Y,
    /// motion's `drag` (both).
    #[default]
    Both,
}

impl DragAxis {
    fn includes(self, axis: Axis) -> bool {
        matches!(
            (self, axis),
            (DragAxis::Both, _) | (DragAxis::X, Axis::X) | (DragAxis::Y, Axis::Y)
        )
    }
}

/// The two axes, so per-axis work is a loop rather than a pair of copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

/// One axis of a constraint range. `None` is unbounded, as in motion.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DragRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl DragRange {
    pub const FREE: DragRange = DragRange {
        min: None,
        max: None,
    };

    /// motion's `{left, right}` / `{top, bottom}` written as a range.
    pub fn new(min: Option<f64>, max: Option<f64>) -> Self {
        Self { min, max }
    }

    /// A range that only allows one position, motion's `dragSnapToOrigin`.
    pub fn pinned(value: f64) -> Self {
        Self {
            min: Some(value),
            max: Some(value),
        }
    }

    pub fn is_free(self) -> bool {
        self.min.is_none() && self.max.is_none()
    }
}

/// motion's `dragConstraints`, as an offset range per axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DragConstraints {
    pub x: DragRange,
    pub y: DragRange,
}

impl DragConstraints {
    pub fn range(self, axis: Axis) -> DragRange {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
        }
    }
}

/// motion's `dragElastic`: how much of the excess beyond a bound the element
/// actually takes. A number applies to both sides; the per-side form is what the
/// sheet uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragElastic {
    pub min: f64,
    pub max: f64,
}

/// `defaultElastic` from `constraints.mjs`.
pub const DEFAULT_ELASTIC: f64 = 0.35;

impl Default for DragElastic {
    fn default() -> Self {
        Self {
            min: DEFAULT_ELASTIC,
            max: DEFAULT_ELASTIC,
        }
    }
}

impl DragElastic {
    pub const fn uniform(value: f64) -> Self {
        Self {
            min: value,
            max: value,
        }
    }
}

/// `applyConstraints` from `constraints.mjs`.
///
/// Inside the range the point is the pointer's. Outside it, motion mixes the
/// point toward the bound by the elastic factor — so an elastic of `0.4` means
/// the element keeps moving, but only 40% as fast as the finger.
pub fn apply_constraints(point: f64, range: DragRange, elastic: DragElastic) -> f64 {
    if let Some(min) = range.min {
        if point < min {
            return mix_number(min, point, elastic.min);
        }
    }
    if let Some(max) = range.max {
        if point > max {
            return mix_number(max, point, elastic.max);
        }
    }
    point
}

/// `mixNumber(from, to, p)`.
pub fn mix_number(from: f64, to: f64, progress: f64) -> f64 {
    from + (to - from) * progress
}

/// `shouldDrag` from `VisualElementDragControls`.
pub fn should_drag(axis: Axis, drag: DragAxis, locked: Option<Axis>) -> bool {
    drag.includes(axis) && (locked.is_none() || locked == Some(axis))
}

/// `getCurrentDirection` from `VisualElementDragControls`: the axis of the larger
/// offset, once it clears the lock threshold.
pub fn get_current_direction(x: f64, y: f64, lock_threshold: f64) -> Option<Axis> {
    if y.abs() > lock_threshold {
        Some(Axis::Y)
    } else if x.abs() > lock_threshold {
        Some(Axis::X)
    } else {
        None
    }
}

/// The inertia generator's tunables, from `VisualElementDragControls.startAnimation`
/// — the numbers are motion's, including the two "no bounce" extremes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragTransition {
    pub power: f64,
    pub time_constant: f64,
    pub rest_delta: f64,
}

impl Default for DragTransition {
    fn default() -> Self {
        Self {
            power: 0.8,
            time_constant: 750.0,
            rest_delta: 1.0,
        }
    }
}

/// `power * velocity`, motion's ideal friction target.
pub fn inertia_target(origin: f64, velocity: f64, power: f64) -> f64 {
    origin + power * velocity
}

/// How long the friction takes to come within `rest_delta` of its target.
///
/// motion integrates `target + (-amplitude * exp(-t / timeConstant))` frame by
/// frame and stops when the remaining delta is under `restDelta`; the same decay
/// is a curve, so the duration is `timeConstant * ln(amplitude / restDelta)`.
pub fn inertia_duration(amplitude: f64, time_constant: f64, rest_delta: f64) -> f64 {
    let amplitude = amplitude.abs();
    if amplitude <= rest_delta {
        return 0.0;
    }
    time_constant * (amplitude / rest_delta).ln()
}

/// The friction decay as an `easer`-shaped easing: `1 - (restDelta / amplitude)^u`.
///
/// The exponential in the generator is `exp(-t / timeConstant)`, which over the
/// duration above is exactly `(restDelta / amplitude)^u` — so a tween with this
/// easing samples the same curve the generator would.
pub fn inertia_easing(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let amplitude = c.abs();
    if amplitude < 1.0e-6 || d <= 0.0 {
        return b + c;
    }
    let u = (t / d).clamp(0.0, 1.0);
    b + c * (1.0 - (1.0 / amplitude).powf(u))
}

/// motion's boundary spring, as `Spring` values it hands straight to dioxus-motion.
pub fn bounce_spring(elastic: DragElastic) -> Spring {
    if elastic.min > 0.0 || elastic.max > 0.0 {
        Spring {
            stiffness: 200.0,
            damping: 40.0,
            mass: 1.0,
        }
    } else {
        Spring {
            stiffness: 1_000_000.0,
            damping: 10_000_000.0,
            mass: 1.0,
        }
    }
}

/// Everything a draggable element configures.
#[derive(Clone, Debug, PartialEq)]
pub struct DragOptions {
    pub axis: DragAxis,
    pub constraints: DragConstraints,
    pub elastic: DragElastic,
    /// motion's `dragMomentum`.
    pub momentum: bool,
    /// motion's `dragDirectionLock`: the drag picks one axis and stays on it.
    pub direction_lock: bool,
    pub transition: DragTransition,
}

impl Default for DragOptions {
    fn default() -> Self {
        Self {
            axis: DragAxis::Both,
            constraints: DragConstraints::default(),
            elastic: DragElastic::default(),
            momentum: true,
            direction_lock: false,
            transition: DragTransition::default(),
        }
    }
}

impl DragOptions {
    pub fn axis(mut self, axis: DragAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn constraints(mut self, constraints: DragConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// motion's `dragConstraints={{ left, right, top, bottom }}`, as ranges.
    pub fn range(mut self, x: DragRange, y: DragRange) -> Self {
        self.constraints = DragConstraints { x, y };
        self
    }

    pub fn elastic(mut self, elastic: DragElastic) -> Self {
        self.elastic = elastic;
        self
    }

    /// motion's `dragElastic={{ top, bottom }}` on a vertical drag.
    pub fn elastic_y(mut self, min: f64, max: f64) -> Self {
        self.elastic = DragElastic { min, max };
        self
    }

    pub fn without_momentum(mut self) -> Self {
        self.momentum = false;
        self
    }

    pub fn direction_lock(mut self) -> Self {
        self.direction_lock = true;
        self
    }
}

/// Where an axis is in its gesture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Dragging,
    /// The release animation is running; what is rendered is `base + motion`.
    Settling,
}

/// One axis's worth of `MotionValue` frame bookkeeping.
///
/// motion keeps `updatedAt`, `prevUpdatedAt` and `prevFrameValue` on every value
/// and derives velocity from them (`getVelocity`): the change since the previous
/// *frame*, over the time between the two frames, zero when the value has been
/// still for longer than [`MAX_VELOCITY_DELTA`]. A frame, not a pointer move —
/// three moves delivered in one frame are one frame of motion.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sample {
    value: f64,
    updated_at: f64,
    previous_frame_value: f64,
    previous_updated_at: f64,
}

impl Sample {
    /// The state a value is in when the gesture starts: at rest, *now*.
    ///
    /// The timestamps matter. motion's `MotionValue` starts with `updatedAt` at
    /// the last frame it was written on, so the first move's velocity is
    /// measured from the frame before it. Starting from zero instead would make
    /// the first move of a drag report its whole displacement divided by the
    /// velocity clamp — a flick with one move would throw at 30x its speed.
    fn at_rest(now: f64) -> Self {
        Self {
            value: 0.0,
            updated_at: now,
            previous_frame_value: 0.0,
            previous_updated_at: now,
        }
    }

    /// `updateAndNotify`.
    fn update(&mut self, value: f64, now: f64) {
        if self.updated_at != now {
            self.previous_frame_value = self.value;
            self.previous_updated_at = self.updated_at;
        }
        self.value = value;
        self.updated_at = now;
    }

    /// `getVelocity`.
    fn velocity(&self, now: f64) -> f64 {
        if now - self.updated_at > MAX_VELOCITY_DELTA {
            return 0.0;
        }
        let delta = (self.updated_at - self.previous_updated_at).min(MAX_VELOCITY_DELTA);
        velocity_per_second(self.value - self.previous_frame_value, delta)
    }
}

/// The offsets and velocities of a drag, one entry per axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Axes {
    pub x: f64,
    pub y: f64,
}

impl Axes {
    pub const ZERO: Axes = Axes { x: 0.0, y: 0.0 };

    pub fn get(self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
        }
    }

    fn set(&mut self, axis: Axis, value: f64) {
        match axis {
            Axis::X => self.x = value,
            Axis::Y => self.y = value,
        }
    }
}

impl std::ops::Add for Axes {
    type Output = Axes;
    fn add(self, other: Axes) -> Axes {
        Axes {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

/// The hook's handle.
#[derive(Clone, Copy)]
pub struct Drag {
    scope: u32,
    options: Signal<DragOptions>,
    /// Where the drag left the element, before the release animation's delta.
    base: Signal<Axes>,
    /// The release animation's delta, added to [`Drag::base`] while settling.
    settle: dioxus_motion::prelude::MotionHandle<Transform>,
    phase: Signal<Phase>,
    start: Signal<Axes>,
    /// The pointer's own displacement, before constraints — motion's
    /// `info.offset`, which is what beUI's thresholds read.
    raw: Signal<Axes>,
    /// motion's per-`MotionValue` frame bookkeeping, one entry per axis.
    samples: Signal<(Sample, Sample)>,
    velocity: Signal<Axes>,
    locked: Signal<Option<Axis>>,
    on_drag_end: Option<EventHandler<DragEnd>>,
}

/// What `onDragEnd` receives: motion's `(event, info)`.
///
/// `offset` is `info.offset`, the pointer's displacement with nothing applied —
/// the toast's `|offset.x| > 72` and the sheet's `offset > threshold` read this
/// one. `value` is where the element actually ended up, constraints and
/// elasticity included, which is the swipeable row's `x.get()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragEnd {
    pub offset: Axes,
    pub value: Axes,
    pub velocity: Axes,
}

impl Drag {
    /// The `data-beui-drag` value the element must carry.
    pub fn scope(&self) -> String {
        self.scope.to_string()
    }

    pub fn selector(&self) -> String {
        format!("[data-beui-drag=\"{}\"]", self.scope)
    }

    /// Whether a gesture is in progress — motion's `whileDrag`.
    pub fn is_dragging(&self) -> bool {
        (self.phase)() == Phase::Dragging
    }

    /// Whether the release animation is still running.
    pub fn is_settling(&self) -> bool {
        (self.phase)() == Phase::Settling
    }

    /// The offset to render right now.
    ///
    /// Reads the release motion while it runs, and the base while a gesture is
    /// in flight: both are what make the element follow the pointer and then
    /// re-render once per frame of the spring or friction.
    pub fn offset(&self) -> Axes {
        let value = {
            let handle = self.settle.current();
            let value = ReadableExt::read(&handle);
            *value
        };
        // `base` is the whole offset while dragging, so a *tracked* read here is
        // what re-renders the drag's owner per pointer move. Reading it untracked
        // (the handler-side path below) leaves the element frozen until something
        // else happens to re-render.
        let base = *ReadableExt::read(&self.base);
        self.offset_from(base, value)
    }

    /// The same offset without subscribing, for the event handlers.
    ///
    /// Handlers run outside a render, but a tracked read there still registers a
    /// subscriber on whichever context is current, and a drag would then be
    /// re-rendering its component for reasons of its own.
    fn offset_untracked(&self) -> Axes {
        let value = {
            let handle = self.settle.current();
            let value = ReadableExt::peek(&handle);
            *value
        };
        let base = *self.base.peek();
        self.offset_from(base, value)
    }

    fn offset_from(&self, base: Axes, settle: Transform) -> Axes {
        match *self.phase.peek() {
            Phase::Settling => Axes {
                x: base.x + f64::from(settle.x),
                y: base.y + f64::from(settle.y),
            },
            _ => base,
        }
    }

    /// The pointer's velocity at the last move, in pixels per second.
    pub fn velocity(&self) -> Axes {
        (self.velocity)()
    }

    /// Watch the settle motion and fold it into the base once it stops.
    ///
    /// A spring stops near its target rather than on it (measured elsewhere in
    /// this port: 0.2px of a 93px slide), so without this the element would keep
    /// a sub-pixel transform and stay in the settling phase for good. Folding the
    /// delta into `base` and clearing the motion leaves it exactly where it was
    /// released to.
    fn watch_settle(&self) {
        let this = *self;
        spawn(async move {
            let mut previous = f32::NAN;
            let mut still = 0u32;
            for _ in 0..120 {
                if Time::delay(Duration::from_millis(50)).await.is_err() {
                    return;
                }
                if *this.phase.peek() != Phase::Settling {
                    return;
                }
                let running = *this.settle.running().read();
                let current = {
                    let handle = this.settle.current();
                    let value = ReadableExt::peek(&handle);
                    value.x
                };
                if (current - previous).abs() < 1.0e-4 {
                    still += 1;
                } else {
                    still = 0;
                }
                previous = current;
                // Twenty polls of no movement is a second of nothing at all: a
                // spring that is still converging will have moved by then.
                if running && still < 20 {
                    continue;
                }
                // Stopped, or frozen on a value that will not move again.
                let settled = this.offset_untracked();
                let mut base = this.base;
                base.set(settled);
                let mut settle = this.settle;
                settle.reset();
                let mut phase = this.phase;
                phase.set(Phase::Idle);
                return;
            }
        });
    }

    /// Animate from where the element is to `target`, on the same release path.
    ///
    /// motion leaves this to the component: a swipeable row animates its own `x`
    /// to the open position, a sheet to its nearest snap point. Here it is one
    /// call because the release motion is already the thing that adds to the
    /// offset.
    pub fn settle_to(&self, target: Axes, config: AnimationConfig) {
        let current = self.offset_untracked();
        let mut base = self.base;
        base.set(current);
        let mut settle = self.settle;
        settle.reset();
        let delta = Axes {
            x: target.x - current.x,
            y: target.y - current.y,
        };
        let mut phase = self.phase;
        if delta == Axes::ZERO {
            phase.set(Phase::Idle);
            return;
        }
        phase.set(Phase::Settling);
        let _ = settle.animate_to(Transform::from(delta), config);
        self.watch_settle();
    }

    /// Put the element back at rest, cancelling anything in flight.
    ///
    /// Back to zero, which is where motion's own `dragSnapToOrigin` puts it; a
    /// component that wants to stay where it was released simply never calls it.
    pub fn reset(&self) {
        let mut settle = self.settle;
        settle.reset();
        let mut base = self.base;
        base.set(Axes::ZERO);
        let mut phase = self.phase;
        phase.set(Phase::Idle);
    }

    /// `onpointerdown`.
    pub fn on_pointer_down(&self, event: &Event<PointerData>) {
        if !platform::capture_pointer(event) {
            platform::capture_pointer_selector(&self.selector(), event.pointer_id());
        }

        // Pick the element up where it actually is: whatever the last release
        // left on the settle motion becomes part of the base, and the motion
        // goes back to zero. Without this the element would jump back to where
        // the previous drag let go of it.
        let current = self.offset_untracked();
        let mut base = self.base;
        base.set(current);
        let mut settle = self.settle;
        settle.reset();

        let point = event.client_coordinates();
        let mut start = self.start;
        start.set(Axes {
            x: point.x,
            y: point.y,
        });
        let mut locked = self.locked;
        locked.set(None);
        let mut velocity = self.velocity;
        velocity.set(Axes::ZERO);
        let mut raw = self.raw;
        raw.set(Axes::ZERO);
        let mut samples = self.samples;
        let now = platform::now_ms();
        samples.set((Sample::at_rest(now), Sample::at_rest(now)));
        let mut phase = self.phase;
        phase.set(Phase::Dragging);
    }

    /// `onpointermove`.
    pub fn on_pointer_move(&self, event: &Event<PointerData>) {
        let now = platform::now_ms();
        if (self.phase)() != Phase::Dragging {
            return;
        }
        let point = event.client_coordinates();
        let start = (self.start)();
        let raw = Axes {
            x: point.x - start.x,
            y: point.y - start.y,
        };

        // motion's direction lock: the first axis to clear 10px wins, and the
        // drag stays on it.
        let options = (self.options)();
        if options.direction_lock && (self.locked)().is_none() {
            let direction = get_current_direction(raw.x, raw.y, 10.0);
            let mut locked = self.locked;
            locked.set(direction);
            if direction.is_none() {
                return;
            }
        }
        let locked = (self.locked)();
        let mut raw_signal = self.raw;
        raw_signal.set(raw);

        let mut next = Axes::ZERO;
        for axis in [Axis::X, Axis::Y] {
            if !should_drag(axis, options.axis, locked) {
                continue;
            }
            let constrained = apply_constraints(
                raw.get(axis),
                options.constraints.range(axis),
                elastic_for(&options.elastic, axis),
            );
            next.set(axis, constrained);
        }

        let mut base = self.base;
        base.set(next);

        // motion records the value against the frame clock and derives velocity
        // from the difference between frames, not between pointer moves: a
        // browser that delivers three moves in one frame is one frame of motion.
        let mut samples = self.samples;
        let mut pair = *samples.peek();
        pair.0.update(next.x, now);
        pair.1.update(next.y, now);
        samples.set(pair);
        let mut velocity = self.velocity;
        velocity.set(Axes {
            x: pair.0.velocity(now),
            y: pair.1.velocity(now),
        });
    }

    /// `onpointerup` / `onlostpointercapture`.
    pub fn on_pointer_up(&self, event: &Event<PointerData>) {
        self.release(event, true);
    }

    /// `onpointercancel`: the gesture is over but there is nothing to throw with.
    pub fn on_pointer_cancel(&self, event: &Event<PointerData>) {
        self.release(event, false);
    }

    fn release(&self, event: &Event<PointerData>, with_velocity: bool) {
        if *self.phase.peek() != Phase::Dragging {
            return;
        }
        if !platform::release_pointer(event) {
            platform::release_pointer_selector(&self.selector(), event.pointer_id());
        }

        let offset = *self.base.peek();
        let raw = *self.raw.peek();
        // Read the velocity *now*, not the one from the last move: motion's
        // `getVelocity` returns zero once the value has been still for longer
        // than `MAX_VELOCITY_DELTA`, so a drag that stops and then lets go is not
        // a fling. Without the re-read, a fling threshold would fire on a
        // gesture the user had already parked.
        let samples = *self.samples.peek();
        let now = platform::now_ms();
        let velocity = if with_velocity {
            Axes {
                x: samples.0.velocity(now),
                y: samples.1.velocity(now),
            }
        } else {
            Axes::ZERO
        };
        let options = self.options.peek().clone();

        // The release runs on a motion whose value is a *delta* from where the
        // drag left off, so the rendered offset is `offset + delta` from here on.
        let mut settle = self.settle;
        settle.reset();

        let mut target = Axes::ZERO;
        let mut config = AnimationConfig::tween(Duration::from_millis(1));
        for axis in [Axis::X, Axis::Y] {
            if !should_drag(axis, options.axis, *self.locked.peek()) {
                continue;
            }
            let (delta, axis_config) = release_axis(
                offset.get(axis),
                velocity.get(axis),
                options.constraints.range(axis),
                elastic_for(&options.elastic, axis),
                options.momentum,
                options.transition,
            );
            target.set(axis, delta);
            // One animation drives both axes. motion builds one per axis; the
            // two share their bounds in every beUI use, and dioxus-motion takes
            // a single transition for the pair.
            config = axis_config;
        }

        let mut phase = self.phase;
        if target == Axes::ZERO {
            phase.set(Phase::Idle);
        } else {
            phase.set(Phase::Settling);
            let _ = settle.animate_to(Transform::from(target), config);
            self.watch_settle();
            if with_velocity {
                // motion hands the release velocity to the spring that catches
                // the throw; dioxus-motion takes it after the animation starts.
                let _ = settle.set_velocity(Transform::from(velocity));
            }
        }

        if let Some(handler) = self.on_drag_end {
            handler.call(DragEnd {
                offset: raw,
                value: offset,
                velocity,
            });
        }
    }
}

/// See [`Drag`]: the element carries `data-beui-drag` and the offset has to be
/// part of its style or transform.
///
/// ```ignore
/// let drag = use_drag(
///     DragOptions::default()
///         .axis(DragAxis::Y)
///         .range(DragRange::FREE, DragRange::pinned(0.0))
///         .elastic_y(0.02, 0.4)
///         .without_momentum(),
///     Some(on_drag_end),
/// );
/// rsx! {
///     div {
///         "data-beui-drag": drag.scope(),
///         style: "transform: translateY({drag.offset().y}px)",
///         onpointerdown: move |e| drag.on_pointer_down(&e),
///         onpointermove: move |e| drag.on_pointer_move(&e),
///         onpointerup: move |e| drag.on_pointer_up(&e),
///         onpointercancel: move |e| drag.on_pointer_cancel(&e),
///         onlostpointercapture: move |e| drag.on_pointer_up(&e),
///     }
/// }
/// ```
///
/// `state` is not a parameter here the way it is on [`crate::flip::use_flip`]:
/// a drag is driven entirely by pointer events, so there is nothing for a
/// dependency to watch.
pub fn use_drag(options: DragOptions, on_drag_end: Option<EventHandler<DragEnd>>) -> Drag {
    let scope = use_hook(platform::next_scope_id);
    let options = use_signal(|| options);
    let base = use_signal(|| Axes::ZERO);
    let phase = use_signal(Phase::default);
    let start = use_signal(|| Axes::ZERO);
    let raw = use_signal(|| Axes::ZERO);
    let samples = use_signal(move || {
        let now = platform::now_ms();
        (Sample::at_rest(now), Sample::at_rest(now))
    });
    let velocity = use_signal(|| Axes::ZERO);
    let locked = use_signal(|| None::<Axis>);
    let settle = dioxus_motion::prelude::use_motion(Transform::identity())
        .expect("the identity transform is a finite animation value");

    Drag {
        scope,
        options,
        base,
        settle,
        phase,
        start,
        raw,
        samples,
        velocity,
        locked,
        on_drag_end,
    }
}

fn elastic_for(elastic: &DragElastic, axis: Axis) -> DragElastic {
    // motion resolves elasticity per axis from `{left, right}` / `{top, bottom}`
    // labels; beUI always passes one pair for the axis it drags on.
    let _ = axis;
    *elastic
}

/// What the release animation should add to the offset, and how.
fn release_axis(
    offset: f64,
    velocity: f64,
    range: DragRange,
    elastic: DragElastic,
    momentum: bool,
    transition: DragTransition,
) -> (f64, AnimationConfig) {
    let inside = |value: f64| {
        range.min.is_none_or(|min| value >= min) && range.max.is_none_or(|max| value <= max)
    };
    let nearest_bound = |value: f64| match (range.min, range.max) {
        (Some(min), Some(max)) => {
            if (min - value).abs() < (max - value).abs() {
                min
            } else {
                max
            }
        }
        (Some(min), None) => min,
        (None, Some(max)) => max,
        (None, None) => value,
    };

    let target = if momentum {
        inertia_target(offset, velocity, transition.power)
    } else {
        offset
    };

    if !inside(target) || !inside(offset) {
        // Outside the bounds: motion's inertia hands straight over to the
        // boundary spring. The spring is dioxus-motion's, with the stiffness and
        // damping motion uses.
        let bound = nearest_bound(offset);
        let spring = bounce_spring(elastic);
        return (
            bound - offset,
            AnimationConfig::spring(spring).with_epsilon(0.5),
        );
    }

    if target == offset {
        return (0.0, AnimationConfig::tween(Duration::from_millis(0)));
    }

    let delta = target - offset;
    let amplitude = delta.abs();
    let duration = inertia_duration(amplitude, transition.time_constant, transition.rest_delta);
    if duration <= 0.0 {
        return (delta, AnimationConfig::tween(Duration::from_millis(0)));
    }
    let millis = duration.round().max(1.0) as u64;
    (
        delta,
        AnimationConfig::new(dioxus_motion::prelude::AnimationMode::Tween(
            Tween::new(Duration::from_millis(millis)).with_easing(inertia_easing),
        )),
    )
}

impl From<Axes> for Transform {
    fn from(axes: Axes) -> Self {
        Transform {
            x: axes.x as f32,
            y: axes.y as f32,
            ..Transform::identity()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_is_per_second_over_the_frame_delta() {
        assert_eq!(velocity_per_second(10.0, 16.0), 625.0);
        assert_eq!(velocity_per_second(10.0, 0.0), 0.0);
    }

    #[test]
    fn a_value_at_rest_has_no_velocity() {
        let sample = Sample::at_rest(100.0);
        assert_eq!(sample.velocity(100.0), 0.0);
    }

    #[test]
    fn the_first_move_measures_from_the_frame_the_gesture_started_on() {
        let mut sample = Sample::at_rest(100.0);
        // 8px in 16ms, measured from where the gesture started rather than from
        // an unset clock.
        sample.update(8.0, 116.0);
        assert_eq!(sample.velocity(116.0), 500.0);
    }

    #[test]
    fn velocity_uses_the_previous_frame_not_the_first_one() {
        let mut sample = Sample::at_rest(0.0);
        sample.update(8.0, 16.0);
        sample.update(20.0, 32.0); // 12px in 16ms, not 20px in 32ms
        assert_eq!(sample.velocity(32.0), 750.0);
    }

    #[test]
    fn everything_that_moves_within_a_frame_counts_as_that_frames_velocity() {
        let mut sample = Sample::at_rest(0.0);
        sample.update(8.0, 16.0);
        // A second update on the same frame is not a new frame: the previous
        // frame's pair stays, so the 16px the value covered during that frame is
        // what the frame's velocity is.
        sample.update(16.0, 16.0);
        assert_eq!(sample.velocity(16.0), 1000.0);
    }

    #[test]
    fn a_stale_value_has_no_velocity() {
        let mut sample = Sample::at_rest(0.0);
        sample.update(8.0, 16.0);
        // The pointer stopped 100ms ago: no velocity to throw with.
        assert_eq!(sample.velocity(116.0), 0.0);
    }

    #[test]
    fn inside_the_range_the_offset_is_the_pointers() {
        let range = DragRange::new(Some(-100.0), Some(100.0));
        let elastic = DragElastic::uniform(0.04);
        assert_eq!(apply_constraints(60.0, range, elastic), 60.0);
        assert_eq!(apply_constraints(-60.0, range, elastic), -60.0);
    }

    #[test]
    fn beyond_a_bound_only_the_elastic_share_of_the_excess_moves() {
        let elastic = DragElastic {
            min: 0.02,
            max: 0.4,
        };
        // The sheet: pushed down (past max 0) it keeps 40% of the travel.
        let range = DragRange::pinned(0.0);
        assert_eq!(apply_constraints(100.0, range, elastic), 40.0);
        // Pushed up, 2%.
        assert_eq!(apply_constraints(-100.0, range, elastic), -2.0);
    }

    #[test]
    fn a_free_range_never_clamps() {
        let elastic = DragElastic::uniform(0.2);
        assert_eq!(apply_constraints(9999.0, DragRange::FREE, elastic), 9999.0);
    }

    #[test]
    fn direction_lock_picks_the_bigger_axis_once_it_clears_the_threshold() {
        assert_eq!(get_current_direction(4.0, 3.0, 10.0), None);
        assert_eq!(get_current_direction(12.0, 3.0, 10.0), Some(Axis::X));
        assert_eq!(get_current_direction(3.0, -12.0, 10.0), Some(Axis::Y));
    }

    #[test]
    fn should_drag_follows_the_axis_and_the_lock() {
        assert!(should_drag(Axis::X, DragAxis::X, None));
        assert!(!should_drag(Axis::Y, DragAxis::X, None));
        assert!(should_drag(Axis::Y, DragAxis::Both, None));
        assert!(should_drag(Axis::X, DragAxis::Both, Some(Axis::X)));
        assert!(!should_drag(Axis::Y, DragAxis::Both, Some(Axis::X)));
    }

    #[test]
    fn the_inertia_target_is_the_ideal_friction_end() {
        assert_eq!(inertia_target(0.0, 520.0, 0.8), 416.0);
    }

    #[test]
    fn the_friction_duration_matches_the_exponential_it_replaces() {
        let amplitude = 416.0;
        let transition = DragTransition::default();
        let duration = inertia_duration(amplitude, transition.time_constant, transition.rest_delta);
        // motion's generator stops when `|amplitude * exp(-t / timeConstant)|`
        // is under `restDelta`. At the duration this computes, it is.
        let remaining = amplitude * (-duration / transition.time_constant).exp();
        assert!(
            (remaining - transition.rest_delta).abs() < 1.0e-9,
            "remaining {remaining}"
        );
    }

    #[test]
    fn the_easing_samples_the_same_curve_as_the_generator() {
        let amplitude = 416.0;
        let time_constant = 750.0;
        let rest_delta = 1.0;
        let duration = inertia_duration(amplitude, time_constant, rest_delta);
        // The generator's value at `t`: `amplitude + (-amplitude * exp(-t / timeConstant))`.
        for step in 0..=10 {
            let t = step as f32 / 10.0;
            let eased = inertia_easing(t, 0.0, amplitude as f32, 1.0);
            let at = f64::from(t) * duration;
            let generator = -amplitude * (-at / time_constant).exp() + amplitude;
            assert!(
                (f64::from(eased) - generator).abs() < 0.5,
                "t={t} eased={eased} generator={generator}"
            );
        }
    }

    #[test]
    fn the_boundary_spring_is_motions_pair() {
        let bouncy = bounce_spring(DragElastic::uniform(0.18));
        assert_eq!((bouncy.stiffness, bouncy.damping), (200.0, 40.0));
        let stiff = bounce_spring(DragElastic::uniform(0.0));
        assert_eq!(
            (stiff.stiffness, stiff.damping),
            (1_000_000.0, 10_000_000.0)
        );
    }

    #[test]
    fn a_throw_outside_the_bounds_springs_and_a_throw_inside_frictions() {
        let range = DragRange::pinned(0.0);
        let elastic = DragElastic::uniform(0.18);
        let transition = DragTransition::default();

        // The toast: released at 90px with 520px/s — 0.8 * 520 puts the ideal
        // end far outside the only legal position, so it springs back.
        let (delta, config) = release_axis(90.0, 520.0, range, elastic, true, transition);
        assert_eq!(delta, -90.0);
        assert!(matches!(
            config.mode,
            dioxus_motion::prelude::AnimationMode::Spring(_)
        ));

        // Released slowly inside a wide range, it frictions toward the ideal
        // end instead.
        let wide = DragRange::new(Some(-1000.0), Some(1000.0));
        let (delta, config) = release_axis(100.0, 200.0, wide, elastic, true, transition);
        assert!((delta - 160.0).abs() < 1.0e-9);
        assert!(matches!(
            config.mode,
            dioxus_motion::prelude::AnimationMode::Tween(_)
        ));

        // Without momentum there is nothing to throw: it settles where it is.
        let (delta, _) = release_axis(100.0, 200.0, wide, elastic, false, transition);
        assert_eq!(delta, 0.0);
    }
}
