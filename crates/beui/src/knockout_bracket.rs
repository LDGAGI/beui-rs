//! `motion/knockout-bracket.tsx` — a single-elimination draw, laid out rather
//! than scrolled.
//!
//! The geometry is the source's, number for number: `CARD_W 250`, `CARD_H 124`,
//! `GAP_X 40`, `GAP_Y 20`, `VISIBLE_COLS 3`, the `20 + 20` connector pocket and
//! stem, `HEADER_H 44`, the `PAD_X 8` / `PAD_Y 12` breathing room, and the
//! `{260, 32, 0.9}` reflow spring with its `0.28s EASE_OUT` opacity. So are the
//! three layout passes: the base column at `PAD_Y + i * ROW + CARD_H / 2`, each
//! later match at the midpoint of its two feeders (with the fallback stack for a
//! round whose feeders run out), behind rounds keeping their natural spread at
//! `ROW / 2^(page - r + 1)` and alternating around their parent, the connector
//! list, and the measured `lowest` that sizes the stage.
//!
//! Also the source's: the `FT` / `FT (P)` badge with a per-match override, the
//! dimming of the losing side once a match is decided, the fixed 6px winner
//! marker slot, `score (penalties)`, the `initials` two-word stand-in, the
//! flagcdn crest for a country `code` (with the failed-URL fallback so a
//! corrected logo is retried), the header's screen-reader page summary, the
//! `maxPage = rounds.length - min(2, rounds.length)` pagination, and the third
//! place play-off below the stage.
//!
//! One difference: the connectors' height animates on its own spring here like
//! the source's, but the crest images are only fetched when a `logo` is given —
//! the gallery draw uses initials instead of flagcdn, so the page stays offline.

use crate::icons::{ChevronLeft, ChevronRight, Shield};
use crate::prelude::*;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle, Spring};

const CARD_W: f64 = 250.0;
const CARD_H: f64 = 124.0;
const GAP_X: f64 = 40.0;
const GAP_Y: f64 = 20.0;
const COL_W: f64 = CARD_W + GAP_X;
const ROW: f64 = CARD_H + GAP_Y;
const VISIBLE_COLS: usize = 3;
const CONNECTOR_POCKET: f64 = 20.0;
const CONNECTOR_STEM: f64 = GAP_X - CONNECTOR_POCKET;
const HEADER_H: f64 = 44.0;
const PAD_X: f64 = 8.0;
const PAD_Y: f64 = 12.0;

/// Firmer than `SPRING_LAYOUT` so the cards, connectors and stage height glide
/// as one piece; just over critical, so it settles without bounce.
fn reflow_spring() -> Spring {
    Spring {
        stiffness: 260.0,
        damping: 32.0,
        mass: 0.9,
    }
}

fn reflow_config(reduce: bool) -> AnimationConfig {
    if reduce {
        beui_core::ease::tween_ease_out(0)
    } else {
        AnimationConfig::spring(reflow_spring())
    }
}

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// A competitor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Team {
    pub name: String,
    /// Any square image URL — club crest, org mark, player photo.
    pub logo: Option<String>,
    /// ISO 3166-1 alpha-2 code, loaded from flagcdn.com when `logo` is absent.
    pub code: Option<String>,
}

impl Team {
    /// A team by name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            logo: None,
            code: None,
        }
    }

    /// A team whose crest is a URL.
    pub fn logo(mut self, logo: impl Into<String>) -> Self {
        self.logo = Some(logo.into());
        self
    }

    /// A team whose flag comes from flagcdn.com.
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// One side of a match.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MatchSide {
    /// `None` renders a TBD slot with a shield icon.
    pub team: Option<Team>,
    pub score: Option<i32>,
    /// Shootout score, drawn Google-style as `1 (3)`.
    pub penalties: Option<i32>,
}

impl MatchSide {
    /// A side without a team: a TBD slot.
    pub fn tbd() -> Self {
        Self::default()
    }

    /// A side with a team and no score yet.
    pub fn team(team: Team) -> Self {
        Self {
            team: Some(team),
            score: None,
            penalties: None,
        }
    }

