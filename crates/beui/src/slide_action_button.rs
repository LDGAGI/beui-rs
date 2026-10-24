//! `motion/slide-action-button.tsx` — drag the thumb to the end to confirm.
//!
//! The drag runs through `beui_core::drag` (pointer capture + velocity
//! sampling), per the port's rule; motion's `dragElastic: 0` and
//! `dragMomentum: false` become `DragElastic::uniform(0.0)` and
//! `without_momentum()`. The travel is measured from the track and thumb boxes,
//! and the fill, label fade and tick path are all derived from the thumb's
//! offset, so there is no second animation to keep in step.

use crate::prelude::*;
use crate::rect::{use_element_rect, ElementRect};
use beui_core::drag::{
    use_drag, Axes, DragAxis, DragConstraints, DragElastic, DragEnd, DragOptions, DragRange,
};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, Time, TimeProvider};

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// The tick path interpolated between motion's three keys.
fn icon_path(progress: f64) -> String {
    const P0: [f64; 6] = [8.0, 5.0, 15.0, 12.0, 8.0, 19.0];
    const P1: [f64; 6] = [7.0, 8.0, 12.0, 14.0, 17.0, 10.0];
    const P2: [f64; 6] = [5.0, 12.0, 10.0, 17.0, 19.0, 7.0];
    let (from, to, t) = if progress <= 0.5 {
        (P0, P1, progress / 0.5)
    } else {
        (P1, P2, (progress - 0.5) / 0.5)
    };
    let value = |index: usize| lerp(from[index], to[index], t);
    format!(
        "M {} {} L {} {} L {} {}",
        value(0),
        value(1),
        value(2),
        value(3),
        value(4),
        value(5)
    )
}

/// Props for [`SlideActionButton`].
#[derive(Props, Clone, PartialEq)]
pub struct SlideActionButtonProps {
    /// The label on the track.
    pub children: Element,
    #[props(default = "Complete".to_string())]
    pub complete_label: String,
    /// Fraction of the travel that completes. Defaults to `0.82`.
    #[props(default = 0.82)]
    pub threshold: f64,
    /// Milliseconds before the control resets. Defaults to `1200`.
    #[props(default = 1200.0)]
    pub reset_delay: f64,
    #[props(default)]
    pub on_complete: Option<EventHandler<()>>,
    #[props(default)]
    pub thumb_class: Option<String>,
    #[props(default)]
    pub fill_class: Option<String>,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub aria_label: Option<String>,
}

