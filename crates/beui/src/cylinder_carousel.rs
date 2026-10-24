//! `motion/cylinder-carousel.tsx` — a row of slides projected onto the inside
//! (or outside) of a cylinder, rolled by drag, wheel, arrow keys or on its own.
//!
//! The geometry is the source's, number for number: `THETA_EDGE = 72°` and
//! `THETA_CLAMP = 95°`; `alpha = THETA_EDGE / edgeOffset` with
//! `edgeOffset = (visibleItems + 1) / 2`; `k = max(0.2, (minScale - cos 72°) /
//! (1 - minScale))`; `projection = halfWidth * (cos 72° + k) / sin 72°`; the
//! slide size `min(itemSize, stageWidth * 0.65 / scaleSum)`; `arc` defaulting to
//! `size * 0.35`. Every ball shares the one `1 / (cosθ + k)` depth term, so
//! horizontal position, size and lift stay consistent, and the wrapped offset
//! `o - round(o / count) * count` makes the row loop.
//!
//! The roll lives in a plain signal (drags, the wheel and the auto-rotation
//! write it 1:1); releases hand the pointer velocity to the glide spring
//! `{40, 20, 3}` through [`AnimationManager::set_velocity`], with the
//! flick projection `velocity * 0.45` clamped to six items before the snap.

use crate::prelude::*;
use crate::rect::use_element_width;
use beui_core::platform::{capture_pointer, release_pointer};
use dioxus::html::geometry::WheelDelta;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{
    use_motion, AnimationConfig, AnimationManager, Duration, MotionHandle, Spring, Time,
    TimeProvider,
};

const THETA_EDGE_DEG: f64 = 72.0;
const THETA_CLAMP_DEG: f64 = 95.0;
/// How far a flick keeps rolling: projected items = release velocity * momentum.
const FLICK_MOMENTUM: f64 = 0.45;
const MAX_FLICK_ITEMS: f64 = 6.0;

fn theta_edge() -> f64 {
    THETA_EDGE_DEG.to_radians()
}

fn theta_clamp() -> f64 {
    THETA_CLAMP_DEG.to_radians()
}

/// The soft spring the release glides on.
fn glide_spring() -> Spring {
    Spring {
        stiffness: 40.0,
        damping: 20.0,
        mass: 3.0,
    }
}

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// The pointer samples a release turns into a velocity, in px and ms.
#[derive(Clone, Copy, PartialEq)]
struct DragTrack {
    start_x: f64,
    start_scroll: f64,
    last_x: f64,
    last_t: f64,
    prev_x: f64,
    prev_t: f64,
}

/// Which way the wall curves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CylinderVariant {
    /// Inside of the cylinder — the centre slide is smallest and dipped.
    #[default]
    Concave,
    /// Outside — the centre slide is biggest and raised.
    Convex,
}

/// Props for [`CylinderCarousel`].
#[derive(Props, Clone, PartialEq)]
pub struct CylinderCarouselProps {
    pub items: Vec<Element>,
    /// Max slide size in px (square). Defaults to `200`.
    #[props(default = 200.0)]
    pub item_size: f64,
    /// How many slots span the container width. Defaults to `5`.
    #[props(default = 5)]
    pub visible_items: usize,
    #[props(default)]
    pub variant: CylinderVariant,
    /// Scale of the smallest slide. Defaults to `0.55`.
    #[props(default = 0.55)]
    pub min_scale: f64,
    /// Items rolled per item-width dragged. Defaults to `1.5`.
    #[props(default = 1.5)]
    pub drag_speed: f64,
    /// Curve depth in px. Defaults to 35% of the slide size.
    #[props(default)]
    pub arc: Option<f64>,
    /// Snap to the nearest slide when the roll settles. Defaults to `true`.
    #[props(default = true)]
    pub snap: bool,
    #[props(default)]
    pub auto_rotate: bool,
    /// Auto-roll speed in items per second. Defaults to `0.4`.
    #[props(default = 0.4)]
    pub auto_rotate_speed: f64,
    #[props(default)]
    pub default_index: usize,
    #[props(default)]
    pub on_index_change: Option<EventHandler<usize>>,
    /// Stage height in px. Defaults to the slide size.
    #[props(default)]
    pub height: Option<f64>,
    #[props(default)]
    pub class: Option<String>,
}

