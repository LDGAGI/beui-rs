//! `motion/card-folder.tsx` — a landscape card tucked into an animated folder
//! sleeve.
//!
//! One `progress` value drives everything: the card lifts and grows by
//! `sin(π·p)` while the sleeve's two layers collapse into its bottom seam
//! (`translateY(18%)` + `scaleY(0.18)` / `rotateX(-72deg)`) and the purse fades
//! out after `p = 0.76`. The morph is a 0.28s `EASE_IN_OUT` tween; reduced
//! motion swaps the transforms for a plain opacity cross-fade. The number and
//! CVV roll through [`DigitSwap`](crate::DigitSwap), as in the source; a
//! separate privacy control toggles them.

use crate::digit_swap::{DigitSwap, DigitSwapDirection};
use crate::icons::{EllipsisVertical, Eye, EyeOff};
use crate::prelude::*;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationManager, MotionHandle};

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// Props for [`CardFolder`].
#[derive(Props, Clone, PartialEq)]
pub struct CardFolderProps {
    pub title: String,
    pub card_number: String,
    pub expiry: String,
    pub cvv: String,
    /// The card's artwork.
    pub card: Element,
    #[props(default)]
    pub open: Option<bool>,
    #[props(default)]
    pub default_open: bool,
    #[props(default)]
    pub on_open_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub details_visible: Option<bool>,
    #[props(default)]
    pub default_details_visible: bool,
    #[props(default)]
    pub on_details_visible_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub on_click: Option<EventHandler<()>>,
    #[props(default)]
    pub on_action: Option<EventHandler<()>>,
    #[props(default)]
    pub aria_label: Option<String>,
    #[props(default)]
    pub action_label: Option<String>,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub card_class: Option<String>,
}

