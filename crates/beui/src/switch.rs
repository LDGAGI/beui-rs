//! `motion/switch.tsx` — the switch.
//!
//! The thumb travels the track on a heavy spring (`{800, 80, 4}` — high mass
//! keeps it weighty without wobble) and squishes to `0.9` while the pointer is
//! down. The source positions the thumb with flex `justify-start` / `justify-end`
//! and lets motion's `layout` animate the jump; the track is a fixed `w-12` with
//! `px-1` and a `w-5` thumb, so the travel is a constant `20px`. Animating that
//! translate directly only works if the flex alignment stays at `justify-start`
//! in **both** states: keeping the source's `justify-end` on the checked track
//! placed the thumb twice (once by layout, once by the translate), landing it
//! `20px` further right — `16px` of it past the track's edge, on top of the
//! label. That is this file's one deliberate class deviation from the source.
//!
//! Pressing a *disabled* switch shakes the thumb, which is why the shake lives on
//! an inner layer: it composes with the travel instead of fighting it for the
//! same transform.

use dioxus::prelude::*;
use dioxus_motion::prelude::{AnimationConfig, Duration};
use dioxus_motion::{AnimationManager, KeyframeAnimation};

use beui_core::cn;
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;

use crate::motion::{
    animate_number, animate_style, css, motion_style, number, tween, use_number_motion,
    use_style_motion, Ease,
};

/// `THUMB_SPRING` — heavy, deliberate thumb.
const THUMB_SPRING: dioxus_motion::prelude::Spring = dioxus_motion::prelude::Spring {
    stiffness: 800.0,
    damping: 80.0,
    mass: 4.0,
};

/// The track is `w-12` (48px) with `px-1` (4px) and a `w-5` (20px) thumb.
const TRAVEL: f32 = 20.0;

fn thumb_config(reduce: bool) -> AnimationConfig {
    if reduce {
        tween(0, Ease::Out)
    } else {
        AnimationConfig::spring(THUMB_SPRING)
    }
}

#[component]
pub fn Switch(
    checked: bool,
    on_checked_change: EventHandler<bool>,
    #[props(default)] disabled: bool,
    #[props(default)] label: Option<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let id = use_hook(|| format!("beui-switch-{}", platform::next_scope_id()));
    let reduce = use_reduced_motion();
    let mut pressed = use_signal(|| false);

    let mut thumb = use_style_motion(motion_style! { x: if checked { TRAVEL } else { 0.0 } });
    let mut shake = use_number_motion(0.0);

    let squish = !disabled && pressed() && !reduce();

    // The thumb travel and the squish share one transform; a re-target that
    // repeats the unchanged axis is a no-op. `pressed`/`reduce` are read inside
    // the effect so the squish re-targets on press as well as on `checked`.
    use_effect(use_reactive!(|checked, disabled| {
        let squish = !disabled && pressed() && !reduce();
        let target = motion_style! {
            x: if checked { TRAVEL } else { 0.0 },
            scale: if squish { 0.9 } else { 1.0 },
        };
        animate_style(&mut thumb, target, thumb_config(reduce()));
    }));

    // Disabled press: a small horizontal shake, the source's
    // `animate(x, [0,-2,2,-1,0], { delay: 0.2, duration: 0.6 })`.
    //
    // The keyframe API has no separate delay, so the delay is folded into the
    // offsets: the clip runs `delay + duration` = 800ms and each source
    // keyframe keeps its position inside that clip. Motion spaces the five
    // values evenly across `duration` after holding the first through the
    // delay, so the source's absolute times are 200/350/500/650/800ms — that
    // is 0.25/0.4375/0.625/0.8125/1.0 here. Spacing them evenly across the
    // whole 800ms instead (the first cut) compressed the shake into 560ms.
    use_effect(use_reactive!(|disabled| {
        if reduce() || !disabled || !pressed() {
            return;
        }
        let frames = KeyframeAnimation::new(Duration::from_millis(800))
            .add_keyframe(0.0f32, 0.0, None)
            .expect("valid shake")
            .add_keyframe(0.0, 0.25, None)
            .expect("valid shake")
            .add_keyframe(-2.0, 0.4375, None)
            .expect("valid shake")
            .add_keyframe(2.0, 0.625, None)
            .expect("valid shake")
            .add_keyframe(-1.0, 0.8125, None)
            .expect("valid shake")
            .add_keyframe(0.0, 1.0, None)
            .expect("valid shake");
        let _ = shake.animate_keyframes(frames);
    }));

    let settle = move |_event: Event<PointerData>| {
        pressed.set(false);
        if !reduce() {
            animate_number(&mut shake, 0.0, thumb_config(false));
        }
    };

    rsx! {
        span { class: cn!("inline-flex items-center gap-3", class),
            button {
                id: id.clone(),
                r#type: "button",
                role: "switch",
                "aria-checked": if checked { "true" } else { "false" },
                "aria-label": aria_label,
                disabled,
                "data-state": if checked { "checked" } else { "unchecked" },
                class: cn!(
                    "group peer inline-flex h-7 w-12 shrink-0 cursor-pointer items-center px-1 rounded-full outline-none transition-colors duration-200",
                    "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    "disabled:cursor-not-allowed disabled:opacity-60",
                    "justify-start",
                    if checked { "bg-primary" } else { "bg-muted-foreground/60" },
                ),
                onpointerdown: move |_| {
                    pressed.set(true);
                },
                onpointerup: settle,
                onpointerleave: settle,
                onpointercancel: settle,
                onclick: move |_| {
                    if !disabled {
                        on_checked_change.call(!checked);
                    }
                },
                div {
                    style: css(&thumb),
                    class: "pointer-events-none block h-5 w-5",
                    div {
                        style: "transform:translateX({number(&shake)}px)",
                        class: "h-5 w-5 rounded-full bg-background shadow-md",
                        // The source's stretch spacer, kept verbatim: `size-5`
                        // with `ml-1`/`mr-1`, a child of the painted circle. The
                        // circle is a fixed `w-5`, so the spacer overflows
                        // invisibly and stretches nothing — the source squishes
                        // about the circle's own centre. Carrying these margins on
                        // the painted circle instead (this port's first cut)
                        // dragged it 3.6px sideways on press, which the source
                        // never does.
                        div {
                            class: cn!(
                                "size-5",
                                if squish { if checked { "ml-1" } else { "mr-1" } } else { "" },
                            ),
                        },
                    }
                }
            }
            if let Some(label) = label {
                label { r#for: id.clone(), class: "cursor-pointer text-sm text-foreground", "{label}" }
            }
        }
    }
}
