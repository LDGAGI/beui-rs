//! `motion/button/base.tsx` — the button and its link twin.
//!
//! Press and hover feedback are the same spring: `whileTap` scales to
//! `pressScale` (0.93 by default) and `whileHover` to `1.02`, both on
//! `SPRING_PRESS`. The source lets motion arbitrate between the two gestures;
//! here the gesture state is explicit — pressed beats hovered, as motion's
//! gesture priority does — and the spring is re-targeted on every transition.
//!
//! `ripple` is the optional Material-style press: a circle spawns at the press
//! point and scales to twice the button's longest side while fading out over
//! `1.6s` on `EASE_OUT`.

use dioxus::prelude::*;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

use beui_core::cn;
use beui_core::ease::SPRING_PRESS;
use beui_core::hooks::{use_hover_capable, use_reduced_motion};
use beui_core::platform::{self, Rect};

use crate::motion::{
    animate_number, animate_style, css, motion_style, number, scale_style, spring, tween,
    use_number_motion, use_style_motion, Ease,
};

/// `ButtonVariant`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Outline,
}

/// `ButtonSize`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
    Icon,
}

impl ButtonVariant {
    const fn class(self) -> &'static str {
        match self {
            Self::Primary => "bg-primary text-primary-foreground hover:bg-primary/90",
            Self::Secondary => "border border-border bg-card text-foreground hover:border-border",
            Self::Ghost => "text-muted-foreground hover:text-foreground hover:bg-muted/60",
            Self::Outline => {
                "border border-border bg-transparent text-foreground hover:bg-muted/60"
            }
        }
    }
}

impl ButtonSize {
    const fn class(self) -> &'static str {
        match self {
            Self::Sm => "h-8 px-3 text-xs gap-1.5 rounded-full",
            Self::Md => "h-10 px-5 text-sm gap-2 rounded-full",
            Self::Lg => "h-12 px-6 text-base gap-2 rounded-full",
            Self::Icon => "h-8 w-8 rounded-lg",
        }
    }
}

/// `whileTap` beats `whileHover`, as motion's gesture priority does.
fn gesture_target(pressed: bool, hovered: bool, press_scale: f32, can_hover: bool) -> f32 {
    if pressed {
        press_scale
    } else if can_hover && hovered {
        1.02
    } else {
        1.0
    }
}

/// One entry in the ripple list.
#[derive(Clone, PartialEq)]
struct Ripple {
    id: u32,
    x: f64,
    y: f64,
    size: f64,
}

#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default = 0.93)] press_scale: f32,
    #[props(default)] ripple: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] on_hover_start: Option<EventHandler<()>>,
    #[props(default)] on_hover_end: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion();
    let can_hover = use_hover_capable();
    let mut scale = use_number_motion(1.0);
    let mut hovered = use_signal(|| false);
    let mut pressed = use_signal(|| false);

    let mut ripples = use_signal(Vec::<Ripple>::new);
    let mut next_id = use_signal(|| 0u32);
    let mut rect = use_signal(|| None::<Rect>);

    let can_hover_now = can_hover() && !reduce();

    rsx! {
        button {
            r#type: "button",
            disabled,
            "aria-label": aria_label,
            class: cn!(
                "inline-flex items-center justify-center font-medium select-none",
                "transition-colors",
                "disabled:pointer-events-none disabled:opacity-50",
                if ripple { "relative overflow-hidden" } else { "" },
                variant.class(),
                size.class(),
                class,
            ),
            style: scale_style(number(&scale)),
            onmounted: move |data| {
                spawn(async move {
                    if let Some(measured) = platform::client_rect(&data).await {
                        rect.set(Some(measured));
                    }
                });
            },
            onpointerdown: move |event| {
                if !reduce() {
                    pressed.set(true);
                    animate_number(
                        &mut scale,
                        gesture_target(true, hovered(), press_scale, can_hover_now),
                        spring(SPRING_PRESS),
                    );
                }
                if !ripple || reduce() {
                    return;
                }
                let Some(measured) = *rect.peek() else {
                    return;
                };
                let point = event.client_coordinates();
                let size = measured.width.max(measured.height) * 2.0;
                let id = next_id();
                next_id += 1;
                ripples.write().push(Ripple {
                    id,
                    x: point.x - measured.left,
                    y: point.y - measured.top,
                    size,
                });
            },
            onpointerenter: move |_| {
                if let Some(on_hover_start) = on_hover_start {
                    on_hover_start.call(());
                }
                if !can_hover_now {
                    return;
                }
                hovered.set(true);
                if !pressed() {
                    animate_number(&mut scale, 1.02, spring(SPRING_PRESS));
                }
            },
            onpointerleave: move |_| {
                if let Some(on_hover_end) = on_hover_end {
                    on_hover_end.call(());
                }
                hovered.set(false);
                if !pressed() {
                    animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
                }
            },
            onpointerup: move |_| {
                if !pressed() {
                    return;
                }
                pressed.set(false);
                animate_number(
                    &mut scale,
                    gesture_target(false, hovered(), press_scale, can_hover_now),
                    spring(SPRING_PRESS),
                );
            },
            onpointercancel: move |_| {
                if !pressed() {
                    return;
                }
                pressed.set(false);
                animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
            },
            onclick: move |event| {
                if let Some(onclick) = onclick {
                    onclick.call(event);
                }
            },
            if ripple && !reduce() {
                span {
                    class: "pointer-events-none absolute inset-0 overflow-hidden rounded-[inherit]",
                    for item in ripples() {
                        ButtonRipple {
                            key: "{item.id}",
                            item: item.clone(),
                            on_done: move |id| {
                                ripples.write().retain(|r| r.id != id);
                            },
                        }
                    }
                }
            }
            {children}
        }
    }
}

