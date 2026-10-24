//! `motion/prediction-market-card.tsx` — a listing surface with outcome CTAs
//! and an independent bookmark toggle.
//!
//! Everything is the source's: the `2 + i/(n-1)*44, 18 - ratio*14` sparkline
//! inside a 48×22 viewBox with its gradient area, the per-outcome probability
//! line scaling from `origin-left` over 250ms `EASE_OUT`, the `1/probability`
//! payout readout, the placeholder-to-action label roll on the odds button, and
//! the bookmark's `[1, 1.2, 1]` pop.
//!
//! One substitution: the source wraps the payout readout in `Tooltip` (another
//! lane's component), so the note is a native `title` plus the same
//! `aria-label` here.

use crate::action_swap::{ActionSwapAnimation, ActionSwapText};
use crate::icons::Bookmark;
use crate::prelude::*;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle, Spring};

const BOOKMARK_POP: &str = "@keyframes beui-bookmark-pop{0%{transform:scale(1)}50%{transform:scale(1.2)}100%{transform:scale(1)}}";
const PAYOUT_NOTE: &str = "Potential payout per $1 if this outcome wins, including your stake. Before fees; based on the displayed price.";

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

fn press() -> Spring {
    SPRING_PRESS
}

fn probability(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Which side of an outcome the reader picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketSide {
    Yes,
    No,
}

/// One outcome row.
#[derive(Clone, PartialEq)]
pub struct PredictionMarketCardOutcome {
    pub id: String,
    pub label: String,
    /// Probability between 0 and 1.
    pub probability: f64,
    pub icon: Option<Element>,
    /// Optional team color for the compact probability line.
    pub color: Option<String>,
}

impl PredictionMarketCardOutcome {
    /// An outcome with a label and a probability.
    pub fn new(id: impl Into<String>, label: impl Into<String>, probability: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            probability,
            icon: None,
            color: None,
        }
    }

    /// Attach a leading icon.
    pub fn icon(mut self, icon: Element) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Override the probability line's color.
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// What a click on an outcome reports.
#[derive(Clone, PartialEq)]
pub struct PredictionMarketSelection {
    pub outcome_id: String,
    pub side: MarketSide,
}

/// Props for [`PredictionMarketCard`].
#[derive(Props, Clone, PartialEq)]
pub struct PredictionMarketCardProps {
    pub title: String,
    #[props(default)]
    pub icon: Option<Element>,
    #[props(default)]
    pub category: Option<String>,
    #[props(default)]
    pub volume: Option<String>,
    /// Sparkline samples.
    #[props(default)]
    pub volume_history: Option<Vec<f64>>,
    #[props(default)]
    pub status: Option<String>,
    #[props(default)]
    pub live: bool,
    pub outcomes: Vec<PredictionMarketCardOutcome>,
    #[props(default)]
    pub on_outcome_click: Option<EventHandler<PredictionMarketSelection>>,
    #[props(default)]
    pub bookmarked: Option<bool>,
    #[props(default)]
    pub default_bookmarked: bool,
    #[props(default)]
    pub on_bookmark_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub class: Option<String>,
}

/// A prediction-market listing.
#[component]
pub fn PredictionMarketCard(props: PredictionMarketCardProps) -> Element {
    let PredictionMarketCardProps {
        title,
        icon,
        category,
        volume,
        volume_history,
        status,
        live,
        outcomes,
        on_outcome_click,
        bookmarked,
        default_bookmarked,
        on_bookmark_change,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let chart_id = use_hook(beui_core::platform::next_scope_id);

    let samples: Vec<f64> = volume_history
        .unwrap_or_default()
        .into_iter()
        .filter(|sample| sample.is_finite())
        .collect();
    let low = samples.iter().cloned().fold(f64::INFINITY, f64::min);
    let high = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = if samples.is_empty() { 0.0 } else { high - low };
    let chart_points = if samples.len() > 1 {
        let last = (samples.len() - 1) as f64;
        let points: Vec<String> = samples
            .iter()
            .enumerate()
            .map(|(index, sample)| {
                let x = 2.0 + (index as f64 / last) * 44.0;
                let ratio = if range != 0.0 {
                    (sample - low) / range
                } else {
                    0.5
                };
                let y = 18.0 - ratio * 14.0;
                format!("{x},{y}")
            })
            .collect();
        Some(points.join(" "))
    } else {
        None
    };

    let mut internal_bookmark = use_signal(|| default_bookmarked);
    let saved = bookmarked.unwrap_or(internal_bookmark());

    let mut bookmark_tap = use_motion(1.0f32).expect("1.0 is finite");

    let class = cn!(
        "flex h-full w-full min-w-0 flex-col overflow-hidden rounded-3xl bg-card text-foreground",
        class,
    );
    let status_live = cn!(
        "inline-flex items-center gap-1.5",
        if live { "text-rose-500" } else { "" }
    );

    rsx! {
        style { dangerous_inner_html: BOOKMARK_POP }
        article { aria_labelledby: "market-title-{chart_id}", class,
            header { class: "flex shrink-0 items-center gap-3 px-4 py-3",
                if let Some(icon) = icon {
                    div {
                        aria_hidden: "true",
                        class: "flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-full border border-border bg-background text-foreground",
                        {icon}
                    }
                }
                div { class: "min-w-0 flex-1",
                    h3 {
                        id: "market-title-{chart_id}",
                        class: "break-words font-display text-base font-medium leading-snug tracking-tight",
                        "{title}"
                    }
                    p { class: "mt-1 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground",
                        if let Some(status) = status.clone() {
                            span { class: status_live,
                                if live {
                                    span { aria_hidden: "true", class: "size-1.5 rounded-full bg-current" }
                                }
                                "{status}"
                            }
                        }
                        if status.is_some() && category.is_some() {
                            span { aria_hidden: "true", "·" }
                        }
                        if let Some(category) = category.clone() {
                            span { "{category}" }
                        }
                    }
                }
            }
            div { class: "mx-2 mb-2 flex flex-1 flex-col rounded-3xl bg-background px-4 py-3",
                div { class: "flex flex-1 flex-col justify-center gap-3",
                    for (index, outcome) in outcomes.iter().enumerate() {
                        div { key: "{outcome.id}", class: "space-y-1",
                            div { class: "flex min-h-10 items-center gap-2",
                                if let Some(outcome_icon) = outcome.icon.clone() {
                                    span {
                                        aria_hidden: "true",
                                        class: "flex size-8 shrink-0 items-center justify-center overflow-hidden rounded-full bg-muted",
                                        {outcome_icon}
                                    }
                                }
                                span { class: "min-w-0 flex-1 break-words text-sm font-medium", "{outcome.label}" }
                                // `Tooltip` in the source; a native title here.
                                button {
                                    r#type: "button",
                                    title: PAYOUT_NOTE,
                                    aria_label: "Potential payout for {outcome.label}",
                                    class: "shrink-0 rounded-md py-2 text-sm tabular-nums text-muted-foreground focus-visible:outline-2 focus-visible:outline-ring",
                                    if probability(outcome.probability) > 0.0 {
                                        "{(1.0 / probability(outcome.probability)):.1}×"
                                    } else {
                                        "—"
                                    }
                                }
                                MarketOddsButton {
                                    positive: index % 2 == 0,
                                    outcome: outcome.clone(),
                                    onclick: {
                                        let id = outcome.id.clone();
                                        move |_| {
                                            if let Some(handler) = on_outcome_click {
                                                handler.call(PredictionMarketSelection {
                                                    outcome_id: id.clone(),
                                                    side: MarketSide::Yes,
                                                });
                                            }
                                        }
                                    },
                                }
                            }
                            div { aria_hidden: "true", class: "h-0.5 w-24 overflow-hidden rounded-full",
                                ProbabilityLine {
                                    probability: probability(outcome.probability),
                                    color: outcome.color.clone(),
                                    reduce: reduce_value,
                                }
                            }
                        }
                    }
                }
                footer { class: "mt-2 flex shrink-0 items-center gap-2 text-xs text-muted-foreground",
                    if let Some(points) = chart_points.clone() {
                        svg {
                            "aria-hidden": "true",
                            view_box: "0 0 48 22",
                            class: "h-5 w-12 shrink-0 text-emerald-500 dark:text-emerald-400",
                            fill: "none",
                            defs {
                                linearGradient { id: "market-chart-{chart_id}", x1: "0", y1: "0", x2: "0", y2: "1",
                                    stop { offset: "0%", stop_color: "currentColor", stop_opacity: "0.22" }
                                    stop { offset: "100%", stop_color: "currentColor", stop_opacity: "0" }
                                }
                            }
                            polygon { points: "2,22 {points} 46,22", fill: "url(#market-chart-{chart_id})" }
                            polyline {
                                points,
                                stroke: "currentColor",
                                stroke_width: "1.75",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                            }
                        }
                    }
                    if let Some(volume) = volume.clone() {
                        span { class: "shrink-0", "{volume} vol." }
                    }
                    button {
                        r#type: "button",
                        aria_label: "Bookmark {title}",
                        aria_pressed: if saved { "true" } else { "false" },
                        class: cn!(
                            "ml-auto flex size-9 shrink-0 items-center justify-center rounded-xl transition-colors hover:bg-muted focus-visible:outline-2 focus-visible:outline-ring",
                            if saved { "text-foreground" } else { "" },
                        ),
                        onclick: move |_| {
                            if bookmarked.is_none() {
                                internal_bookmark.set(!saved);
                            }
                            if let Some(handler) = on_bookmark_change {
                                handler.call(!saved);
                            }
                        },
                        style: if reduce_value { String::new() } else { format!("transform:scale({})", current(bookmark_tap)) },
                        onpointerdown: move |_| {
                            if !reduce_value {
                                let _ = bookmark_tap.animate_to(0.85, AnimationConfig::spring(press()));
                            }
                        },
                        onpointerup: move |_| {
                            let _ = bookmark_tap.animate_to(1.0, AnimationConfig::spring(press()));
                        },
                        onpointercancel: move |_| {
                            let _ = bookmark_tap.animate_to(1.0, AnimationConfig::spring(press()));
                        },
                        // `[1, 1.2, 1]` over 220ms EASE_OUT. A CSS keyframe keyed by
                        // the flag restarts it on every toggle; the motion manager
                        // has no "jump to value" to seed a three-step spring with.
                        span {
                            key: "{saved}",
                            class: "inline-flex",
                            style: if reduce_value { String::new() } else { "animation:beui-bookmark-pop 220ms ease-out".to_string() },
                            Bookmark { class: Some(cn!("size-4", if saved { "fill-current" } else { "" })) }
                        }
                    }
                }
            }
        }
    }
}

/// The probability line under an outcome row.
#[component]
fn ProbabilityLine(probability: f64, color: Option<String>, reduce: bool) -> Element {
    let mut scale = use_motion(probability as f32).expect("probability is finite");
    use_effect(use_reactive((&probability,), move |(probability,)| {
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            beui_core::ease::tween_ease_out(250)
        };
        let _ = scale.animate_to(probability as f32, config);
    }));
    let scale_value = f64::from(current(scale));
    let class = "h-full origin-left rounded-full bg-emerald-300/70 dark:bg-emerald-400/40";
    let style = match color {
        Some(color) => format!("transform:scaleX({scale_value});background-color:{color}"),
        None => format!("transform:scaleX({scale_value})"),
    };

    rsx! {
        div { class, style }
    }
}

/// The odds button: the placeholder rolls to the side label on hover/focus.
#[component]
fn MarketOddsButton(
    positive: bool,
    outcome: PredictionMarketCardOutcome,
    onclick: EventHandler<()>,
) -> Element {
    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let can_hover = use_hover_capable();
    let mut hovered = use_signal(|| false);
    let mut focused = use_signal(|| false);
    let mut tap = use_motion(1.0f32).expect("1.0 is finite");
    let cents = (probability(outcome.probability) * 100.0).round() as i64;
    let show_action = (can_hover() && hovered()) || focused();

    let class = cn!(
        "relative flex min-h-10 min-w-16 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-border bg-background px-3 text-sm font-semibold text-foreground shadow-[0_3px_0] transition-colors hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
        if positive {
            "shadow-emerald-500/20 border-emerald-500/25 bg-emerald-500/10 text-emerald-700 hover:bg-emerald-500/15 dark:text-emerald-400"
        } else {
            "shadow-rose-500/20 border-rose-500/25 bg-rose-500/10 text-rose-700 hover:bg-rose-500/15 dark:text-rose-400"
        },
    );

    rsx! {
        button {
            r#type: "button",
            aria_label: "Trade {outcome.label} at {cents}%",
            class,
            style: if reduce_value { String::new() } else { format!("transform:scale({})", current(tap)) },
            onclick: move |_| onclick.call(()),
            onpointerenter: move |event: Event<PointerData>| {
                if event.pointer_type() != "touch" {
                    hovered.set(true);
                }
            },
            onpointerleave: move |_| hovered.set(false),
            onpointerdown: move |_| {
                if !reduce_value {
                    let _ = tap.animate_to(0.96, AnimationConfig::spring(press()));
                }
            },
            onpointerup: move |_| {
                let _ = tap.animate_to(1.0, AnimationConfig::spring(press()));
            },
            onfocus: move |_| focused.set(true),
            onblur: move |_| focused.set(false),
            ActionSwapText {
                value: if show_action { "action".to_string() } else { cents.to_string() },
                label: if show_action {
                    if positive { "Yes".to_string() } else { "No".to_string() }
                } else {
                    format!("{cents}%")
                },
                animation: ActionSwapAnimation::Roll,
            }
        }
    }
}