/// A cylinder carousel.
#[component]
pub fn CylinderCarousel(props: CylinderCarouselProps) -> Element {
    let CylinderCarouselProps {
        items,
        item_size,
        visible_items,
        variant,
        min_scale,
        drag_speed,
        arc,
        snap,
        auto_rotate,
        auto_rotate_speed,
        default_index,
        on_index_change,
        height,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let count = items.len();
    let width_stream = use_element_width();
    let measured = width_stream.width();
    let stage_width = if measured > 0.0 { measured } else { 800.0 };
    let half_width = stage_width / 2.0;
    let convex = variant == CylinderVariant::Convex;
    let visible_items = visible_items.max(1);
    let edge_offset = (visible_items as f64 + 1.0) / 2.0;

    // Fit: the resting row's diameters may take at most ~66% of the stage.
    let mut scale_sum = 0.0;
    for index in 0..visible_items {
        let t = (index as f64 - (visible_items - 1) as f64 / 2.0).abs() / edge_offset;
        scale_sum += if convex {
            1.0 - (1.0 - min_scale) * t
        } else {
            min_scale + (1.0 - min_scale) * t
        };
    }
    let size = item_size.min(stage_width * 0.65 / scale_sum.max(f64::EPSILON));
    let gap = stage_width / (visible_items as f64 + 1.0);
    let arc = arc.unwrap_or(size * 0.35);
    let alpha = theta_edge() / edge_offset;
    let k = ((min_scale - theta_edge().cos()) / (1.0 - min_scale)).max(0.2);
    let projection = half_width * (theta_edge().cos() + k) / theta_edge().sin();

    // The roll, in item units.
    let mut scroll = use_signal(|| default_index as f64);
    // The glide runs on a delta motion so the release can carry its velocity:
    // `rendered = base + glide`, reset to zero at each new glide.
    let glide_base = use_signal(|| default_index as f64);
    let mut glide = use_motion(0.0f32).expect("0.0 is finite");
    let gliding = use_signal(|| false);
    let dragging = use_signal(|| false);
    let mut hovering = use_signal(|| false);
    let drag = use_signal(|| DragTrack {
        start_x: 0.0,
        start_scroll: 0.0,
        last_x: 0.0,
        last_t: 0.0,
        prev_x: 0.0,
        prev_t: 0.0,
    });
    let mut reported = use_signal(|| default_index);

    let position = if gliding() {
        glide_base() + f64::from(current(glide))
    } else {
        scroll()
    };

    let stop_glide = {
        let mut gliding = gliding;
        move || {
            if gliding() {
                glide.stop();
                scroll.set(glide_base() + f64::from(current(glide)));
                gliding.set(false);
            }
        }
    };

    let glide_to = {
        let mut glide = glide;
        let mut glide_base = glide_base;
        let mut gliding = gliding;
        move |to: f64, velocity: f64| {
            if reduce_value {
                scroll.set(to);
                return;
            }
            glide_base.set(scroll());
            glide.reset();
            gliding.set(true);
            let _ = glide.animate_to(
                (to - scroll()) as f32,
                AnimationConfig::spring(glide_spring()),
            );
            let _ = glide.set_velocity(velocity as f32);
        }
    };

    let settle = {
        let mut glide_to = glide_to;
        move |velocity: f64| {
            let projected =
                scroll() + (velocity * FLICK_MOMENTUM).clamp(-MAX_FLICK_ITEMS, MAX_FLICK_ITEMS);
            glide_to(if snap { projected.round() } else { projected }, velocity);
        }
    };

    // Report the resting index.
    use_effect(move || {
        if count == 0 {
            return;
        }
        let rounded = position.round();
        let index = ((rounded as i64 % count as i64 + count as i64) % count as i64) as usize;
        if index != reported() {
            reported.set(index);
            if let Some(handler) = on_index_change {
                handler.call(index);
            }
        }
    });

    // Auto-rotation, paused while dragging, hovering or gliding.
    use_future(move || async move {
        if !auto_rotate || reduce_value || count == 0 {
            return;
        }
        loop {
            if Time::delay(Duration::from_millis(16)).await.is_err() {
                return;
            }
            if !dragging() && !hovering() && !gliding() {
                scroll.set(scroll() + auto_rotate_speed * 0.016);
            }
        }
    });

    let stage_height = height.unwrap_or(size);
    let stage_class = cn!(
        "relative w-full touch-none outline-none [clip-path:inset(0)]",
        TOUCH_GESTURE_CLASS,
        "cursor-grab active:cursor-grabbing",
        "focus-visible:ring-2 focus-visible:ring-foreground/20",
        class,
    );

    rsx! {
        div {
            "data-beui-width": width_stream.attr(),
            role: "application",
            aria_roledescription: "carousel",
            tabindex: "0",
            class: stage_class,
            style: "height:{stage_height}px",
            onpointerenter: move |_| hovering.set(true),
            onpointerleave: move |_| hovering.set(false),
            onpointerdown: {
                let mut drag = drag;
                let mut dragging = dragging;
                let mut stop_glide = stop_glide;
                move |event: Event<PointerData>| {
                    event.prevent_default();
                    stop_glide();
                    dragging.set(true);
                    let _ = capture_pointer(&event);
                    let x = event.client_coordinates().x;
                    let now = now_ms();
                    drag.set(DragTrack {
                        start_x: x,
                        start_scroll: scroll(),
                        last_x: x,
                        last_t: now,
                        prev_x: x,
                        prev_t: now,
                    });
                }
            },
            onpointermove: {
                let mut drag = drag;
                move |event: Event<PointerData>| {
                    if !dragging() {
                        return;
                    }
                    let track = drag();
                    let x = event.client_coordinates().x;
                    scroll.set(track.start_scroll - ((x - track.start_x) * drag_speed) / gap);
                    drag.set(DragTrack {
                        prev_x: track.last_x,
                        prev_t: track.last_t,
                        last_x: x,
                        last_t: now_ms(),
                        ..track
                    });
                }
            },
            onpointerup: {
                let mut dragging = dragging;
                let mut settle = settle;
                move |event: Event<PointerData>| {
                    if !dragging() {
                        return;
                    }
                    dragging.set(false);
                    let _ = release_pointer(&event);
                    let track = drag();
                    let dt = track.last_t - track.prev_t;
                    let vpx = if dt > 0.0 {
                        (track.last_x - track.prev_x) / dt
                    } else {
                        0.0
                    };
                    settle(-vpx * drag_speed * 1000.0 / gap);
                }
            },
            onpointercancel: {
                let mut dragging = dragging;
                move |event: Event<PointerData>| {
                    if dragging() {
                        dragging.set(false);
                        let _ = release_pointer(&event);
                    }
                }
            },
            onwheel: {
                let mut stop_glide = stop_glide;
                move |event: Event<WheelData>| {
                    stop_glide();
                    let delta = match event.delta() {
                        WheelDelta::Pixels(vector) => {
                            if vector.x.abs() > vector.y.abs() { vector.x } else { vector.y }
                        }
                        WheelDelta::Lines(vector) => {
                            let (x, y) = (vector.x * 16.0, vector.y * 16.0);
                            if x.abs() > y.abs() { x } else { y }
                        }
                        WheelDelta::Pages(vector) => {
                            let (x, y) = (vector.x * 400.0, vector.y * 400.0);
                            if x.abs() > y.abs() { x } else { y }
                        }
                    };
                    scroll.set(scroll() + delta / gap);
                    // A short debounce, then the same settle a release takes.
                    let mut settle = settle;
                    spawn(async move {
                        if Time::delay(Duration::from_millis(140)).await.is_err() {
                            return;
                        }
                        settle(0.0);
                    });
                }
            },
            onkeydown: {
                let mut glide_to = glide_to;
                move |event: Event<KeyboardData>| {
                    let direction = match event.key() {
                        Key::ArrowRight => 1.0,
                        Key::ArrowLeft => -1.0,
                        _ => return,
                    };
                    event.prevent_default();
                    glide_to(scroll().round() + direction, 0.0);
                }
            },
            for (index, item) in items.iter().enumerate() {
                CylinderBall {
                    key: "{index}",
                    index,
                    count,
                    scroll: position,
                    alpha,
                    k,
                    projection,
                    gap,
                    edge_offset,
                    min_scale,
                    convex,
                    arc,
                    half_width,
                    size,
                    {item.clone()}
                }
            }
        }
    }
}

/// One slide on the wall.
#[component]
fn CylinderBall(
    index: usize,
    count: usize,
    scroll: f64,
    alpha: f64,
    k: f64,
    projection: f64,
    gap: f64,
    edge_offset: f64,
    min_scale: f64,
    convex: bool,
    arc: f64,
    half_width: f64,
    size: f64,
    children: Element,
) -> Element {
    // Nearest wrapped offset, so the row loops continuously.
    let mut offset = index as f64 - scroll;
    if count > 0 {
        offset -= (offset / count as f64).round() * count as f64;
    }
    let x = if convex {
        offset * gap
    } else {
        let theta = (offset * alpha).clamp(-theta_clamp(), theta_clamp());
        projection * theta.sin() / (theta.cos() + k)
    };
    let t = (offset.abs() / edge_offset).min(theta_clamp() / theta_edge());
    let scale = if convex {
        1.0 - (1.0 - min_scale) * t
    } else {
        min_scale + (1.0 - min_scale) * t
    };
    let ratio = x / half_width;
    let valley = arc * (0.5 - ratio * ratio);
    let y = if convex { -valley } else { valley };
    let visibility = if x.abs() > half_width + size {
        "hidden"
    } else {
        "visible"
    };
    let style = format!(
        "position:absolute;top:50%;left:50%;width:{size}px;height:{size}px;\
         margin-left:{}px;margin-top:{}px;visibility:{visibility};\
         transform:translate({x}px, {y}px) scale({scale})",
        -size / 2.0,
        -size / 2.0,
    );

    rsx! {
        div { "data-carousel-ball": "{index}", class: "absolute top-1/2 left-1/2", style, {children} }
    }
}

fn now_ms() -> f64 {
    beui_core::platform::now_ms()
}
