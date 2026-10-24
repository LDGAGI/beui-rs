//! `KnockoutWheel` — the same draw as the bracket, laid out as a wheel. The
//! source's crests come from flagcdn; this uses the initials stand-in so the
//! page stays offline.

use beui::prelude::*;
use beui::{BracketRound, BracketSide, KnockoutWheel, Match, MatchSide, MatchStatus, Team};

fn played(id: &str, home: &str, hs: i32, away: &str, as_: i32, winner: BracketSide) -> Match {
    Match::new(
        id,
        MatchSide::team(Team::new(home)).score(hs),
        MatchSide::team(Team::new(away)).score(as_),
    )
    .date("Sat, 4 Jul")
    .status(MatchStatus::Finished)
    .winner(winner)
}

fn rounds() -> Vec<BracketRound> {
    vec![
        BracketRound::new(
            "Quarter-finals",
            vec![
                played("qf-1", "Netherlands", 1, "Morocco", 1, BracketSide::Away),
                played("qf-2", "France", 3, "Sweden", 0, BracketSide::Home),
                played("qf-3", "Belgium", 3, "Senegal", 2, BracketSide::Home),
                played("qf-4", "Spain", 2, "Austria", 0, BracketSide::Home),
            ],
        ),
        BracketRound::new(
            "Semi-finals",
            vec![
                played("sf-1", "Morocco", 1, "France", 2, BracketSide::Away),
                played("sf-2", "Belgium", 1, "Spain", 1, BracketSide::Away),
            ],
        ),
        BracketRound::new(
            "Final",
            vec![Match::new(
                "final",
                MatchSide::team(Team::new("France")).score(2),
                MatchSide::team(Team::new("Spain")).score(1),
            )
            .status(MatchStatus::Finished)
            .winner(BracketSide::Home)],
        ),
    ]
}

#[component]
pub fn KnockoutWheelPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "KnockoutWheel" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The bracket's draw laid out as a wheel: the final at the hub, one ring per round, and each parent's wedge split in half at every step out. Hover or focus a flag to light its path; the arrow keys walk the tree."
                }
            }
            KnockoutWheel { rounds: rounds() }
        }
    }
}
