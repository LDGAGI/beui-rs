//! `KnockoutBracket` — the beUI preview draws a World Cup style draw. The
//! source's teams carry flagcdn country codes; this uses the initials stand-in
//! so the page stays offline.

use beui::prelude::*;
use beui::{BracketRound, BracketSide, KnockoutBracket, Match, MatchSide, MatchStatus, Team};

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

fn upcoming(id: &str, home: &str, away: &str) -> Match {
    Match::new(
        id,
        MatchSide::team(Team::new(home)),
        MatchSide::team(Team::new(away)),
    )
    .date("Tue, 7 Jul")
    .time("20:00")
    .status(MatchStatus::Upcoming)
}

fn rounds() -> Vec<BracketRound> {
    vec![
        BracketRound::new(
            "Round of 8",
            vec![
                played("qf-1", "Netherlands", 1, "Morocco", 1, BracketSide::Away).badge("FT (P)"),
                played("qf-2", "France", 3, "Sweden", 0, BracketSide::Home),
                played("qf-3", "Belgium", 3, "Senegal", 2, BracketSide::Home),
                played("qf-4", "Spain", 2, "Austria", 0, BracketSide::Home),
            ],
        ),
        BracketRound::new(
            "Semi-finals",
            vec![
                played("sf-1", "Morocco", 1, "France", 2, BracketSide::Away),
                Match::new(
                    "sf-2",
                    MatchSide::team(Team::new("Belgium")).score(1),
                    MatchSide::team(Team::new("Spain")).score(1).penalties(3),
                )
                .date("Sun, 5 Jul")
                .status(MatchStatus::Finished)
                .winner(BracketSide::Away),
            ],
        ),
        BracketRound::new("Final", vec![upcoming("final", "France", "Spain")]),
    ]
}

#[component]
pub fn KnockoutBracketPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-6xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "KnockoutBracket" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A single-elimination draw that is laid out, not scrolled. Every later match sits at the exact midpoint of its two feeders, the columns page together under one reflow spring, and the connectors are CSS pockets rather than SVG paths."
                }
            }
            KnockoutBracket {
                rounds: rounds(),
                initial_round: 0,
                third_place: Some(played("third", "Morocco", 2, "Belgium", 1, BracketSide::Home).badge("AET")),
            }
        }
    }
}
