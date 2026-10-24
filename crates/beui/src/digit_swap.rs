//! `motion/digit-swap.tsx` — fixed-slot glyphs that roll when their value
//! changes.
//!
//! Each slot holds an invisible in-flow copy of its character, so the slot has
//! a real text baseline (it lines up with neighbouring text) and punctuation
//! keeps its natural width. Digits use a fixed `1ch` slot so rolls never
//! jitter. Slots clip with `clip-path`, which, unlike `overflow: hidden`, leaves
//! the baseline alone.
//!
//! The source wraps each slot in its own `AnimatePresence`; here the incoming
//! and outgoing glyphs are two layers driven by one progress value, which is
//! the same two-element cross-roll without the per-slot presence boundary.
//! `initial={false}` is the `first` flag: the first mount lands at rest.

use std::cell::Cell;
use std::rc::Rc;

use crate::prelude::*;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationManager, MotionHandle};

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// Which way the next glyph enters from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DigitSwapDirection {
    #[default]
    Up,
    Down,
}

/// Props for [`DigitSwap`].
#[derive(Props, Clone, PartialEq)]
pub struct DigitSwapProps {
    /// Numeric or masked value rendered in fixed character slots.
    pub value: String,
    /// Replays every glyph when the value itself contains unchanged characters.
    #[props(default)]
    pub animation_key: Option<String>,
    #[props(default)]
    pub direction: DigitSwapDirection,
    /// Per-glyph transition duration in seconds. Defaults to `0.18`.
    #[props(default = 0.18)]
    pub duration: f64,
    /// Delay in seconds between neighbouring glyphs. Defaults to `0.006`.
    #[props(default = 0.006)]
    pub stagger: f64,
    /// Number of final characters that receive `suffix_class`.
    #[props(default)]
    pub suffix_length: usize,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub glyph_class: Option<String>,
    #[props(default)]
    pub suffix_class: Option<String>,
}

/// Fixed-slot digits and mask glyphs that roll when their value changes.
#[component]
pub fn DigitSwap(props: DigitSwapProps) -> Element {
    let DigitSwapProps {
        value,
        animation_key,
        direction,
        duration,
        stagger,
        suffix_length,
        class,
        glyph_class,
        suffix_class,
    } = props;

    let reduce = use_reduced_motion();
    let text = value.clone();
    let suffix_start = text.chars().count().saturating_sub(suffix_length);
    let class = cn!("inline-flex items-baseline whitespace-nowrap", class);

    rsx! {
        span {
            "data-slot": "digit-swap",
            "data-direction": if direction == DigitSwapDirection::Up { "up" } else { "down" },
            class,
            span { class: "sr-only", "{text}" }
            span { aria_hidden: "true", class: "inline-flex items-baseline",
                for (position, character) in text.chars().enumerate() {
                    if character == ' ' {
                        span { key: "space-{position}", class: "inline-block w-[0.7ch]" }
                    } else {
                        GlyphSlot {
                            key: "slot-{position}",
                            character: character.to_string(),
                            version: match &animation_key {
                                Some(key) => format!("{position}-{character}-{key}"),
                                None => format!("{position}-{character}"),
                            },
                            position,
                            direction,
                            duration,
                            stagger,
                            reduce: reduce(),
                            is_digit: character.is_ascii_digit(),
                            is_suffix: position >= suffix_start,
                            glyph_class: glyph_class.clone(),
                            suffix_class: suffix_class.clone(),
                        }
                    }
                }
            }
        }
    }
}

/// One slot: the invisible baseline copy plus the rolling glyph layers.
#[component]
fn GlyphSlot(
    character: String,
    version: String,
    position: usize,
    direction: DigitSwapDirection,
    duration: f64,
    stagger: f64,
    reduce: bool,
    is_digit: bool,
    is_suffix: bool,
    glyph_class: Option<String>,
    suffix_class: Option<String>,
) -> Element {
    let progress = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let mut displayed = use_signal(|| character.clone());
    let mut outgoing = use_signal(|| None::<String>);
    let first = use_hook(|| Rc::new(Cell::new(true)));

    let next_character = character.clone();
    use_effect(use_reactive((&version,), move |_| {
        let next = next_character.clone();
        let previous = displayed();
        if previous != next {
            outgoing.set(Some(previous));
            displayed.set(next);
        }
        let mut handle = progress;
        handle.reset();
        if first.get() {
            first.set(false);
            let _ = handle.animate_to(1.0, beui_core::ease::tween_ease_out(0));
            return;
        }
        let seconds = if reduce { duration.min(0.12) } else { duration };
        let delay = if reduce {
            0.0
        } else {
            position as f64 * stagger
        };
        let config = beui_core::ease::tween_ease_out((seconds * 1000.0).round() as u64).with_delay(
            dioxus_motion::Duration::from_millis((delay * 1000.0).round() as u64),
        );
        let _ = handle.animate_to(1.0, config);
    }));

    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let (enter_from, exit_to) = match direction {
        DigitSwapDirection::Up => (45.0, -45.0),
        DigitSwapDirection::Down => (-45.0, 45.0),
    };

    let incoming_style = if reduce {
        format!("opacity:{p};transform:none")
    } else {
        format!(
            "opacity:{p};transform:translateY({}%)",
            enter_from * (1.0 - p)
        )
    };
    let outgoing_style = if reduce {
        format!("opacity:{};transform:none", 1.0 - p)
    } else {
        format!("opacity:{};transform:translateY({}%)", 1.0 - p, exit_to * p)
    };

    let layer_class = cn!(
        "absolute inset-0 flex items-center justify-center leading-none",
        glyph_class.clone(),
        if is_suffix {
            suffix_class.clone()
        } else {
            None
        },
    );
    let slot_class = cn!(
        "relative inline-block h-[1.1em] shrink-0 leading-[1.1em] [clip-path:inset(0)]",
        if is_digit { "w-[1ch] text-center" } else { "" },
    );
    let slot = character.clone();

    rsx! {
        span { "data-slot": "digit-swap-glyph", class: slot_class,
            span { class: "invisible", "{slot}" }
            if let Some(previous) = outgoing() {
                span { class: layer_class.clone(), style: outgoing_style, "{previous}" }
            }
            span { class: layer_class, style: incoming_style, "{character}" }
        }
    }
}
