//! AnimatedBadge, on screen — `animated-badge.preview.tsx`.
//!
//! A badge that cycles through four statuses every 1.6 s (so the roll can be
//! watched), then the six statuses as small static badges.

use crate::pages::common::*;
use beui::prelude::*;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

const STATES: [(AnimatedBadgeStatus, &str); 4] = [
    (AnimatedBadgeStatus::Loading, "Syncing"),
    (AnimatedBadgeStatus::Success, "Synced"),
    (AnimatedBadgeStatus::Warning, "Review"),
    (AnimatedBadgeStatus::Danger, "Failed"),
];

const ALL: [(AnimatedBadgeStatus, &str); 6] = [
    (AnimatedBadgeStatus::Neutral, "Queued"),
    (AnimatedBadgeStatus::Info, "Live"),
    (AnimatedBadgeStatus::Loading, "Indexing"),
    (AnimatedBadgeStatus::Success, "Verified"),
    (AnimatedBadgeStatus::Warning, "Pending"),
    (AnimatedBadgeStatus::Danger, "Blocked"),
];

#[component]
pub fn AnimatedBadgePage() -> Element {
    let mut active = use_signal(|| 0usize);
    use_future(move || async move {
        loop {
            let _ = Time::delay(Duration::from_millis(1600)).await;
            let next = (active() + 1) % STATES.len();
            active.set(next);
        }
    });

    let (status, label) = STATES[active().min(STATES.len() - 1)];

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "AnimatedBadge",
                lede: "A status pill whose icon and label roll when the status changes — the leaving content slides up and blurs out while the next slides in from below."
            }

            Section {
                title: "Rolling",
                note: "Cycles every 1.6 s through loading, success, warning and danger.",
                div { class: "flex h-16 items-center justify-center rounded-xl border border-border bg-card",
                    AnimatedBadge { status, size: AnimatedBadgeSize::Md, aria_live: "polite", "{label}" }
                }
            }

            Section {
                title: "Statuses",
                note: "All six at sm, each with its own glyph.",
                Stage {
                    for (status, label) in ALL {
                        AnimatedBadge { key: "{label}", status, size: AnimatedBadgeSize::Sm, "{label}" }
                    }
                }
            }
        }
    }
}
