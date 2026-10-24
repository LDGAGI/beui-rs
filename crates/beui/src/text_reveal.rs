//! `motion/text-reveal.tsx` — words or characters arriving one at a time.
//!
//! One tokenizer for both modes: a line becomes the words it is made of, each
//! carrying the whitespace that follows it. Word mode animates a group at a
//! time, char mode the characters inside one, so the two cannot drift apart on
//! what counts as a word or where a space belongs. Runs of whitespace and tabs
//! survive as their own group rather than collapsing.
//!
//! Each unit is its own component with its own springs — `y` on a spring,
//! `opacity` and `filter` on `EASE_OUT` tweens of 0.7s and 0.9s, exactly the
//! three transition entries the source gives a unit.

use crate::prelude::*;
use crate::scroll::{use_in_view, InViewAmount, InViewOptions};
use crate::text::TextTag;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle, Spring};

/// `DEFAULT_SPRING` in the source.
pub const TEXT_REVEAL_SPRING: Spring = Spring {
    stiffness: 140.0,
    damping: 26.0,
    mass: 1.2,
};

/// `split` — a group at a time, or a character at a time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitMode {
    #[default]
    Word,
    Char,
}

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// `toWordGroups(line)`: `(\S+\s*|\s+)` chunks, split into text and trailing.
fn to_word_groups(line: &str) -> Vec<(String, String)> {
    let characters: Vec<char> = line.chars().collect();
    let mut groups = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let start = index;
        if characters[index].is_whitespace() {
            while index < characters.len() && characters[index].is_whitespace() {
                index += 1;
            }
            groups.push((String::new(), characters[start..index].iter().collect()));
        } else {
            while index < characters.len() && !characters[index].is_whitespace() {
                index += 1;
            }
            let text: String = characters[start..index].iter().collect();
            let whitespace_start = index;
            while index < characters.len() && characters[index].is_whitespace() {
                index += 1;
            }
            let trailing: String = characters[whitespace_start..index].iter().collect();
            groups.push((text, trailing));
        }
    }
    groups
}

/// Parse `"40%"` / `"20"` into a value and whether it is a percentage.
fn parse_offset(offset: &str) -> (f64, bool) {
    let trimmed = offset.trim();
    if let Some(percent) = trimmed.strip_suffix('%') {
        (percent.trim().parse().unwrap_or(0.0), true)
    } else {
        (trimmed.parse().unwrap_or(0.0), false)
    }
}

/// One animating unit — a word (plus its trailing space) or a single character.
#[component]
fn RevealUnit(
    text: String,
    delay: f64,
    reduce: bool,
    animate: bool,
    y_offset: f64,
    percent: bool,
    blur: f64,
    spring: Spring,
) -> Element {
    let mut y = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let mut opacity = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let mut filter = use_motion(0.0f32).expect("0.0 is a finite animation value");

    use_effect(use_reactive(
        (&animate, &reduce),
        move |(animate, reduce)| {
            let target = if animate { 1.0 } else { 0.0 };
            if reduce {
                let config = beui_core::ease::tween_ease_out(250).with_delay(
                    dioxus_motion::Duration::from_millis((delay * 0.3 * 1000.0).round() as u64),
                );
                let _ = opacity.animate_to(target, config);
                return;
            }
            let delay = dioxus_motion::Duration::from_millis((delay * 1000.0).round() as u64);
            let _ = y.animate_to(target, AnimationConfig::spring(spring).with_delay(delay));
            let _ = opacity.animate_to(
                target,
                beui_core::ease::tween_ease_out(700).with_delay(delay),
            );
            let _ = filter.animate_to(
                target,
                beui_core::ease::tween_ease_out(900).with_delay(delay),
            );
        },
    ));

    let yp = current(y);
    let op = current(opacity);
    let fp = current(filter);
    // `transform` and `filter` are always declared: the web interpreter restores
    // inline properties a later `style` omits, so an omitted transform would
    // freeze the last pin. (`none` is equivalent to "no transform".)
    let style = if reduce {
        format!("opacity:{op};transform:none;filter:none")
    } else {
        let unit = if percent { "%" } else { "px" };
        format!(
            "opacity:{op};transform:translateY({}{unit});filter:blur({}px)",
            y_offset * f64::from(1.0 - yp),
            blur * f64::from(1.0 - fp)
        )
    };

    rsx! {
        span {
            class: "inline-block whitespace-pre will-change-transform",
            style,
            "{text}"
        }
    }
}

