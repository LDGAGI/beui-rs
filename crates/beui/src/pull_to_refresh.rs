//! `motion/pull-to-refresh.tsx` — pull the top of a scroller down to refresh.
//!
//! The pull uses the source's resisted distance
//! (`maxPull · (1 - e^(-d / maxPull))`), threshold and status machine
//! (`idle → pulling → ready → refreshing`), and holds the content at
//! `holdDistance` while refreshing. The indicator is a small character that
//! lifts, tilts and stretches with the pull, spins and blinks while refreshing,
//! and swaps its label between "pull", "release" and "refreshing".
//!
//! The scroller's own offset comes from [`crate::use_scroll_state`], so the
//! gesture handlers can check `scrollTop > 0` without a DOM read. Pointer input
//! (mouse, pen) runs through pointer capture; touch runs through the native
//! touch handlers, which can `preventDefault` the page scroll.

use crate::prelude::*;
use crate::smooth_scroll::use_scroll_state;
use beui_core::platform::{capture_pointer, next_scope_id, release_pointer};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

/// Indicator animations that have no spring equivalent.
const PULL_KEYFRAMES: &str = "\
@keyframes beui-pull-spin{to{transform:rotate(360deg)}}\
@keyframes beui-pull-blink{0%,100%{transform:scaleY(1)}45%{transform:scaleY(1)}55%{transform:scaleY(0.15)}65%{transform:scaleY(1)}}\
@keyframes beui-pull-bob{0%,100%{transform:translateY(0) rotate(0deg)}50%{transform:translateY(-2px) rotate(3deg)}}";

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// The pull's state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PullToRefreshStatus {
    #[default]
    Idle,
    Pulling,
    Ready,
    Refreshing,
}

/// `resistedDistance`.
fn resisted_distance(distance: f64, max_pull: f64) -> f64 {
    max_pull * (1.0 - (-distance.max(0.0) / max_pull).exp())
}

/// Props for [`PullToRefresh`].
#[derive(Props, Clone, PartialEq)]
pub struct PullToRefreshProps {
    /// Runs after the user pulls past the threshold and releases.
    pub on_refresh: EventHandler<()>,
    pub children: Element,
    /// Keeps the indicator active while an externally managed refresh runs.
    #[props(default)]
    pub refreshing: bool,
    #[props(default)]
    pub disabled: bool,
    /// Resisted pull distance in px required to refresh. Defaults to `76`.
    #[props(default = 76.0)]
    pub threshold: f64,
    /// Maximum resisted pull distance in px. Defaults to `132`.
    #[props(default = 132.0)]
    pub max_pull: f64,
    /// Content offset in px while refreshing. Defaults to `68`.
    #[props(default = 68.0)]
    pub hold_distance: f64,
    #[props(default = "Pull to refresh".to_string())]
    pub pulling_label: String,
    #[props(default = "Release to refresh".to_string())]
    pub release_label: String,
    #[props(default = "Refreshing".to_string())]
    pub refreshing_label: String,
    #[props(default = "Refreshable content".to_string())]
    pub aria_label: String,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub content_class: Option<String>,
    #[props(default)]
    pub indicator_class: Option<String>,
}

