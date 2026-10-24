//! `agents/loading-states/` — the three "agent is working" surfaces.
//!
//! `ThinkingShimmer` and `AgentProgress` keep the source's markup and timing:
//! the shimmer sweep is the same `@keyframes beui-text-shimmer` (from
//! `lib/text-shimmer.ts`), and the progress grid runs the same 1.55s
//! ease-in-out pulse with the same per-cell delays, driven from a page timer
//! (see [`crate::timers`]).
//!
//! `ReasoningText` keeps the phrase cycle and the shimmer; the per-character
//! cascade and the swap are CSS enter animations instead of motion variants, so
//! a phrase that cycles back re-enters rather than cross-fading mid-exit. The
//! `scramble` variant falls back to `swap` until motion's `TextScramble` lands
//! (deliberate).

use beui::prelude::*;

use crate::standins::{shimmer_style, AsciiLoader, SHIMMER_CLASS};
use crate::timers::{use_elapsed_seconds, use_interval_index};

const AGENT_CELL_KEYFRAMES: &str = "@keyframes beui-agent-cell{0%,100%{opacity:0.28;transform:scale(0.72)}50%{opacity:1;transform:scale(1)}}";
const CASCADE_KEYFRAMES: &str = "@keyframes beui-cascade-in{from{opacity:0;transform:translateY(100%)}to{opacity:1;transform:translateY(0)}}@keyframes beui-swap-in{from{opacity:0;transform:translateY(3px)}to{opacity:1;transform:translateY(0)}}";

/// `ThinkingShimmer`.
#[component]
pub fn ThinkingShimmer(
    #[props(default)] children: Option<Element>,
    #[props(default = 1.8)] duration: f64,
    #[props(default)] class: String,
) -> Element {
    let label = children.unwrap_or_else(|| rsx! { "Thinking…" });
    let style = shimmer_style(duration);
    rsx! {
        style { dangerous_inner_html: "{crate::standins::SHIMMER_KEYFRAMES}" }
        span {
            class: cn!(SHIMMER_CLASS, "font-medium", class),
            style: "{style}",
            {label}
        }
    }
}

/// `AgentProgress`.
#[component]
pub fn AgentProgress(
    #[props(default = "Churning".to_string())] label: String,
    #[props(default)] elapsed_seconds: Option<f64>,
    #[props(default)] initial_seconds: f64,
    #[props(default = true)] running: bool,
    #[props(default)] class: String,
) -> Element {
    let internal = use_elapsed_seconds(initial_seconds, running && elapsed_seconds.is_none());
    let elapsed = elapsed_seconds.unwrap_or(internal());

    rsx! {
        style { dangerous_inner_html: "{AGENT_CELL_KEYFRAMES}" }
        span {
            role: "status",
            "aria-label": "{label}, in progress",
            class: cn!("inline-flex items-center gap-3 font-mono text-sm text-muted-foreground", class),
            span { "aria-hidden": "true", class: "grid size-5 shrink-0 grid-cols-3 gap-[2px]",
                for index in 0..9 {
                    span {
                        key: "{index}",
                        class: "rounded-[1px] bg-current",
                        style: "animation:beui-agent-cell 1.55s cubic-bezier(0.65,0,0.35,1) infinite;animation-delay:{index as f64 * 0.14}s;",
                    }
                }
            }
            span { class: "font-sans font-medium", "{label}" }
            span { "aria-hidden": "true", class: "tabular-nums text-muted-foreground/70",
                "{format_elapsed(elapsed)}"
            }
        }
    }
}

fn format_elapsed(total_seconds: f64) -> String {
    let safe = total_seconds.max(0.0);
    let minutes = (safe / 60.0).floor();
    let seconds = safe % 60.0;
    if minutes > 0.0 {
        format!("{}m {:.1}s", minutes, seconds)
    } else {
        format!("{seconds:.1}s")
    }
}

/// `ReasoningTextVariant`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ReasoningTextVariant {
    #[default]
    Cascade,
    Swap,
    Scramble,
}

const DEFAULT_PHRASES: [&str; 4] = [
    "Thinking",
    "Reading the context",
    "Connecting the details",
    "Forming a response",
];

/// `ReasoningText`.
#[component]
pub fn ReasoningText(
    #[props(default)] phrases: Vec<String>,
    #[props(default)] variant: ReasoningTextVariant,
    #[props(default = 1800)] interval: u32,
    #[props(default = 2.2)] shimmer_duration: f64,
    #[props(default)] indicator: Option<Element>,
    #[props(default)] class: String,
) -> Element {
    let owned: Vec<String> = if phrases.is_empty() {
        DEFAULT_PHRASES
            .iter()
            .map(|phrase| phrase.to_string())
            .collect()
    } else {
        phrases
    };
    let longest = owned
        .iter()
        .max_by_key(|phrase| phrase.len())
        .cloned()
        .unwrap_or_default();
    let index = use_interval_index(owned.len(), interval.max(600));
    let phrase = owned[index() % owned.len()].clone();
    let status_id = crate::common::use_unique_id("reasoning");
    let shimmer = shimmer_style(shimmer_duration);
    // `scramble` needs motion's `TextScramble`; it renders as `swap` for now.
    let effective = if variant == ReasoningTextVariant::Scramble {
        ReasoningTextVariant::Swap
    } else {
        variant
    };

    let active = match effective {
        ReasoningTextVariant::Cascade => {
            let chars: Vec<(usize, char)> = format!("{phrase}…").chars().enumerate().collect();
            rsx! {
                span {
                    key: "{phrase}",
                    class: "col-start-1 row-start-1 inline-block justify-self-start whitespace-pre",
                    for (char_index, character) in chars {
                        {
                            let delay = char_index as f64 * 0.025;
                            rsx! {
                                span {
                                    key: "{char_index}",
                                    class: cn!("inline-block whitespace-pre will-change-[opacity,transform]", SHIMMER_CLASS),
                                    style: "animation:beui-cascade-in 300ms cubic-bezier(0.34,1.56,0.64,1) both {delay}s,{shimmer}",
                                    "{character}"
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => rsx! {
            span {
                key: "{phrase}",
                class: cn!("col-start-1 row-start-1 inline-block justify-self-start whitespace-nowrap will-change-[opacity,transform]", SHIMMER_CLASS),
                style: "animation:beui-swap-in 200ms cubic-bezier(0.16,1,0.3,1) both,{shimmer}",
                "{phrase}…"
            }
        },
    };

    let leading = indicator.unwrap_or_else(|| rsx! { AsciiLoader {} });

    rsx! {
        style { dangerous_inner_html: "{CASCADE_KEYFRAMES}" }
        span {
            role: "status",
            "aria-live": "polite",
            "aria-labelledby": "{status_id}",
            class: cn!("inline-flex items-center gap-2 text-sm font-medium text-muted-foreground", class),
            span { "aria-hidden": "true", class: "inline-flex size-3 shrink-0 items-center justify-center", {leading} }
            span { "aria-hidden": "true", class: "grid overflow-hidden text-left",
                span { class: "invisible col-start-1 row-start-1 whitespace-nowrap", "{longest}…" }
                {active}
            }
            span { id: "{status_id}", class: "sr-only", "{phrase}" }
        }
    }
}
