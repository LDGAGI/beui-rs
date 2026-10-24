//! `motion/number-ticker.tsx` — digits rolling on a 0–9 column.
//!
//! Each digit is a column of `0…9` translated to its place; the column animates
//! on `EASE_OUT` to the new digit, with an optional blur that clears as it
//! lands. Glyphs are keyed by place value (position from the right), so a
//! changing digit keeps its identity and rolls instead of remounting, and a
//! growing number adds glyphs on the left without re-keying the ones already on
//! screen.

use crate::prelude::*;
use crate::scroll::{use_in_view, InViewAmount, InViewOptions};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationManager, MotionHandle, Time, TimeProvider};
use dioxus_motion::KeyframeAnimation;

const DIGIT_HEIGHT_EM: f64 = 1.1;

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

fn group_integer(digits: &str) -> String {
    let (sign, digits) = match digits.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", digits),
    };
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(character);
    }
    format!("{sign}{out}")
}

/// `toLocaleString(undefined, {minimumFractionDigits, maximumFractionDigits})`
/// for `en-US`.
fn locale_fixed(value: f64, places: usize) -> String {
    let fixed = format!("{value:.places$}");
    match fixed.split_once('.') {
        Some((integer, fraction)) => format!("{}.{}", group_integer(integer), fraction),
        None => group_integer(&fixed),
    }
}

/// Props for [`NumberTicker`].
#[derive(Props, Clone, PartialEq)]
pub struct NumberTickerProps {
    pub value: f64,
    /// Digits to pad to (left).
    #[props(default)]
    pub pad: Option<usize>,
    /// Fraction digits to keep. Defaults to `0`.
    #[props(default)]
    pub decimals: usize,
    /// Per-digit roll duration in seconds. Defaults to `0.9`.
    #[props(default = 0.9)]
    pub duration: f64,
    /// Stagger between digits. Defaults to `0.04`.
    #[props(default = 0.04)]
    pub stagger: f64,
    /// Render only after the element enters the viewport. Defaults to `true`.
    #[props(default = true)]
    pub start_on_view: bool,
    #[props(default)]
    pub prefix: Option<String>,
    #[props(default)]
    pub suffix: Option<String>,
    /// Add a small blur during digit rolls.
    #[props(default)]
    pub blur: bool,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub digit_class: Option<String>,
    /// Insert locale group separators (commas).
    #[props(default)]
    pub locale: bool,
    /// Custom formatter.
    #[props(default)]
    pub format: Option<Callback<f64, String>>,
}

/// A number whose digits roll into place.
#[component]
pub fn NumberTicker(props: NumberTickerProps) -> Element {
    let NumberTickerProps {
        value,
        pad,
        decimals,
        duration,
        stagger,
        start_on_view,
        prefix,
        suffix,
        blur,
        class,
        digit_class,
        locale,
        format,
    } = props;

    let reduce = use_reduced_motion();
    let view = use_in_view(InViewOptions {
        once: true,
        amount: InViewAmount::Fraction(0.6),
        root: None,
    });
    let mut armed = use_signal(|| !start_on_view);
    let in_view = view.signal();
    use_effect(use_reactive((&start_on_view,), move |(start_on_view,)| {
        if start_on_view && in_view() {
            armed.set(true);
        }
    }));

    let places = decimals;
    let factor = 10f64.powi(places as i32);
    let rounded = (value * factor).round() / factor;
    let formatted = match &format {
        Some(format) => format.call(rounded),
        None if locale => locale_fixed(rounded, places),
        None => format!("{rounded:.places$}"),
    };
    let text = match pad {
        Some(width) => {
            let mut padded = formatted;
            while padded.len() < width {
                padded.insert(0, '0');
            }
            padded
        }
        None => formatted,
    };

    let characters: Vec<char> = text.chars().collect();
    let glyph_count = characters.len();
    let readable = format!(
        "{}{}{}",
        prefix.clone().unwrap_or_default(),
        text,
        suffix.clone().unwrap_or_default()
    );

    // Stagger is an entrance flourish; after it has played, value changes roll
    // every digit immediately.
    let entered = use_signal(|| false);
    let generation = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(0u32)));
    use_effect(use_reactive(
        (&duration, &stagger, &glyph_count, &start_on_view),
        move |(duration, stagger, glyph_count, _start_on_view)| {
            if !armed() || entered() {
                return;
            }
            let run = generation.get() + 1;
            generation.set(run);
            let generation = generation.clone();
            let mut entered = entered;
            spawn(async move {
                let total = ((duration + glyph_count as f64 * stagger) * 1000.0).round() as u64;
                if Time::delay(dioxus_motion::Duration::from_millis(total))
                    .await
                    .is_err()
                {
                    return;
                }
                if generation.get() == run {
                    entered.set(true);
                }
            });
        },
    ));

    let class = cn!("inline-flex items-baseline tabular-nums", class);

    rsx! {
        span { class, "data-beui-inview": view.attr(),
            span { class: "sr-only", "{readable}" }
            span { aria_hidden: "true", class: "inline-flex items-baseline",
                if let Some(prefix) = prefix {
                    span { "{prefix}" }
                }
                for (index, character) in characters.iter().enumerate() {
                    if character.is_ascii_digit() {
                        TickerDigit {
                            key: "g-{glyph_count - 1 - index}",
                            digit: if armed() { character.to_digit(10).unwrap_or(0) as u8 } else { 0 },
                            delay: if entered() { 0.0 } else { index as f64 * stagger },
                            duration,
                            blur,
                            reduce: reduce(),
                            class: digit_class.clone(),
                        }
                    } else {
                        span { key: "g-{glyph_count - 1 - index}", class: "inline-block", "{character}" }
                    }
                }
                if let Some(suffix) = suffix {
                    span { "{suffix}" }
                }
            }
        }
    }
}