/// A pull-to-refresh scroll container.
#[component]
pub fn PullToRefresh(props: PullToRefreshProps) -> Element {
    let PullToRefreshProps {
        on_refresh,
        children,
        refreshing,
        disabled,
        threshold,
        max_pull,
        hold_distance,
        pulling_label,
        release_label,
        refreshing_label,
        aria_label,
        class,
        content_class,
        indicator_class,
    } = props;

    let reduce = use_reduced_motion();
    let scope = use_hook(next_scope_id);
    let selector = format!("[data-beui-pull-scroll=\"{scope}\"]");
    let scroll = use_scroll_state(Some(selector));
    let mut status = use_signal(PullToRefreshStatus::default);
    let mut internal_refreshing = use_signal(|| false);
    let mut gesture = use_signal(|| None::<(f64, f64, Option<i32>)>);
    let mut y = use_motion(0.0f32).expect("0.0 is finite");
    let reduce_value = reduce();

    let pull_threshold = threshold.max(24.0);
    let pull_limit = max_pull.max(pull_threshold + 24.0);
    let resting_distance = hold_distance.max(0.0).min(pull_threshold);
    let is_refreshing = refreshing || internal_refreshing();

    let mut settle = move |target: f64| {
        let config = if reduce_value {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(SPRING_PANEL)
        };
        let _ = y.animate_to(target as f32, config);
    };
    let mut update_pull = move |distance: f64| {
        if disabled || is_refreshing {
            return;
        }
        let next = resisted_distance(distance, pull_limit);
        let _ = y.animate_to(next as f32, beui_core::ease::tween_ease_out(0));
        status.set(if next >= pull_threshold {
            PullToRefreshStatus::Ready
        } else {
            PullToRefreshStatus::Pulling
        });
    };
    let mut finish_pull = move || {
        let should_refresh =
            f64::from(*y.current().peek()) >= pull_threshold && !disabled && !is_refreshing;
        gesture.set(None);
        if should_refresh {
            internal_refreshing.set(true);
            status.set(PullToRefreshStatus::Refreshing);
            settle(resting_distance);
            on_refresh.call(());
            // A synchronous refresh resolves at once; if the consumer did not
            // take over with `refreshing`, release here.
            if !refreshing {
                internal_refreshing.set(false);
                status.set(PullToRefreshStatus::Idle);
                settle(0.0);
            }
        } else {
            status.set(PullToRefreshStatus::Idle);
            settle(0.0);
        }
    };

    // The external `refreshing` prop drives the hold/release.
    let mut was_external = use_signal(|| false);
    use_effect(use_reactive((&refreshing,), move |(refreshing,)| {
        if refreshing && !*was_external.peek() {
            was_external.set(true);
            status.set(PullToRefreshStatus::Refreshing);
            settle(resting_distance);
        } else if !refreshing && *was_external.peek() {
            was_external.set(false);
            internal_refreshing.set(false);
            status.set(PullToRefreshStatus::Idle);
            settle(0.0);
        }
    }));

    let y_value = f64::from(current(y));
    let progress = (y_value / pull_threshold).clamp(0.0, 1.0);
    let indicator_opacity =
        (y_value / 10.0).clamp(0.0, 1.0) * 0.55 + (y_value / pull_threshold).clamp(0.0, 1.0) * 0.45;
    let indicator_scale = 0.86 + 0.14 * progress;
    let status_now = status();
    let label = match status_now {
        PullToRefreshStatus::Refreshing => refreshing_label.clone(),
        PullToRefreshStatus::Ready => release_label.clone(),
        _ => pulling_label.clone(),
    };
    let label_opacity = |candidate: PullToRefreshStatus| {
        if status_now == candidate {
            1.0
        } else {
            0.0
        }
    };
    let label_a_style = format!(
        "opacity:{};transform:translateY({}px);transition:opacity 160ms,transform 160ms",
        label_opacity(PullToRefreshStatus::Pulling),
        0.0
    );
    let label_b_style = format!(
        "opacity:{};transform:translateY({}px);transition:opacity 160ms,transform 160ms",
        label_opacity(PullToRefreshStatus::Ready),
        0.0
    );
    let label_c_style = format!(
        "opacity:{};transform:translateY({}px);transition:opacity 160ms,transform 160ms",
        label_opacity(PullToRefreshStatus::Refreshing),
        0.0
    );

    let dragging = matches!(
        status_now,
        PullToRefreshStatus::Pulling | PullToRefreshStatus::Ready
    );
    let root_class = cn!(
        "relative w-full overflow-y-auto overscroll-contain bg-background",
        TOUCH_GESTURE_CONTENT_CLASS,
        if dragging {
            "cursor-grabbing select-none"
        } else {
            "cursor-grab"
        },
        if disabled || is_refreshing {
            "cursor-default"
        } else {
            ""
        },
        class,
    );
    let indicator_class = cn!(
        "pointer-events-none absolute inset-x-0 top-0 z-20 flex h-[4.25rem] flex-col items-center justify-center gap-0.5 bg-gradient-to-b from-background via-background/95 to-transparent text-[11px] font-medium text-muted-foreground",
        indicator_class,
    );
    let content_class = cn!(
        "relative z-10 min-h-full bg-inherit will-change-transform",
        content_class,
    );
    let indicator_style = if reduce_value {
        format!("opacity:{indicator_opacity}")
    } else {
        format!("opacity:{indicator_opacity};transform:scale({indicator_scale})")
    };

    rsx! {
        style { dangerous_inner_html: PULL_KEYFRAMES }
        section {
            "data-beui-pull-scroll": "{scope}",
            aria_label,
            aria_busy: if is_refreshing { "true" } else { "false" },
            "data-state": match status_now {
                PullToRefreshStatus::Idle => "idle",
                PullToRefreshStatus::Pulling => "pulling",
                PullToRefreshStatus::Ready => "ready",
                PullToRefreshStatus::Refreshing => "refreshing",
            },
            class: root_class,
            onpointerdown: move |event: Event<PointerData>| {
                if event.pointer_type() == "touch"
                    || disabled
                    || is_refreshing
                    || (*scroll.scroll_y.peek()) > 0.0
                {
                    return;
                }
                let _ = capture_pointer(&event);
                gesture.set(Some((
                    event.client_coordinates().x,
                    event.client_coordinates().y,
                    Some(event.pointer_id()),
                )));
            },
            onpointermove: move |event: Event<PointerData>| {
                let Some((start_x, start_y, pointer_id)) = gesture() else {
                    return;
                };
                if pointer_id != Some(event.pointer_id()) {
                    return;
                }
                let point = event.client_coordinates();
                let delta_x = point.x - start_x;
                let delta_y = point.y - start_y;
                if delta_y < 0.0 || delta_x.abs() > delta_y {
                    return;
                }
                event.prevent_default();
                update_pull(delta_y);
            },
            onpointerup: move |event: Event<PointerData>| {
                let _ = release_pointer(&event);
                if let Some((_, _, Some(pointer_id))) = gesture() {
                    if pointer_id == event.pointer_id() {
                        finish_pull();
                    }
                }
            },
            onpointercancel: move |event: Event<PointerData>| {
                if let Some((_, _, Some(pointer_id))) = gesture() {
                    if pointer_id == event.pointer_id() {
                        finish_pull();
                    }
                }
            },
            ontouchstart: move |event: Event<TouchData>| {
                let touches = event.touches();
                if touches.len() != 1 || (*scroll.scroll_y.peek()) > 0.0 || disabled || is_refreshing {
                    return;
                }
                let point = touches[0].client_coordinates();
                gesture.set(Some((point.x, point.y, None)));
            },
            ontouchmove: move |event: Event<TouchData>| {
                let Some((start_x, start_y, _)) = gesture() else {
                    return;
                };
                let touches = event.touches();
                let Some(touch) = touches.first() else {
                    return;
                };
                let point = touch.client_coordinates();
                let delta_x = point.x - start_x;
                let delta_y = point.y - start_y;
                if (*scroll.scroll_y.peek()) > 0.0 || delta_y < 0.0 {
                    gesture.set(None);
                    return;
                }
                if delta_x.abs() > delta_y {
                    return;
                }
                event.prevent_default();
                update_pull(delta_y);
            },
            ontouchend: move |_| {
                if gesture().is_some() {
                    finish_pull();
                }
            },
            div {
                aria_live: "polite",
                aria_atomic: "true",
                class: indicator_class,
                style: indicator_style,
                RefreshBuddy { progress, status: status_now, reduce: reduce_value }
                span { class: "relative h-4 min-w-24 text-center",
                    span { class: "absolute inset-x-0 whitespace-nowrap", style: label_a_style, {pulling_label.clone()} }
                    span { class: "absolute inset-x-0 whitespace-nowrap", style: label_b_style, {release_label.clone()} }
                    span { class: "absolute inset-x-0 whitespace-nowrap", style: label_c_style, "{label}" }
                }
            }
            div {
                class: content_class,
                style: if reduce_value { String::new() } else { format!("transform:translateY({y_value}px)") },
                {children}
            }
        }
    }
}

