//! `motion/text-scramble.tsx` — characters resolving left to right out of a
//! glyph soup.
//!
//! A `requestAnimationFrame` algorithm in the source; here a 40ms timer loop
//! with the same progress arithmetic and the same per-position settle rule.
//! Reduced motion, or an empty glyph set, shows the text straight away.

use std::cell::Cell;
use std::rc::Rc;

use crate::prelude::*;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

/// `DEFAULT_GLYPHS`.
pub const DEFAULT_GLYPHS: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ0123456789#%&@$?/";

/// A tiny xorshift64* so the scramble is reproducible per run without pulling
/// in `rand` (and without `js_sys::Math::random`, which is web-only).
fn next_random(state: &mut u64) -> f64 {
    let mut value = *state;
    value ^= value >> 12;
    value ^= value << 25;
    value ^= value >> 27;
    *state = value;
    let mixed = value.wrapping_mul(0x2545_F491_4F6C_DD1D);
    ((mixed >> 11) as f64) / ((1u64 << 53) as f64)
}

/// Props for [`TextScramble`].
#[derive(Props, Clone, PartialEq)]
pub struct TextScrambleProps {
    /// The final text the scramble resolves to.
    pub text: String,
    /// Maximum animation duration in milliseconds.
    #[props(default)]
    pub duration: Option<f64>,
    /// Characters sampled while unresolved positions scramble.
    #[props(default = DEFAULT_GLYPHS.to_string())]
    pub glyphs: String,
    /// Scramble on mount as well as on change. The source always does; the
    /// `NotFound` glitch variant relies on it.
    #[props(default)]
    pub auto_play: bool,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub style: Option<String>,
}

/// Scramble to `text`.
#[component]
pub fn TextScramble(props: TextScrambleProps) -> Element {
    let TextScrambleProps {
        text,
        duration,
        glyphs,
        auto_play,
        class,
        style,
    } = props;

    let reduce = use_reduced_motion();
    let mut display = use_signal(|| text.clone());
    let mounted = use_hook(|| Rc::new(Cell::new(false)));
    let generation = use_hook(|| Rc::new(Cell::new(0u32)));

    use_effect(use_reactive(
        (&text, &duration, &glyphs, &reduce),
        move |(text, duration, glyphs, reduce)| {
            if !mounted.get() {
                mounted.set(true);
                if !auto_play {
                    display.set(text.clone());
                    return;
                }
            }
            if reduce() || glyphs.is_empty() {
                display.set(text.clone());
                return;
            }

            let run = generation.get() + 1;
            generation.set(run);
            let generation = generation.clone();
            let mut display = display;
            let characters: Vec<char> = text.chars().collect();
            let glyph_characters: Vec<char> = glyphs.chars().collect();
            let animation_duration =
                duration.unwrap_or_else(|| (characters.len() as f64 * 32.0).clamp(420.0, 760.0));
            let final_text = text.clone();

            spawn(async move {
                let started = Time::now();
                let mut seed = 0x9E37_79B9_7F4A_7C15u64 ^ u64::from(run);
                loop {
                    if generation.get() != run {
                        return;
                    }
                    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
                    let progress = (elapsed / animation_duration).min(1.0);
                    let settled = (progress * characters.len() as f64).floor() as usize;
                    let mut next = String::with_capacity(characters.len());
                    for (index, character) in characters.iter().enumerate() {
                        if index < settled || *character == ' ' {
                            next.push(*character);
                        } else {
                            let pick =
                                (next_random(&mut seed) * glyph_characters.len() as f64) as usize;
                            next.push(glyph_characters[pick.min(glyph_characters.len() - 1)]);
                        }
                    }
                    display.set(next);
                    if progress >= 1.0 {
                        display.set(final_text.clone());
                        return;
                    }
                    if Time::delay(Duration::from_millis(40)).await.is_err() {
                        return;
                    }
                }
            });
        },
    ));

    let text_now = text.clone();
    let shown = if reduce() { text.clone() } else { display() };
    let class = cn!("inline-block whitespace-pre", class);

    rsx! {
        span { class, style: style.unwrap_or_default(),
            span { class: "sr-only", "{text_now}" }
            span { aria_hidden: "true", "{shown}" }
        }
    }
}