/// One ripple circle. It animates on mount and reports back when the timer that
/// stands in for `onAnimationComplete` fires, so the parent can drop it.
#[component]
fn ButtonRipple(item: Ripple, on_done: EventHandler<u32>) -> Element {
    let mut style = use_style_motion(motion_style! { scale: 0.05, opacity: 0.3 });
    let id = item.id;

    use_effect(move || {
        animate_style(
            &mut style,
            motion_style! { scale: 1.0, opacity: 0.0 },
            tween(1600, Ease::Out),
        );
        spawn(async move {
            let _ = Time::delay(Duration::from_millis(1600)).await;
            on_done.call(id);
        });
    });

    rsx! {
        span {
            class: "absolute rounded-full bg-current",
            style: "left:{item.x}px;top:{item.y}px;width:{item.size}px;height:{item.size}px;margin-left:{-(item.size / 2.0)}px;margin-top:{-(item.size / 2.0)}px;{css(&style)}",
        }
    }
}

/// The anchor twin: same variants, sizes and springs, no ripple.
#[component]
pub fn ButtonLink(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default = 0.93)] press_scale: f32,
    #[props(default)] class: Option<String>,
    #[props(default)] href: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion();
    let can_hover = use_hover_capable();
    let mut scale = use_number_motion(1.0);
    let mut hovered = use_signal(|| false);
    let mut pressed = use_signal(|| false);

    rsx! {
        a {
            href,
            class: cn!(
                "inline-flex items-center justify-center font-medium select-none",
                "transition-colors",
                variant.class(),
                size.class(),
                class,
            ),
            style: scale_style(number(&scale)),
            onpointerenter: move |_| {
                if !can_hover() || reduce() {
                    return;
                }
                hovered.set(true);
                if !pressed() {
                    animate_number(&mut scale, 1.02, spring(SPRING_PRESS));
                }
            },
            onpointerleave: move |_| {
                hovered.set(false);
                if !pressed() {
                    animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
                }
            },
            onpointerdown: move |_| {
                if reduce() {
                    return;
                }
                pressed.set(true);
                animate_number(&mut scale, press_scale, spring(SPRING_PRESS));
            },
            onpointerup: move |_| {
                if !pressed() {
                    return;
                }
                pressed.set(false);
                let target = gesture_target(false, hovered(), press_scale, can_hover() && !reduce());
                animate_number(&mut scale, target, spring(SPRING_PRESS));
            },
            onpointercancel: move |_| {
                pressed.set(false);
                animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
            },
            {children}
        }
    }
}