/// The pull indicator character.
#[component]
fn RefreshBuddy(progress: f64, status: PullToRefreshStatus, reduce: bool) -> Element {
    let ready = status == PullToRefreshStatus::Ready;
    let refreshing = status == PullToRefreshStatus::Refreshing;
    let lift = -7.0 * (1.0 - progress);
    let tilt = -10.0 * (1.0 - progress);
    // `[0, 0.55, 1] → [0.68, 1.1, 0.92]`
    let stretch = if progress <= 0.55 {
        0.68 + (progress / 0.55) * (1.1 - 0.68)
    } else {
        1.1 + ((progress - 0.55) / 0.45) * (0.92 - 1.1)
    };
    let buddy_style = if reduce {
        String::new()
    } else {
        format!("transform:translateY({lift}px) rotate({tilt}deg) scaleY({stretch})")
    };
    let svg_style = if refreshing {
        if reduce {
            "animation:beui-pull-bob 1.2s ease-in-out infinite".to_string()
        } else {
            "animation:beui-pull-bob 0.9s ease-in-out infinite".to_string()
        }
    } else if ready && !reduce {
        "transform:scale(1.08)".to_string()
    } else {
        String::new()
    };
    let arc_style = if refreshing && !reduce {
        "transform-origin:18px 18px;animation:beui-pull-spin 0.9s linear infinite".to_string()
    } else if reduce {
        "transform-origin:18px 18px".to_string()
    } else {
        format!(
            "transform-origin:18px 18px;transform:rotate({}deg)",
            if ready { 0.0 } else { -35.0 }
        )
    };
    let eyes_style = if refreshing && !reduce {
        "transform-origin:18px 16px;animation:beui-pull-blink 0.9s ease-in-out infinite".to_string()
    } else if reduce {
        "transform-origin:18px 16px".to_string()
    } else {
        format!(
            "transform-origin:18px 16px;transform:scaleY({})",
            if ready { 1.18 } else { 1.0 }
        )
    };

    rsx! {
        span { aria_hidden: "true", class: "block h-9 w-9 origin-bottom", style: buddy_style,
            svg { "aria-hidden": "true", view_box: "0 0 36 36", class: "h-full w-full overflow-visible", style: svg_style,
                g {
                    style: arc_style,
                    class: cn!(
                        "transition-opacity duration-150",
                        if ready || refreshing { "opacity-100" } else { "opacity-0" },
                    ),
                    path { d: "M18 2.5a15.5 15.5 0 0 1 12.7 6.6", fill: "none", stroke: "currentColor", stroke_width: "1.5", stroke_linecap: "round", class: "text-muted-foreground" }
                    circle { cx: "31.3", cy: "10.2", r: "2.2", class: "fill-foreground" }
                }
                rect { x: "7", y: "7", width: "22", height: "22", rx: "9", class: "fill-foreground" }
                g { style: eyes_style,
                    circle { cx: "14.2", cy: "16", r: "1.45", class: "fill-background" }
                    circle { cx: "21.8", cy: "16", r: "1.45", class: "fill-background" }
                }
                path {
                    d: "M14.5 21h7",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                    class: cn!(
                        "text-background transition-opacity duration-150",
                        if ready || refreshing { "opacity-0" } else { "opacity-100" },
                    ),
                }
                path {
                    d: "M14 20.5c1 2.4 7 2.4 8 0",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                    class: cn!(
                        "text-background transition-opacity duration-150",
                        if ready { "opacity-100" } else { "opacity-0" },
                    ),
                }
                circle {
                    cx: "18",
                    cy: "21",
                    r: "1.6",
                    class: cn!(
                        "fill-background transition-opacity duration-150",
                        if refreshing { "opacity-100" } else { "opacity-0" },
                    ),
                }
            }
        }
    }
}
