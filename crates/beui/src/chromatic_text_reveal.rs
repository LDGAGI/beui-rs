//! `motion/chromatic-text-reveal.tsx` — a clipped text gradient sweeping across
//! one word at a time.
//!
//! The sweep is a CSS custom property (`--chromatic-sweep`) animated from `-14%`
//! to `114%`; the gradient string is composed around it, so the moving edge
//! paints the word as it passes. Each word is a fresh layer keyed by word and
//! index, so it re-runs its sweep, and the parent advances the word after
//! `delay + duration + pauseDuration`.

use crate::prelude::*;
use crate::scroll::{use_in_view, InViewAmount, InViewOptions};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationManager, MotionHandle, Time, TimeProvider};

const TRAIL_HALF_WIDTH: f64 = 14.0;
const REVEAL_START: f64 = -TRAIL_HALF_WIDTH;
const REVEAL_FINISH: f64 = 100.0 + TRAIL_HALF_WIDTH;

/// `CHROMATIC_PALETTE`.
pub const CHROMATIC_PALETTE: [&str; 5] = ["#60a5fa", "#818cf8", "#c084fc", "#fb7185", "#fbbf24"];

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// `composeChromaticGradient`.
fn compose_chromatic_gradient(colors: &[String], foreground: &str) -> String {
    let palette: Vec<String> = if colors.is_empty() {
        CHROMATIC_PALETTE
            .iter()
            .map(|color| color.to_string())
            .collect()
    } else {
        colors.to_vec()
    };
    let count = palette.len();
    let stops: Vec<String> = palette
        .iter()
        .enumerate()
        .map(|(index, color)| {
            let offset = if count == 1 {
                0.0
            } else {
                -TRAIL_HALF_WIDTH + (index as f64 / (count - 1) as f64) * TRAIL_HALF_WIDTH * 2.0
            };
            let operator = if offset < 0.0 { "-" } else { "+" };
            let distance = (offset.abs() * 100.0).round() / 100.0;
            format!("{color} calc(var(--chromatic-sweep) {operator} {distance}%)")
        })
        .collect();
    format!(
        "linear-gradient(90deg, {foreground} 0%, {foreground} calc(var(--chromatic-sweep) - {TRAIL_HALF_WIDTH}%), {}, transparent calc(var(--chromatic-sweep) + {TRAIL_HALF_WIDTH}%), transparent 100%)",
        stops.join(", ")
    )
}

/// Props for [`ChromaticTextReveal`].
#[derive(Props, Clone, PartialEq)]
pub struct ChromaticTextRevealProps {
    /// Sentence fragment that remains fixed while the final word changes.
    pub prefix: String,
    /// Words revealed one after another after the fixed prefix.
    pub words: Vec<String>,
    /// Colors used along the moving chromatic edge.
    #[props(default)]
    pub colors: Option<Vec<String>>,
    /// Final text color after the sweep passes.
    #[props(default = "var(--foreground)".to_string())]
    pub foreground_color: String,
    /// Sweep duration in seconds. Defaults to `1.2`.
    #[props(default = 1.2)]
    pub duration: f64,
    /// Delay before the first sweep, in seconds.
    #[props(default)]
    pub delay: f64,
    /// Rest after a word finishes revealing, in seconds. Defaults to `0.8`.
    #[props(default = 0.8)]
    pub pause_duration: f64,
    /// Returns to the first word after the final word.
    #[props(default = true)]
    pub r#loop: bool,
    /// Starts when the text enters the viewport. Defaults to `true`.
    #[props(default = true)]
    pub start_on_view: bool,
    /// Only starts on the first viewport entry. Defaults to `true`.
    #[props(default = true)]
    pub once: bool,
    #[props(default)]
    pub class: Option<String>,
}