/// One rolling digit column.
#[component]
fn TickerDigit(
    digit: u8,
    delay: f64,
    duration: f64,
    blur: bool,
    reduce: bool,
    class: Option<String>,
) -> Element {
    let position = use_motion(digit as f32).expect("a digit is finite");
    let blur_progress = use_motion(1.0f32).expect("1.0 is finite");

    use_effect(use_reactive(
        (&digit, &duration, &delay, &blur, &reduce),
        move |(digit, duration, delay, blur, reduce)| {
            let mut handle = position;
            if reduce {
                let _ = handle.animate_to(digit as f32, beui_core::ease::tween_ease_out(0));
            } else {
                let config = beui_core::ease::tween_ease_out((duration * 1000.0).round() as u64)
                    .with_delay(dioxus_motion::Duration::from_millis(
                        (delay * 1000.0).round() as u64,
                    ));
                let _ = handle.animate_to(digit as f32, config);
            }

            if blur && !reduce {
                // A keyframe animation restarts from its first frame, which is
                // how the source's `filter: ["blur(10px)", "blur(0px)"]` keyframes
                // behave on every digit change.
                let keyframes = KeyframeAnimation::new(dioxus_motion::Duration::from_millis(
                    ((duration * 0.75).min(0.32) * 1000.0).round() as u64,
                ))
                .add_keyframe(10.0f32, 0.0, Some(beui_core::ease::ease_out_easer))
                .and_then(|animation| animation.add_keyframe(0.0, 1.0, None));
                if let Ok(animation) = keyframes {
                    let mut handle = blur_progress;
                    handle.delay(dioxus_motion::Duration::from_millis(
                        (delay * 1000.0).round() as u64,
                    ));
                    let _ = handle.animate_keyframes(animation);
                }
            }
        },
    ));

    let position = f64::from(current(position));
    let blur_value = 10.0 * (1.0 - f64::from(current(blur_progress).clamp(0.0, 1.0)));
    let style = format!(
        "transform:translateY(-{}em);filter:blur({blur_value}px)",
        position * DIGIT_HEIGHT_EM
    );
    let class = cn!(
        "relative inline-block text-center [clip-path:inset(0)]",
        class
    );
    let label = digit;

    rsx! {
        span {
            class,
            style: "height:{DIGIT_HEIGHT_EM}em;line-height:{DIGIT_HEIGHT_EM}em;width:1ch",
            span { class: "invisible", "{label}" }
            span {
                class: "absolute inset-x-0 top-0 flex flex-col items-center will-change-[transform,filter]",
                style,
                for n in 0..10u8 {
                    span { key: "{n}", class: "flex h-[1.1em] items-center justify-center leading-none", "{n}" }
                }
            }
        }
    }
}