/// A card in an animated folder sleeve.
#[component]
pub fn CardFolder(props: CardFolderProps) -> Element {
    let CardFolderProps {
        title,
        card_number,
        expiry,
        cvv,
        card,
        open,
        default_open,
        on_open_change,
        details_visible,
        default_details_visible,
        on_details_visible_change,
        on_click,
        on_action,
        aria_label,
        action_label,
        disabled,
        class,
        card_class,
    } = props;

    let reduce = use_reduced_motion();
    let internal_open = use_signal(|| default_open);
    let internal_details = use_signal(|| default_details_visible);
    let open_controlled = open.is_some();
    let details_controlled = details_visible.is_some();
    let is_open = match open {
        Some(open) => open,
        None => internal_open(),
    };
    let are_details_visible = match details_visible {
        Some(visible) => visible,
        None => internal_details(),
    };
    let reduce_value = reduce();

    let mut progress = use_motion(if is_open { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    use_effect(use_reactive((&is_open,), move |(is_open,)| {
        let config = if reduce_value {
            beui_core::ease::tween_ease_out(0)
        } else {
            beui_core::ease::tween_ease_in_out(280)
        };
        let _ = progress.animate_to(if is_open { 1.0 } else { 0.0 }, config);
    }));

    let set_open = move |next: bool| {
        if disabled {
            return;
        }
        if !open_controlled {
            let mut signal = internal_open;
            signal.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    };
    let set_details = move |next: bool| {
        if disabled {
            return;
        }
        if !details_controlled {
            let mut signal = internal_details;
            signal.set(next);
        }
        if let Some(handler) = on_details_visible_change {
            handler.call(next);
        }
    };

    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let lift = (std::f64::consts::PI * p).sin();
    let purse_opacity = if p <= 0.76 {
        1.0
    } else {
        (1.0 - (p - 0.76) / 0.24).clamp(0.0, 1.0)
    };

    let card_style = if reduce_value {
        "transform:none".to_string()
    } else {
        format!(
            "transform:translateY({}%) scale({})",
            -8.0 * lift,
            1.0 + 0.01 * lift
        )
    };
    let back_style = if reduce_value {
        format!("opacity:{}", if is_open { 0.0 } else { 1.0 })
    } else {
        format!(
            "opacity:{purse_opacity};transform:translateY({}%) scaleY({})",
            18.0 * p,
            1.0 - 0.82 * p
        )
    };
    let front_style = if reduce_value {
        format!("opacity:{}", if is_open { 0.0 } else { 1.0 })
    } else {
        format!(
            "opacity:{purse_opacity};transform:translateY({}%) rotateX({}deg)",
            18.0 * p,
            -72.0 * p
        )
    };
    let eye_duration = if reduce_value { 0 } else { 120 };
    let show_eye_style = format!(
        "opacity:{};scale:{};filter:blur({}px);transition:opacity {eye_duration}ms {EASE_OUT_CSS}, scale {eye_duration}ms {EASE_OUT_CSS}, filter {eye_duration}ms {EASE_OUT_CSS}",
        if are_details_visible { 0.0 } else { 1.0 },
        if are_details_visible { 0.25 } else { 1.0 },
        if are_details_visible { 4.0 } else { 0.0 },
    );
    let hide_eye_style = format!(
        "opacity:{};scale:{};filter:blur({}px);transition:opacity {eye_duration}ms {EASE_OUT_CSS}, scale {eye_duration}ms {EASE_OUT_CSS}, filter {eye_duration}ms {EASE_OUT_CSS}",
        if are_details_visible { 1.0 } else { 0.0 },
        if are_details_visible { 1.0 } else { 0.25 },
        if are_details_visible { 0.0 } else { 4.0 },
    );

    // Numbers, normalised the way the source does.
    let digits: String = card_number.chars().filter(char::is_ascii_digit).collect();
    let last_four = if digits.len() >= 4 {
        digits[digits.len() - 4..].to_string()
    } else {
        format!("{:•>4}", digits)
    };
    let revealed = if digits.is_empty() {
        last_four.clone()
    } else {
        digits
            .as_bytes()
            .chunks(4)
            .map(|chunk| String::from_utf8_lossy(chunk).to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let masked_number = format!("•••• •••• •••• {last_four}");
    let masked_cvv = "•".repeat(cvv.len().max(3));
    let default_aria = format!(
        "{} {title}, card ending in {last_four}, expires {expiry}",
        if is_open { "Close" } else { "Open" }
    );
    let aria = aria_label.unwrap_or(default_aria);
    let action_aria = action_label.unwrap_or_else(|| format!("Open actions for {title}"));
    let action_y = -50.0 - 14.0 * p;

    let container_class = cn!(
        "relative aspect-[1029/592] w-96 max-w-full select-none [perspective:1200px]",
        class,
    );
    let card_class = cn!(
        "absolute left-[4.7%] right-[4.7%] top-0 z-10 aspect-[1.586/1] overflow-hidden rounded-xl border border-foreground/10 bg-background [transform-origin:center_bottom] will-change-transform",
        card_class,
    );
    let title_for_label = title.clone();
    let cvv_value = if are_details_visible {
        cvv.clone()
    } else {
        masked_cvv
    };
    let number_value = if are_details_visible {
        revealed
    } else {
        masked_number
    };
    let animation_key = if are_details_visible {
        "revealed"
    } else {
        "masked"
    }
    .to_string();
    let direction = if are_details_visible {
        DigitSwapDirection::Up
    } else {
        DigitSwapDirection::Down
    };
    let number_glyph_class = if are_details_visible {
        "text-foreground"
    } else {
        "text-muted-foreground"
    }
    .to_string();

    rsx! {
        div {
            "data-open": if is_open { "true" } else { "false" },
            "data-details-visible": if are_details_visible { "true" } else { "false" },
            class: container_class,
            button {
                r#type: "button",
                disabled,
                aria_label: aria,
                aria_expanded: if is_open { "true" } else { "false" },
                class: "absolute inset-0 block rounded-2xl text-left outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-4 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 active:scale-[0.96] transition-transform",
                onclick: move |_| {
                    set_open(!is_open);
                    if let Some(handler) = on_click {
                        handler.call(());
                    }
                },
                span {
                    aria_hidden: "true",
                    "data-slot": "card-folder-back",
                    class: "absolute inset-x-0 bottom-0 top-[10%] rounded-2xl border border-foreground/10 bg-background [transform-origin:center_bottom]",
                    style: back_style,
                }
                span { aria_hidden: "true", class: card_class, style: card_style, {card} }
            }
            span {
                aria_hidden: if is_open { "true" } else { "false" },
                inert: if is_open { Some("true") } else { None },
                class: "pointer-events-none absolute inset-x-0 bottom-0 top-1/2 z-20 [backface-visibility:hidden] [transform-origin:center_bottom]",
                style: front_style,
                svg {
                    "aria-hidden": "true",
                    view_box: "0 0 384 110",
                    preserve_aspect_ratio: "none",
                    class: "absolute inset-0 size-full overflow-visible",
                    path {
                        d: "M0 17C15 7 31 4 49 4H87C110 4 126 17 144 32L158 44C176 59 206 59 225 43L240 30C257 16 271 4 295 4H335C354 4 370 8 384 18V94C384 103 377 110 368 110H16C7 110 0 103 0 94Z",
                        fill: "var(--background)",
                        stroke: "var(--foreground)",
                        stroke_opacity: "0.12",
                        vector_effect: "non-scaling-stroke",
                    }
                    path {
                        d: "M10 21C22 13 35 11 51 11H85C105 11 120 23 137 37L153 50C175 68 208 68 231 49L246 36C262 23 275 11 297 11H333C350 11 363 14 374 22V89C374 97 369 101 360 101H24C15 101 10 96 10 89Z",
                        fill: "none",
                        stroke: "var(--foreground)",
                        stroke_dasharray: "5 5",
                        stroke_linecap: "round",
                        stroke_opacity: "0.22",
                        vector_effect: "non-scaling-stroke",
                    }
                }
                span { class: "absolute inset-x-[5%] inset-y-0 z-10 flex min-w-0 flex-col justify-between py-4",
                    span { class: "flex items-start justify-between pr-2",
                        button {
                            r#type: "button",
                            disabled,
                            tabindex: if is_open { Some("-1") } else { None },
                            aria_label: if are_details_visible { "Hide card details" } else { "Show card details" },
                            aria_pressed: if are_details_visible { "true" } else { "false" },
                            class: cn!(
                                "z-30 flex size-10 shrink-0 items-center justify-center rounded-full text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 active:scale-[0.96]",
                                if is_open { "pointer-events-none" } else { "pointer-events-auto" },
                            ),
                            onclick: move |event: Event<MouseData>| {
                                event.stop_propagation();
                                set_details(!are_details_visible);
                            },
                            span { class: "relative flex items-center justify-center",
                                span { class: "absolute flex items-center justify-center", style: show_eye_style,
                                    Eye { class: Some("size-4".to_string()) }
                                }
                                span { class: "flex items-center justify-center", style: hide_eye_style,
                                    EyeOff { class: Some("size-4".to_string()) }
                                }
                            }
                        }
                        span { class: "flex shrink-0 items-end gap-3.5 pt-2",
                            span { class: "flex flex-col gap-0.5",
                                span { class: "text-[9px] font-medium uppercase tracking-[0.12em] text-muted-foreground/65", "Expiry" }
                                span { class: "text-xs font-medium text-foreground tabular-nums", "{expiry}" }
                            }
                            span { class: "flex flex-col gap-0.5",
                                span { class: "text-[9px] font-medium uppercase tracking-[0.12em] text-muted-foreground/65", "CVV" }
                                DigitSwap {
                                    value: cvv_value,
                                    animation_key: Some(animation_key.clone()),
                                    direction,
                                    class: "text-xs font-medium text-foreground tabular-nums",
                                }
                            }
                        }
                    }
                    span { class: "flex min-w-0 items-baseline justify-between gap-4",
                        span { class: "truncate text-lg font-medium leading-tight text-foreground", "{title_for_label}" }
                        DigitSwap {
                            value: number_value,
                            animation_key: Some(animation_key),
                            direction,
                            suffix_length: 4,
                            glyph_class: Some(number_glyph_class),
                            suffix_class: Some("text-foreground".to_string()),
                            class: "truncate font-mono text-xs tracking-[0.08em] tabular-nums",
                        }
                    }
                }
            }
            if let Some(on_action) = on_action {
                button {
                    r#type: "button",
                    disabled,
                    aria_label: action_aria,
                    class: "absolute right-[2.6%] top-[10%] z-10 flex size-10 items-center justify-center rounded-full text-white/65 outline-none transition-colors hover:bg-white/10 hover:text-white focus-visible:ring-2 focus-visible:ring-white disabled:cursor-not-allowed disabled:opacity-50",
                    style: "transform:translateY({action_y}%)",
                    onclick: move |event: Event<MouseData>| {
                        event.stop_propagation();
                        on_action.call(());
                    },
                    EllipsisVertical { class: Some("size-4".to_string()) }
                }
            }
        }
    }
}