/// A sentence whose last word is painted on by a chromatic sweep.
#[component]
pub fn ChromaticTextReveal(props: ChromaticTextRevealProps) -> Element {
    let ChromaticTextRevealProps {
        prefix,
        words,
        colors,
        foreground_color,
        duration,
        delay,
        pause_duration,
        r#loop,
        start_on_view,
        once,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let view = use_in_view(InViewOptions {
        once,
        amount: InViewAmount::Fraction(0.4),
        root: None,
    });
    let should_reveal = !start_on_view || view.value() || reduce();
    let word_index = use_signal(|| 0usize);
    let has_words = !words.is_empty();
    let active_index = if has_words {
        word_index() % words.len()
    } else {
        0
    };
    let active_word = words.get(active_index).cloned().unwrap_or_default();

    // Advance to the next word after the sweep has had its time. The source
    // does this from `onAnimationComplete`; the schedule is deterministic, so a
    // timer is equivalent and avoids a `Send` callback.
    let generation = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(0u32)));
    let is_last = has_words && active_index + 1 == words.len();
    let word_count = words.len();
    use_effect(use_reactive(
        (&active_word, &should_reveal, &r#loop),
        move |(active_word, should_reveal, r#loop)| {
            let _ = active_word;
            if !should_reveal || reduce() || word_count < 2 || (is_last && !r#loop) {
                return;
            }
            let run = generation.get() + 1;
            generation.set(run);
            let generation = generation.clone();
            let mut word_index = word_index;
            spawn(async move {
                let total = ((delay + duration + pause_duration) * 1000.0).round() as u64;
                if Time::delay(dioxus_motion::Duration::from_millis(total))
                    .await
                    .is_err()
                {
                    return;
                }
                if generation.get() == run {
                    word_index += 1;
                }
            });
        },
    ));

    let palette = colors.clone().unwrap_or_default();
    let gradient = compose_chromatic_gradient(&palette, &foreground_color);
    let sizing_words: Vec<String> = {
        let mut unique: Vec<String> = Vec::new();
        for word in &words {
            if !unique.contains(word) {
                unique.push(word.clone());
            }
        }
        unique
    };
    let class = cn!("inline-flex items-baseline", class);
    let layer_key = format!("{active_word}-{active_index}");

    rsx! {
        span { class, "data-beui-inview": view.attr(),
            span { class: "whitespace-nowrap",
                "{prefix}"
                if has_words {
                    "\u{00A0}"
                }
            }
            if has_words {
                span { class: "relative inline-grid",
                    for word in sizing_words {
                        span {
                            key: "{word}",
                            aria_hidden: "true",
                            class: "invisible col-start-1 row-start-1 whitespace-nowrap",
                            "{word}"
                        }
                    }
                    SweepWord {
                        key: "{layer_key}",
                        word: active_word.clone(),
                        gradient,
                        should_reveal,
                        reduce: reduce(),
                        duration,
                        delay,
                    }
                    span { class: "sr-only", "{active_word}" }
                }
            }
        }
    }
}

/// One word layer, keyed by word and index so it re-runs its sweep on change.
#[component]
fn SweepWord(
    word: String,
    gradient: String,
    should_reveal: bool,
    reduce: bool,
    duration: f64,
    delay: f64,
) -> Element {
    let mut sweep = use_motion(0.0f32).expect("0.0 is finite");
    let mut opacity = use_motion(0.56f32).expect("0.56 is finite");
    let mut filter = use_motion(0.0f32).expect("0.0 is finite");
    let mut lift = use_motion(0.0f32).expect("0.0 is finite");

    use_effect(use_reactive(
        (&should_reveal, &reduce),
        move |(should_reveal, reduce)| {
            if reduce {
                let instant = beui_core::ease::tween_ease_out(0);
                let _ = sweep.animate_to(1.0, instant.clone());
                let _ = opacity.animate_to(1.0, instant.clone());
                let _ = filter.animate_to(1.0, instant.clone());
                let _ = lift.animate_to(1.0, instant);
                return;
            }
            let target = if should_reveal { 1.0 } else { 0.0 };
            let sweep_config =
                beui_core::ease::tween_ease_in_out((duration * 1000.0).round() as u64).with_delay(
                    dioxus_motion::Duration::from_millis((delay * 1000.0).round() as u64),
                );
            let _ = sweep.animate_to(target, sweep_config);
            let _ = opacity.animate_to(target.max(0.56), beui_core::ease::tween_ease_out(280));
            let _ = filter.animate_to(target, beui_core::ease::tween_ease_out(360));
            let _ = lift.animate_to(target, beui_core::ease::tween_ease_out(360));
        },
    ));

    let p = f64::from(current(sweep).clamp(0.0, 1.0));
    let sweep_value = REVEAL_START + p * (REVEAL_FINISH - REVEAL_START);
    let op = f64::from(current(opacity));
    let blur = 6.0 * (1.0 - f64::from(current(filter).clamp(0.0, 1.0)));
    let y = 5.0 * (1.0 - f64::from(current(lift).clamp(0.0, 1.0)));
    let style = format!(
        "--chromatic-sweep:{sweep_value}%;--chromatic-gradient:{gradient};background-image:var(--chromatic-gradient);background-size:100% 100%;background-repeat:no-repeat;opacity:{op};filter:blur({blur}px);transform:translateY({y}px)"
    );

    rsx! {
        span {
            aria_hidden: "true",
            class: "absolute start-0 top-0 whitespace-nowrap bg-clip-text text-transparent [contain:paint]",
            style,
            "{word}"
        }
    }
}