/// A drag-to-confirm action.
#[component]
pub fn SlideActionButton(props: SlideActionButtonProps) -> Element {
    let SlideActionButtonProps {
        children,
        complete_label,
        threshold,
        reset_delay,
        on_complete,
        thumb_class,
        fill_class,
        class,
        aria_label,
    } = props;

    let reduce = use_reduced_motion();
    let track_rect: ElementRect = use_element_rect();
    let thumb_rect: ElementRect = use_element_rect();
    use_effect(move || {
        track_rect.refresh();
        thumb_rect.refresh();
    });

    // Travel, derived from the two boxes once measured (reading them here is
    // what re-renders the component when a measurement lands).
    let max = track_rect
        .get()
        .zip(thumb_rect.get())
        .map(|(track, thumb)| (track.width - thumb.width - 8.0).max(1.0))
        .unwrap_or(1.0);

    let mut drag_end = use_signal(|| None::<f64>);
    let options = DragOptions::default()
        .axis(DragAxis::X)
        .constraints(DragConstraints {
            x: DragRange::new(Some(0.0), Some(max)),
            y: DragRange::pinned(0.0),
        })
        .elastic(DragElastic::uniform(0.0))
        .without_momentum();
    let drag = use_drag(
        options,
        Some(EventHandler::new(move |end: DragEnd| {
            drag_end.set(Some(end.offset.x));
        })),
    );

    let mut completed = use_signal(|| false);
    let mut completed_flag = use_signal(|| false);
    let mut generation = use_signal(|| 0u32);
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");
    let reduce_value = reduce();

    let settle_config = move || {
        if reduce_value {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(SPRING_LAYOUT)
        }
    };

    let mut complete = move || {
        if completed_flag() || max <= 1.0 {
            return;
        }
        completed_flag.set(true);
        completed.set(true);
        drag.settle_to(Axes { x: max, y: 0.0 }, settle_config());
        if let Some(handler) = on_complete {
            handler.call(());
        }
        let run = generation() + 1;
        generation.set(run);
        let generation = generation;
        let mut completed_flag = completed_flag;
        let mut completed = completed;
        spawn(async move {
            if Time::delay(dioxus_motion::Duration::from_millis(
                reset_delay.round() as u64
            ))
            .await
            .is_err()
            {
                return;
            }
            if generation() == run {
                completed_flag.set(false);
                completed.set(false);
                drag.settle_to(Axes::ZERO, settle_config());
            }
        });
    };

    use_effect(move || {
        let Some(offset) = drag_end() else {
            return;
        };
        drag_end.set(None);
        if offset >= max * threshold {
            complete();
        } else {
            drag.settle_to(Axes::ZERO, settle_config());
        }
    });

    let offset = drag.offset().x;
    let progress = (offset / max).clamp(0.0, 1.0);
    let label_opacity = if offset <= max * 0.35 {
        1.0 - (offset / (max * 0.35)) * 0.25
    } else if offset <= max * 0.65 {
        0.75 - ((offset - max * 0.35) / (max * 0.3)) * 0.75
    } else {
        0.0
    };
    let is_completed = completed();
    let scale_value = f64::from(*scale.current().read());
    let path = icon_path(progress);

    let track_class = cn!(
        "relative h-16 w-72 overflow-hidden rounded-[22px] bg-muted p-1",
        "ring-1 ring-primary/10",
        TOUCH_GESTURE_CONTENT_CLASS,
        class,
    );
    let fill_class = cn!(
        "absolute inset-0 origin-left bg-primary will-change-transform",
        fill_class,
    );
    let thumb_class = cn!(
        "relative z-10 grid size-14 touch-none cursor-grab place-items-center rounded-[18px] bg-primary text-primary-foreground shadow-sm",
        TOUCH_GESTURE_CLASS,
        "outline-none active:cursor-grabbing focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        if is_completed {
            "cursor-default bg-background text-foreground"
        } else {
            ""
        },
        thumb_class,
    );
    let label = aria_label.unwrap_or_else(|| "Slide action".to_string());
    let completed_opacity = if is_completed { 1.0 } else { 0.0 };
    let completed_label = if is_completed {
        Some(complete_label.clone())
    } else {
        None
    };

    rsx! {
        div {
            class: track_class,
            "data-beui-rect": track_rect.attr(),
            onmounted: move |data| track_rect.measure(data.data()),
            span { aria_hidden: "true", class: fill_class, style: "transform:scaleX({progress})" }
            span {
                aria_hidden: "true",
                class: "pointer-events-none absolute inset-0 grid place-items-center pl-10 text-sm font-medium text-foreground",
                style: "opacity:{label_opacity}",
                {children}
            }
            span {
                aria_live: "polite",
                class: "pointer-events-none absolute inset-0 grid place-items-center text-sm font-medium text-primary-foreground",
                style: "opacity:{completed_opacity}",
                if let Some(completed_label) = completed_label {
                    "{completed_label}"
                }
            }
            button {
                r#type: "button",
                aria_label: label,
                "data-beui-rect": thumb_rect.attr(),
                onmounted: move |data| thumb_rect.measure(data.data()),
                class: thumb_class,
                style: "transform:translateX({offset}px) scale({scale_value})",
                onpointerdown: move |event: Event<PointerData>| {
                    if is_completed {
                        return;
                    }
                    if !reduce_value {
                        let _ = scale.animate_to(0.94, AnimationConfig::spring(SPRING_PRESS));
                    }
                    drag.on_pointer_down(&event);
                },
                onpointermove: move |event: Event<PointerData>| drag.on_pointer_move(&event),
                onpointerup: move |event: Event<PointerData>| {
                    let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
                    drag.on_pointer_up(&event);
                },
                onpointercancel: move |event: Event<PointerData>| {
                    let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
                    drag.on_pointer_cancel(&event);
                },
                onpointerleave: move |_| {
                    let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
                },
                onkeydown: move |event: Event<KeyboardData>| {
                    let activate = event.key() == Key::Enter
                        || event.key() == Key::Character(" ".to_string());
                    if activate {
                        event.prevent_default();
                        complete();
                    }
                },
                svg {
                    view_box: "0 0 24 24",
                    "aria-hidden": "true",
                    class: "size-5",
                    path {
                        d: "{path}",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        stroke_width: "2",
                    }
                }
            }
        }
    }
}