    /// Set the score.
    pub fn score(mut self, score: i32) -> Self {
        self.score = Some(score);
        self
    }

    /// Set the shootout score.
    pub fn penalties(mut self, penalties: i32) -> Self {
        self.penalties = Some(penalties);
        self
    }
}

/// Whether a match has been played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchStatus {
    Finished,
    Upcoming,
}

/// Which side of a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Home,
    Away,
}

/// One tie.
#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub id: String,
    pub date: Option<String>,
    pub time: Option<String>,
    pub status: Option<MatchStatus>,
    pub home: MatchSide,
    pub away: MatchSide,
    pub winner: Option<Side>,
    /// Replaces the derived result chip ("FT", "FT (P)").
    pub badge: Option<String>,
}

impl Match {
    /// A match between two sides.
    pub fn new(id: impl Into<String>, home: MatchSide, away: MatchSide) -> Self {
        Self {
            id: id.into(),
            date: None,
            time: None,
            status: None,
            home,
            away,
            winner: None,
            badge: None,
        }
    }

    /// The kick-off day, already formatted.
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }

    /// The kick-off time.
    pub fn time(mut self, time: impl Into<String>) -> Self {
        self.time = Some(time.into());
        self
    }

    /// The played state, when the winner alone is not the story.
    pub fn status(mut self, status: MatchStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Which side went through.
    pub fn winner(mut self, winner: Side) -> Self {
        self.winner = Some(winner);
        self
    }

    /// A result chip of your own, such as "AET" or "BO5".
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

/// One column of the draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Round {
    pub name: String,
    pub matches: Vec<Match>,
}

impl Round {
    /// A round with its matches.
    pub fn new(name: impl Into<String>, matches: Vec<Match>) -> Self {
        Self {
            name: name.into(),
            matches,
        }
    }
}

/// Props for [`KnockoutBracket`].
#[derive(Props, Clone, PartialEq)]
pub struct KnockoutBracketProps {
    /// The whole draw, widest round first.
    pub rounds: Vec<Round>,
    /// Round shown as the leftmost column on mount. Defaults to `1`.
    #[props(default = 1)]
    pub initial_round: usize,
    /// Third place play-off, rendered under the bracket.
    #[props(default)]
    pub third_place: Option<Match>,
    #[props(default = "Third place play-off".to_string())]
    pub third_place_label: String,
    #[props(default)]
    pub class: Option<String>,
}