/// Props for [`TextReveal`].
#[derive(Props, Clone, PartialEq)]
pub struct TextRevealProps {
    /// One entry per line. A single string is a one-line vec.
    pub text: Vec<String>,
    #[props(default)]
    pub r#as: TextTag,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub split: SplitMode,
    /// Seconds between neighbouring units. Defaults to `0.09`.
    #[props(default = 0.09)]
    pub stagger: f64,
    #[props(default)]
    pub delay: f64,
    /// Enter blur in px. Defaults to `12`.
    #[props(default = 12.0)]
    pub blur: f64,
    /// Slide distance, `"40%"` or `"20"`. Defaults to `"40%"`.
    #[props(default = "40%".to_string())]
    pub y_offset: String,
    /// Override the spring. Defaults to `{140, 26, 1.2}`.
    #[props(default)]
    pub spring: Option<Spring>,
    /// Reveal once (default) or every time the element enters view.
    #[props(default = true)]
    pub once: bool,
    /// Wait for the element to be in view. Defaults to `false`.
    #[props(default)]
    pub while_in_view: bool,
    #[props(default)]
    pub children: Option<Element>,
}

/// Reveal its text word by word (or character by character).
#[component]
pub fn TextReveal(props: TextRevealProps) -> Element {
    let TextRevealProps {
        text,
        r#as,
        class,
        split,
        stagger,
        delay,
        blur,
        y_offset,
        spring,
        once,
        while_in_view,
        children,
    } = props;

    let reduce = use_reduced_motion();
    let view = use_in_view(InViewOptions {
        once,
        amount: InViewAmount::Some,
        root: None,
    });
    let should_animate = if while_in_view { view.value() } else { true };
    let (offset_value, percent) = parse_offset(&y_offset);
    let spring = spring.unwrap_or(TEXT_REVEAL_SPRING);

    // Precompute the unit tree so the running unit index (which sets the
    // stagger) is a plain Rust loop rather than mutable state inside `rsx!`.
    let mut unit_index = 0usize;
    let mut lines: Vec<Vec<Vec<(String, f64)>>> = Vec::with_capacity(text.len());
    for line in &text {
        let mut groups: Vec<Vec<(String, f64)>> = Vec::new();
        for (word, trailing) in to_word_groups(line) {
            let whole = format!("{word}{trailing}");
            let mut units = Vec::new();
            if split == SplitMode::Char {
                for character in whole.chars() {
                    let unit_delay = delay + unit_index as f64 * stagger;
                    unit_index += 1;
                    units.push((character.to_string(), unit_delay));
                }
            } else {
                let unit_delay = delay + unit_index as f64 * stagger;
                unit_index += 1;
                units.push((whole, unit_delay));
            }
            groups.push(units);
        }
        lines.push(groups);
    }

    let class = cn!("block", class);
    let animate = should_animate;
    let reduce_now = reduce();

    let body = rsx! {
        for (line_index, groups) in lines.iter().enumerate() {
            span { key: "line-{line_index}", class: "block",
                for (group_index, units) in groups.iter().enumerate() {
                    if split == SplitMode::Char {
                        span {
                            key: "group-{line_index}-{group_index}",
                            class: "inline-block whitespace-pre",
                            for (unit_index, (unit_text, unit_delay)) in units.iter().enumerate() {
                                RevealUnit {
                                    key: "unit-{line_index}-{group_index}-{unit_index}",
                                    text: unit_text.clone(),
                                    delay: *unit_delay,
                                    reduce: reduce_now,
                                    animate,
                                    y_offset: offset_value,
                                    percent,
                                    blur,
                                    spring,
                                }
                            }
                        }
                    } else if let Some((unit_text, unit_delay)) = units.first() {
                        RevealUnit {
                            key: "unit-{line_index}-{group_index}",
                            text: unit_text.clone(),
                            delay: *unit_delay,
                            reduce: reduce_now,
                            animate,
                            y_offset: offset_value,
                            percent,
                            blur,
                            spring,
                        }
                    }
                }
            }
        }
        {children}
    };

    match r#as {
        TextTag::Span => rsx! { span { class, "data-beui-inview": view.attr(), {body} } },
        TextTag::P => rsx! { p { class, "data-beui-inview": view.attr(), {body} } },
        TextTag::Div => rsx! { div { class, "data-beui-inview": view.attr(), {body} } },
        TextTag::H1 => rsx! { h1 { class, "data-beui-inview": view.attr(), {body} } },
        TextTag::H2 => rsx! { h2 { class, "data-beui-inview": view.attr(), {body} } },
        TextTag::H3 => rsx! { h3 { class, "data-beui-inview": view.attr(), {body} } },
        TextTag::H4 => rsx! { h4 { class, "data-beui-inview": view.attr(), {body} } },
        TextTag::H5 => rsx! { h5 { class, "data-beui-inview": view.attr(), {body} } },
        TextTag::H6 => rsx! { h6 { class, "data-beui-inview": view.attr(), {body} } },
    }
}