/// A single-elimination bracket.
#[component]
pub fn KnockoutBracket(props: KnockoutBracketProps) -> Element {
    let KnockoutBracketProps {
        rounds,
        initial_round,
        third_place,
        third_place_label,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let visible_cols = VISIBLE_COLS.min(rounds.len());
    let max_page = rounds.len().saturating_sub(2.min(rounds.len()));
    let mut page = use_signal(|| initial_round.min(max_page));

    // The three layout passes, in one place so the render cannot drift from it.
    let mut centers: Vec<Vec<f64>> = vec![Vec::new(); rounds.len()];
    let current_page = page().min(max_page);
    if let Some(base) = rounds.get(current_page) {
        centers[current_page] = (0..base.matches.len())
            .map(|index| PAD_Y + index as f64 * ROW + CARD_H / 2.0)
            .collect();
    }
    for round in (current_page + 1)..rounds.len() {
        let feeders = centers[round - 1].clone();
        let mut row: Vec<f64> = Vec::with_capacity(rounds[round].matches.len());
        for index in 0..rounds[round].matches.len() {
            let top = feeders.get(2 * index).copied();
            let value = match top {
                Some(top) => {
                    let bottom = feeders.get(2 * index + 1).copied().unwrap_or(top);
                    (top + bottom) / 2.0
                }
                None => match row.get(index.wrapping_sub(1)).copied() {
                    Some(previous) if index > 0 => previous + ROW,
                    _ => PAD_Y + CARD_H / 2.0,
                },
            };
            row.push(value);
        }
        centers[round] = row;
    }
    for round in (0..current_page).rev() {
        let half = ROW / 2f64.powi((current_page - round + 1) as i32);
        centers[round] = (0..rounds[round].matches.len())
            .map(|index| {
                let parent = centers
                    .get(round + 1)
                    .and_then(|row| row.get(index / 2))
                    .copied()
                    .unwrap_or(PAD_Y);
                parent + if index % 2 == 0 { -half } else { half }
            })
            .collect();
    }

    // `(x, y, height, visible)`
    let mut connectors: Vec<(f64, f64, f64, bool)> = Vec::new();
    for round in 1..rounds.len() {
        let feeder_right = col_x(round - 1, current_page) + CARD_W;
        let visible = in_window(round, current_page, visible_cols)
            && in_window(round - 1, current_page, visible_cols);
        for index in 0..rounds[round].matches.len() {
            let top = centers[round - 1]
                .get(2 * index)
                .copied()
                .or_else(|| centers[round].get(index).copied())
                .unwrap_or(PAD_Y);
            let bottom = centers[round - 1]
                .get(2 * index + 1)
                .copied()
                .unwrap_or(top);
            connectors.push((feeder_right, top, (bottom - top).max(0.0), visible));
        }
    }
    let lowest = centers
        .iter()
        .skip(current_page)
        .take(visible_cols)
        .flat_map(|row| row.iter().copied())
        .fold(PAD_Y + CARD_H / 2.0, f64::max);
    let container_height = lowest + CARD_H / 2.0 + PAD_Y;
    let container_width = visible_cols as f64 * CARD_W
        + (visible_cols.saturating_sub(1)) as f64 * GAP_X
        + 2.0 * PAD_X;

    let third_label = third_place
        .as_ref()
        .map(|third| match_label(&third_place_label, third))
        .unwrap_or_default();
    let page_status = {
        let names: Vec<String> = rounds
            .iter()
            .skip(current_page)
            .take(visible_cols)
            .map(|round| round.name.clone())
            .collect();
        match names.len() {
            0 => "Showing rounds".to_string(),
            1 => format!("Showing {}", names[0]),
            2 => format!("Showing {} and {}", names[0], names[1]),
            _ => format!(
                "Showing {}, and {}",
                names[..names.len() - 1].join(", "),
                names[names.len() - 1]
            ),
        }
    };

    let root_class = cn!(
        "w-full max-w-full overflow-x-auto overscroll-x-contain",
        class,
    );

    rsx! {
        div { class: root_class,
            section {
                aria_label: "Tournament bracket",
                class: "relative mx-auto",
                style: "width:{container_width}px",
                div { class: "sr-only", aria_live: "polite", "{page_status}" }

                div { class: "relative", style: "height:{HEADER_H}px",
                    div { class: "absolute inset-0 overflow-hidden",
                        for (index, round) in rounds.iter().enumerate() {
                            BracketHeader {
                                key: "{round.name}",
                                name: round.name.clone(),
                                x: col_x(index, current_page),
                                visible: in_window(index, current_page, visible_cols),
                                reduce: reduce_value,
                            }
                        }
                    }
                    if current_page > 0 {
                        button {
                            r#type: "button",
                            aria_label: "Previous round",
                            style: "left:{PAD_X}px",
                            class: "group absolute top-1/2 z-10 grid size-11 -translate-y-1/2 place-items-center rounded-full outline-none",
                            onclick: move |_| {
                                let next = page().saturating_sub(1).min(max_page);
                                page.set(next);
                            },
                            span { class: "grid size-9 place-items-center rounded-full text-muted-foreground transition-colors group-hover:bg-foreground/10 group-hover:text-foreground group-focus-visible:ring-2 group-focus-visible:ring-ring",
                                ChevronLeft { class: Some("size-5".to_string()) }
                            }
                        }
                    }
                    if current_page < max_page {
                        button {
                            r#type: "button",
                            aria_label: "Next round",
                            style: "right:{PAD_X}px",
                            class: "group absolute top-1/2 z-10 grid size-11 -translate-y-1/2 place-items-center rounded-full outline-none",
                            onclick: move |_| {
                                let next = (page() + 1).min(max_page);
                                page.set(next);
                            },
                            span { class: "grid size-9 place-items-center rounded-full text-muted-foreground transition-colors group-hover:bg-foreground/10 group-hover:text-foreground group-focus-visible:ring-2 group-focus-visible:ring-ring",
                                ChevronRight { class: Some("size-5".to_string()) }
                            }
                        }
                    }
                }

                // The stage height springs with the same token, so the bracket
                // collapses as one piece with its cards.
                BracketStage {
                    height: container_height,
                    width: container_width,
                    reduce: reduce_value,
                    for (index, (x, y, height, visible)) in connectors.iter().copied().enumerate() {
                        BracketConnectorLayer {
                            key: "connector-{index}",
                            x,
                            y,
                            height,
                            visible,
                            reduce: reduce_value,
                        }
                    }
                    for (round_index, round) in rounds.iter().enumerate() {
                        ul {
                            key: "{round.name}",
                            aria_label: "{round.name}",
                            aria_hidden: if in_window(round_index, current_page, visible_cols) {
                                None
                            } else {
                                Some("true")
                            },
                            class: "m-0 list-none p-0",
                            for (match_index, entry) in round.matches.iter().enumerate() {
                                MatchCardLayer {
                                    key: "{entry.id}",
                                    match_: entry.clone(),
                                    x: col_x(round_index, current_page),
                                    y: centers
                                        .get(round_index)
                                        .and_then(|row| row.get(match_index))
                                        .copied()
                                        .unwrap_or(PAD_Y)
                                        - CARD_H / 2.0,
                                    visible: in_window(round_index, current_page, visible_cols),
                                    reduce: reduce_value,
                                    label: match_label(&round.name, entry),
                                }
                            }
                        }
                    }
                }

                if let Some(third) = third_place.clone() {
                    div {
                        class: "mt-8 border-t border-border pt-6",
                        style: "padding-left:{PAD_X}px",
                        ul { aria_label: "{third_place_label}", class: "m-0 list-none p-0",
                            li { aria_label: "{third_label}",
                                p { class: "mb-2 text-sm leading-5 text-muted-foreground/70",
                                    "{third_place_label}"
                                }
                                MatchCard { match_: third.clone() }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Column x-offset.
fn col_x(round: usize, page: usize) -> f64 {
    PAD_X + (round as f64 - page as f64) * COL_W
}

/// Window test.
fn in_window(round: usize, page: usize, visible_cols: usize) -> bool {
    round >= page && round < page + visible_cols
}

/// Two-letter stand-in when a team has no artwork.
fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect::<String>()
        .to_uppercase()
}

/// The crest URL, from the logo or the country code.
fn crest_src(team: &Team) -> Option<String> {
    team.logo.clone().or_else(|| {
        team.code
            .clone()
            .map(|code| format!("https://flagcdn.com/w80/{code}.png"))
    })
}

/// `status` is optional, so a decided match reads as finished without it.
fn is_finished(entry: &Match) -> bool {
    match entry.status {
        Some(status) => status == MatchStatus::Finished,
        None => entry.winner.is_some(),
    }
}

fn side_label(side: &MatchSide) -> String {
    let name = side
        .team
        .as_ref()
        .map(|team| team.name.clone())
        .unwrap_or_else(|| "TBD".to_string());
    match side.score {
        None => name,
        Some(score) => {
            let pen = side
                .penalties
                .map(|penalties| format!(" ({penalties} on penalties)"))
                .unwrap_or_default();
            format!("{name} {score}{pen}")
        }
    }
}

/// The screen-reader label for a card.
fn match_label(round_name: &str, entry: &Match) -> String {
    let finished = is_finished(entry);
    let sides = if finished {
        format!("{}, {}", side_label(&entry.home), side_label(&entry.away))
    } else {
        format!(
            "{} versus {}",
            side_label(&entry.home),
            side_label(&entry.away)
        )
    };
    let schedule: Vec<String> = if finished {
        Vec::new()
    } else {
        [entry.date.clone(), entry.time.clone()]
            .into_iter()
            .flatten()
            .collect()
    };
    let when = if schedule.is_empty() {
        String::new()
    } else {
        format!(", {}", schedule.join(", "))
    };
    let outcome = entry
        .winner
        .and_then(|side| match side {
            Side::Home => entry.home.team.as_ref(),
            Side::Away => entry.away.team.as_ref(),
        })
        .map(|team| format!(", {} won", team.name))
        .unwrap_or_default();
    format!("{round_name}: {sides}{when}{outcome}")
}

/// The stage: a box whose height springs with the reflow.
#[component]
fn BracketStage(height: f64, width: f64, reduce: bool, children: Element) -> Element {
    let mut motion = use_motion(height as f32).expect("height is finite");
    use_effect(use_reactive((&height,), move |(height,)| {
        let _ = motion.animate_to(height as f32, reflow_config(reduce));
    }));

    let style = format!(
        "width:{width}px;height:{}px;transition:height 280ms ease-out",
        current(motion)
    );

    rsx! {
        div {
            class: "relative overflow-hidden",
            style,
            {children}
        }
    }
}

/// A round title that glides with the columns.
#[component]
fn BracketHeader(name: String, x: f64, visible: bool, reduce: bool) -> Element {
    let mut motion = use_motion(x as f32).expect("x is finite");
    use_effect(use_reactive((&x,), move |(x,)| {
        let _ = motion.animate_to(x as f32, reflow_config(reduce));
    }));
    let translate = current(motion);

    let style = format!(
        "width:{CARD_W}px;transform:translateX({translate}px);opacity:{};transition:opacity 280ms ease-out",
        if visible { 1.0 } else { 0.0 }
    );

    rsx! {
        div {
            aria_hidden: if visible { None } else { Some("true") },
            class: "absolute left-0 top-0 flex h-full items-center justify-center text-sm font-bold text-foreground",
            style,
            "{name}"
        }
    }
}

/// The `]` pocket and stem between two rounds.
#[component]
fn BracketConnectorLayer(x: f64, y: f64, height: f64, visible: bool, reduce: bool) -> Element {
    let mut x_motion = use_motion(x as f32).expect("x is finite");
    let mut y_motion = use_motion(y as f32).expect("y is finite");
    let mut h_motion = use_motion(height as f32).expect("height is finite");
    use_effect(use_reactive((&x, &y, &height), move |(x, y, height)| {
        let config = reflow_config(reduce);
        let _ = x_motion.animate_to(x as f32, config.clone());
        let _ = y_motion.animate_to(y as f32, config.clone());
        let _ = h_motion.animate_to(height as f32, config);
    }));
    let (tx, ty, th) = (
        f64::from(current(x_motion)),
        f64::from(current(y_motion)),
        f64::from(current(h_motion)),
    );

    let style = format!(
        "width:{CONNECTOR_POCKET}px;transform:translate({tx}px, {ty}px);height:{th}px;opacity:{};transition:opacity 280ms ease-out;will-change:transform",
        if visible { 1.0 } else { 0.0 }
    );

    rsx! {
        div {
            aria_hidden: "true",
            class: "pointer-events-none absolute left-0 top-0 rounded-r-xl border-y border-r border-border",
            style,
            span {
                class: "absolute left-full top-1/2 h-px bg-border",
                style: "width:{CONNECTOR_STEM}px",
            }
        }
    }
}

/// A card that glides to its computed slot.
#[component]
fn MatchCardLayer(
    match_: Match,
    x: f64,
    y: f64,
    visible: bool,
    reduce: bool,
    label: String,
) -> Element {
    let mut x_motion = use_motion(x as f32).expect("x is finite");
    let mut y_motion = use_motion(y as f32).expect("y is finite");
    use_effect(use_reactive((&x, &y), move |(x, y)| {
        let config = reflow_config(reduce);
        let _ = x_motion.animate_to(x as f32, config.clone());
        let _ = y_motion.animate_to(y as f32, config);
    }));
    let (tx, ty) = (f64::from(current(x_motion)), f64::from(current(y_motion)));

    let style = format!(
        "transform:translate({tx}px, {ty}px);opacity:{};transition:opacity 280ms ease-out;will-change:transform",
        if visible { 1.0 } else { 0.0 }
    );

    rsx! {
        li {
            aria_label: "{label}",
            class: "absolute left-0 top-0",
            style,
            MatchCard { match_ }
        }
    }
}

/// One tie.
#[component]
fn MatchCard(match_: Match) -> Element {
    let finished = is_finished(&match_);
    let decided = finished && match_.winner.is_some();
    let shootout = match_.home.penalties.is_some() || match_.away.penalties.is_some();
    let badge = match_.badge.clone().or_else(|| {
        if finished {
            Some(if shootout { "FT (P)" } else { "FT" }.to_string())
        } else {
            None
        }
    });
    let schedule = [match_.date.clone(), match_.time.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(", ");
    let home_winner = decided && match_.winner == Some(Side::Home);
    let away_winner = decided && match_.winner == Some(Side::Away);

    rsx! {
        div {
            style: "width:{CARD_W}px;height:{CARD_H}px",
            class: "rounded-2xl border border-border bg-card p-4",
            // `h-5` holds the row open for a dateless match.
            div { class: "mb-3 flex h-5 items-center justify-between gap-2",
                span { class: "min-w-0 flex-1 truncate text-sm leading-5 text-muted-foreground", "{schedule}" }
                if let Some(badge) = badge {
                    span { class: "shrink-0 rounded-full bg-background px-2.5 text-xs font-medium leading-5 text-muted-foreground", "{badge}" }
                }
            }
            div { class: "space-y-2.5",
                TeamRow { side: match_.home.clone(), is_winner: home_winner, decided }
                TeamRow { side: match_.away.clone(), is_winner: away_winner, decided }
            }
        }
    }
}

/// One side's row.
#[component]
fn TeamRow(side: MatchSide, is_winner: bool, decided: bool) -> Element {
    let dim = decided && !is_winner;
    let name = side
        .team
        .as_ref()
        .map(|team| team.name.clone())
        .unwrap_or_else(|| "TBD".to_string());
    let score = side.score.map(|score| match side.penalties {
        Some(penalties) => format!("{score} ({penalties})"),
        None => format!("{score}"),
    });

    rsx! {
        div { class: "flex items-center gap-3",
            TeamCrest { team: side.team.clone() }
            span {
                class: cn!("min-w-0 flex-1 truncate text-base font-medium", if dim { "text-muted-foreground" } else { "" }),
                "{name}"
            }
            if let Some(score) = score {
                span {
                    class: cn!("shrink-0 text-base font-medium tabular-nums", if dim { "text-muted-foreground" } else { "" }),
                    "{score}"
                }
            }
            // A fixed 6px marker slot keeps every score right-aligned.
            span { class: "flex w-1.5 shrink-0 items-center",
                if is_winner {
                    svg {
                        view_box: "0 0 6 8",
                        "aria-hidden": "true",
                        class: "h-2 w-1.5 shrink-0 fill-foreground",
                        path { d: "M6 0 0 4 6 8Z" }
                    }
                }
            }
        }
    }
}

/// The fixed 28px crest slot.
#[component]
fn TeamCrest(team: Option<Team>) -> Element {
    let mut failed = use_signal(|| None::<String>);
    let resolved = team.as_ref().and_then(crest_src);
    let src = resolved
        .clone()
        .filter(|src| failed().as_deref() != Some(src.as_str()));

    rsx! {
        span { class: "flex h-5 w-7 shrink-0 items-center justify-center",
            if let Some(src) = src {
                img {
                    src: "{src}",
                    alt: "",
                    loading: "lazy",
                    draggable: "false",
                    class: cn!(
                        "shrink-0 rounded-[4px] border border-border/40",
                        if team.as_ref().is_some_and(|team| team.logo.is_some()) {
                            "h-5 w-5 border-transparent object-contain"
                        } else {
                            "h-5 w-7 object-cover"
                        },
                    ),
                    onerror: move |_| failed.set(Some(src.clone())),
                }
            } else if let Some(team) = team {
                // `text-foreground`, not muted: the /10 tint leaves 4.44:1 light
                // and 3.55:1 dark, both under AA with a muted label.
                span { class: "grid size-5 place-items-center rounded-full bg-foreground/10 text-[10px] font-semibold leading-none text-foreground",
                    "{initials(&team.name)}"
                }
            } else {
                Shield { class: Some("size-5 fill-current text-muted-foreground/50".to_string()) }
            }
        }
    }
}
