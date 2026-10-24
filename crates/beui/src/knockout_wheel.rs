//! `motion/knockout-wheel.tsx` — the same draw as [`crate::KnockoutBracket`],
//! laid out as a wheel: the final at the hub, one ring per round, and a wedge
//! split in half at every step out.
//!
//! The layout is the source's: `SIZE 760` with the centre at 380, `OUTER_R 318`,
//! `HUB_R 34`, `NODE_MIN 14.2` growing by `NODE_STEP 2.2` a ring, `SIBLING_GAP
//! 0.9` for the sibling spread, `HUB_ANGLE 90`, the `quantize`/`pct` rounding
//! that keeps server and client strings identical, the hub's plain radial lines
//! versus the `midR` arc-and-stem for every deeper parent, and `championPath`
//! walking the winners inward. `Team`, `MatchSide`, `Match` and `Round` are
//! reused from the bracket, which is what the source promises ("the same array
//! the knockout bracket takes").
//!
//! Also the source's: the active node as `hovered ?? pinned ?? focused`, the
//! winner path lighting when nothing is active and a single node lighting when
//! something is, the `dimmed` fade riding on the mark, the arrow-key grammar
//! (`Up` to the parent, `Down` to the first child, `Left`/`Right` around the
//! ring and wrapping), `initialRound` trimming the draw so its own teams become
//! the rim, and the per-depth enter stagger (`0.06s` a ring).
//!
//! Differences, both deliberate: the label tooltip is a native
//! `<title>` plus the same `aria-label` rather than another lane's `Tooltip`,
//! and the enter animation is a keyed CSS keyframe with the depth delay instead
//! of `SPRING_PANEL`.

use crate::icons::Shield;
use crate::knockout_bracket::{Match, MatchSide, Round as BracketRound, Side, Team};
use crate::prelude::*;
use beui_core::platform::next_scope_id;
use std::collections::{HashMap, HashSet};

const SIZE: f64 = 760.0;
const CENTER: f64 = SIZE / 2.0;
const TROPHY_SIZE: f64 = 24.0;
const TROPHY_GAP: f64 = 6.0;
const OUTER_R: f64 = 318.0;
const HUB_R: f64 = 34.0;
const NODE_MIN: f64 = 14.2;
const NODE_STEP: f64 = 2.2;
const INITIALS_MIN: f64 = 15.0;
const SIBLING_GAP: f64 = 0.9;
const HUB_ANGLE: f64 = 90.0;

/// A solid trophy glyph drawn in a 24-unit box.
const TROPHY_PATH: &str = "M5 1h12v3h3a2 2 0 0 1 2 2c0 3.3-2.2 5.6-5.2 6A6 6 0 0 1 12 15a6 6 0 0 1-4.8-2.9C4.2 11.6 2 9.3 2 6a2 2 0 0 1 2-2h1V1Zm0 5H4c0 1.9 1.1 3.2 2.6 3.7A12 12 0 0 1 5 6Zm14 0h-1a12 12 0 0 1-1.6 3.7C17.9 9.2 19 7.9 19 6ZM9 16.5h6V19h2.5v2h-11v-2H9v-2.5Z";

const ENTER: &str = "\
@keyframes beui-wheel-in{0%{opacity:0;transform:scale(0.6)}100%{opacity:1;transform:scale(1)}}\
@keyframes beui-wheel-link{0%{opacity:0}100%{opacity:1}}";

/// `quantize`: keeps the coordinates identical across renderers.
fn quantize(value: f64) -> f64 {
    (value * 1e3).round() / 1e3
}

fn polar(radius: f64, degrees: f64) -> (f64, f64) {
    let radians = degrees * std::f64::consts::PI / 180.0;
    (
        quantize(CENTER + radius * radians.cos()),
        quantize(CENTER + radius * radians.sin()),
    )
}

fn point(radius: f64, degrees: f64) -> String {
    let (x, y) = polar(radius, degrees);
    format!("{x:.2} {y:.2}")
}

/// Fixed precision, so the markup is stable.
fn pct(value: f64) -> String {
    format!("{:.4}%", (value / SIZE) * 100.0)
}

/// Names and round labels wrap as a unit.
fn keep_together(text: &str) -> String {
    text.replace(' ', "\u{a0}")
}

fn team_name(side: &MatchSide) -> String {
    keep_together(
        &side
            .team
            .as_ref()
            .map(|team| team.name.clone())
            .unwrap_or_else(|| "TBD".to_string()),
    )
}

/// A `logo` is used as given; a country `code` loads a flag from flagcdn.com.
fn crest_src(team: &Team) -> Option<String> {
    team.logo.clone().or_else(|| {
        team.code
            .clone()
            .map(|code| format!("https://flagcdn.com/w80/{code}.png"))
    })
}

/// Two-letter stand-in when a team has no artwork.
fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect::<String>()
        .to_uppercase()
}

/// `matchLabel`: teams, then the score in the order they were played.
fn match_label(entry: &Match) -> String {
    let teams = format!("{} v {}", team_name(&entry.home), team_name(&entry.away));
    let (Some(home_score), Some(away_score)) = (entry.home.score, entry.away.score) else {
        return teams;
    };
    let pens = match (entry.home.penalties, entry.away.penalties) {
        (Some(home_pens), Some(away_pens)) => format!(" ({home_pens}–{away_pens} pens)"),
        _ => String::new(),
    };
    format!("{teams} · {home_score}–{away_score}{pens}")
}

/// One node on the wheel.
#[derive(Clone, PartialEq)]
struct WheelNode {
    id: String,
    parent_id: Option<String>,
    depth: usize,
    /// Position around the wheel, in degrees; orders arrow-key navigation.
    angle: f64,
    x: f64,
    y: f64,
    r: f64,
    team: Option<Team>,
    label: String,
    /// The round the node's match belongs to; `None` on the rim, which holds teams.
    round: Option<String>,
}

/// One drawn connector.
#[derive(Clone, PartialEq)]
struct WheelLink {
    id: String,
    d: String,
    depth: usize,
}

/// `buildWheel`: walks the match tree from the final outward.
fn build_wheel(rounds: &[BracketRound]) -> (Vec<WheelNode>, Vec<WheelLink>, Option<Team>) {
    let mut nodes: Vec<WheelNode> = Vec::new();
    let mut links: Vec<WheelLink> = Vec::new();
    let layers = rounds.len();
    let ring_r = |depth: usize| (depth as f64 / layers.max(1) as f64) * OUTER_R;
    let node_r = |depth: usize| NODE_MIN + (depth.saturating_sub(1)) as f64 * NODE_STEP;

    let Some(final_round) = rounds.last() else {
        return (nodes, links, None);
    };
    let Some(entry) = final_round.matches.first() else {
        return (nodes, links, None);
    };

    let champion = entry.winner.and_then(|side| match side {
        Side::Home => entry.home.team.clone(),
        Side::Away => entry.away.team.clone(),
    });
    nodes.push(WheelNode {
        id: entry.id.clone(),
        parent_id: None,
        depth: 0,
        angle: HUB_ANGLE,
        x: CENTER,
        y: CENTER,
        r: HUB_R,
        team: champion.clone(),
        label: match_label(entry),
        round: Some(final_round.name.clone()),
    });

    walk(
        rounds,
        entry,
        layers - 1,
        0,
        // The hub is the parent of the final's two feeders.
        Some(entry.id.as_str()),
        0,
        HUB_ANGLE,
        360.0,
        &ring_r,
        &node_r,
        &mut nodes,
        &mut links,
    );

    (nodes, links, champion)
}

/// The recursive step: lays one match's two feeders and recurses into them.
#[allow(clippy::too_many_arguments)]
fn walk(
    rounds: &[BracketRound],
    entry: &Match,
    round_index: usize,
    index: usize,
    parent_id: Option<&str>,
    parent_depth: usize,
    angle: f64,
    wedge: f64,
    ring_r: &dyn Fn(usize) -> f64,
    node_r: &dyn Fn(usize) -> f64,
    nodes: &mut Vec<WheelNode>,
    links: &mut Vec<WheelLink>,
) {
    let depth = parent_depth + 1;
    let radius = ring_r(depth);
    let r = node_r(depth);
    // The hub's two feeders sit opposite each other, so they get plain radial
    // lines; an arc between them would be a half circle.
    let offset = (wedge / 4.0) * if parent_depth == 0 { 1.0 } else { SIBLING_GAP };
    let angles = [angle - offset, angle + offset];

    let mut children: Vec<(WheelNode, f64, Option<Match>)> = Vec::new();
    for (side_index, child_angle) in angles.iter().copied().enumerate() {
        let (x, y) = polar(radius, child_angle);
        let feeder = if round_index > 0 {
            rounds
                .get(round_index - 1)
                .and_then(|round| round.matches.get(2 * index + side_index))
                .cloned()
        } else {
            None
        };
        let side = if side_index == 0 {
            &entry.home
        } else {
            &entry.away
        };
        let node = match &feeder {
            Some(feeder) => WheelNode {
                id: feeder.id.clone(),
                parent_id: parent_id.map(|id| id.to_string()),
                depth,
                angle: child_angle,
                x,
                y,
                r,
                team: feeder.winner.and_then(|side| match side {
                    Side::Home => feeder.home.team.clone(),
                    Side::Away => feeder.away.team.clone(),
                }),
                label: match_label(feeder),
                round: rounds
                    .get(round_index.saturating_sub(1))
                    .map(|round| round.name.clone()),
            },
            None => WheelNode {
                id: format!(
                    "{}-{}",
                    entry.id,
                    if side_index == 0 { "home" } else { "away" }
                ),
                parent_id: parent_id.map(|id| id.to_string()),
                depth,
                angle: child_angle,
                x,
                y,
                r,
                team: side.team.clone(),
                label: keep_together(
                    &side
                        .team
                        .as_ref()
                        .map(|team| team.name.clone())
                        .unwrap_or_else(|| "TBD".to_string()),
                ),
                round: None,
            },
        };
        nodes.push(node.clone());
        children.push((node, child_angle, feeder));
    }

    let parent_id = parent_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| entry.id.clone());
    if parent_depth == 0 {
        for (child, child_angle, _) in children.iter() {
            links.push(WheelLink {
                id: format!("{parent_id}-{}", child.id),
                d: format!("M {} L {CENTER} {CENTER}", point(radius, *child_angle)),
                depth,
            });
        }
    } else {
        let mid_r = (ring_r(parent_depth) + radius) / 2.0;
        links.push(WheelLink {
            id: format!("{parent_id}-arc"),
            d: format!(
                "M {} L {} A {mid_r} {mid_r} 0 0 1 {} L {}",
                point(radius, angles[0]),
                point(mid_r, angles[0]),
                point(mid_r, angles[1]),
                point(radius, angles[1]),
            ),
            depth,
        });
        links.push(WheelLink {
            id: format!("{parent_id}-stem"),
            d: format!(
                "M {} L {}",
                point(mid_r, angle),
                point(ring_r(parent_depth), angle)
            ),
            depth,
        });
    }

    for (side_index, (child, child_angle, feeder)) in children.into_iter().enumerate() {
        if let Some(feeder) = feeder {
            walk(
                rounds,
                &feeder,
                round_index.saturating_sub(1),
                2 * index + side_index,
                Some(&child.id),
                depth,
                child_angle,
                wedge / 2.0,
                ring_r,
                node_r,
                nodes,
                links,
            );
        }
    }
}

/// `championPath`: match ids from the hub down to the champion's first-round win.
fn champion_path(rounds: &[BracketRound]) -> HashSet<String> {
    let mut path = HashSet::new();
    let mut index = 0usize;
    for round in rounds.iter().rev() {
        let Some(entry) = round.matches.get(index) else {
            return path;
        };
        let Some(winner) = entry.winner else {
            return path;
        };
        path.insert(entry.id.clone());
        index = 2 * index
            + match winner {
                Side::Home => 0,
                Side::Away => 1,
            };
    }
    path
}

/// Props for [`KnockoutWheel`].
#[derive(Props, Clone, PartialEq)]
pub struct KnockoutWheelProps {
    pub rounds: Vec<BracketRound>,
    /// Index of the outermost round to draw. Defaults to `0`.
    #[props(default)]
    pub initial_round: usize,
    #[props(default)]
    pub class: Option<String>,
}

/// A radial single-elimination draw.
#[component]
pub fn KnockoutWheel(props: KnockoutWheelProps) -> Element {
    let KnockoutWheelProps {
        rounds,
        initial_round,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let can_hover = use_hover_capable();
    let can_hover_value = can_hover();
    let scope = use_hook(next_scope_id);
    let uid = scope.to_string();

    let hovered = use_signal(|| None::<String>);
    let pinned = use_signal(|| None::<String>);
    let focused = use_signal(|| None::<String>);

    let active = hovered().or(pinned()).or(focused());
    let from = initial_round.min(rounds.len().saturating_sub(1));
    let visible: Vec<BracketRound> = rounds.iter().skip(from).cloned().collect();
    let (nodes, links, champion) = build_wheel(&visible);
    let winners = champion_path(&visible);
    let active_node = active
        .as_ref()
        .and_then(|id| nodes.iter().find(|node| &node.id == id))
        .cloned();
    let lit: HashSet<String> = match &active_node {
        Some(node) => HashSet::from([node.id.clone()]),
        None => winners.clone(),
    };
    // With a finger there is no hover, so the pinned node is what a tap shows.
    let tapped = if can_hover_value {
        None
    } else {
        active_node.clone()
    };

    // Peers per ring, sorted by angle, and the first child per node: the arrow
    // keys walk those.
    let mut ring: HashMap<usize, Vec<String>> = HashMap::new();
    let mut first_child: HashMap<String, String> = HashMap::new();
    {
        let mut by_depth: HashMap<usize, Vec<(f64, String)>> = HashMap::new();
        for node in nodes.iter() {
            by_depth
                .entry(node.depth)
                .or_default()
                .push((node.angle, node.id.clone()));
            if let Some(parent) = node.parent_id.clone() {
                first_child.entry(parent).or_insert(node.id.clone());
            }
        }
        for (depth, mut peers) in by_depth {
            peers.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            ring.insert(depth, peers.into_iter().map(|(_, id)| id).collect());
        }
    }

    let on_key = EventHandler::new({
        let uid = uid.clone();
        let nodes_for_key = nodes.clone();
        let ring = ring.clone();
        let first_child = first_child.clone();
        move |(node_id, key): (String, Key)| {
            let Some(node) = nodes_for_key.iter().find(|node| node.id == node_id) else {
                return;
            };
            let target: Option<String> = match key {
                Key::ArrowUp => node.parent_id.clone(),
                Key::ArrowDown => first_child.get(&node.id).cloned(),
                Key::ArrowRight | Key::ArrowLeft => {
                    let peers = ring.get(&node.depth).cloned().unwrap_or_default();
                    if peers.len() < 2 {
                        None
                    } else {
                        let at = peers.iter().position(|id| id == &node.id).unwrap_or(0);
                        let next = if key == Key::ArrowRight {
                            at + 1
                        } else {
                            at + peers.len() - 1
                        };
                        peers.get(next % peers.len()).cloned()
                    }
                }
                _ => None,
            };
            let Some(target) = target else {
                return;
            };
            let id = format!("{uid}-{target}");
            spawn(async move {
                let script = format!(
                    "const el = document.getElementById({});\nif (el) el.focus();\nreturn true;",
                    beui_core::platform::js_string(&id)
                );
                let _ = beui_core::platform::json::<bool>(&script).await;
            });
        }
    });

    let on_hover = EventHandler::new({
        let mut hovered = hovered;
        move |id: Option<String>| hovered.set(id)
    });
    let on_focus = EventHandler::new({
        let mut focused = focused;
        move |id: Option<String>| focused.set(id)
    });
    let on_toggle = EventHandler::new({
        let mut pinned = pinned;
        move |id: String| {
            if pinned().as_deref() == Some(id.as_str()) {
                pinned.set(None);
            } else {
                pinned.set(Some(id));
            }
        }
    });

    let tab_stop = active_node
        .as_ref()
        .map(|node| node.id.clone())
        .or_else(|| nodes.first().map(|node| node.id.clone()));
    let root_class = cn!("relative mx-auto w-full min-w-[32rem] max-w-[34rem]", class,);
    let trophy_y = CENTER - HUB_R - TROPHY_GAP - TROPHY_SIZE;
    let glow_id = format!("{uid}-glow");
    let enter_delay = |depth: usize| if reduce_value { 0 } else { depth * 60 };
    let aria = match champion.clone() {
        Some(champion) => format!("Tournament wheel, won by {}", champion.name),
        None => "Tournament wheel".to_string(),
    };
    let anchors: Vec<Element> = nodes
        .iter()
        .map(|node| {
            rsx! {
                WheelAnchor {
                    key: "{node.id}",
                    uid: uid.clone(),
                    node_id: node.id.clone(),
                    x: node.x,
                    y: node.y,
                    r: node.r,
                    caption: node.label.clone(),
                    is_tab_stop: tab_stop.as_deref() == Some(node.id.as_str()),
                    on_hover,
                    on_focus,
                    on_toggle,
                    on_key,
                }
            }
        })
        .collect();

    rsx! {
        style { dangerous_inner_html: ENTER }
        div { class: root_class,
            svg {
                view_box: "0 0 760 760",
                role: "img",
                "aria-label": "{aria}",
                class: "h-auto w-full touch-manipulation",
                defs {
                    radialGradient { id: "{glow_id}",
                        stop { offset: "0%", stop_color: "currentColor", stop_opacity: "0.18" }
                        stop { offset: "100%", stop_color: "currentColor", stop_opacity: "0" }
                    }
                }
                circle {
                    cx: "380",
                    cy: "380",
                    r: "{OUTER_R}",
                    fill: "url(#{glow_id})",
                    class: "text-foreground",
                }
                g {
                    for link in links.iter() {
                        path {
                            key: "{link.id}",
                            d: "{link.d}",
                            fill: "none",
                            stroke_width: "1",
                            // A link lights when either end is active.
                            class: if lit.iter().any(|id| link.id.starts_with(id.as_str()) || link.id.contains(id.as_str())) {
                                "stroke-foreground"
                            } else {
                                "stroke-border"
                            },
                            style: if reduce_value {
                                String::new()
                            } else {
                                format!(
                                    "animation:beui-wheel-link 300ms ease-out {}ms both",
                                    link.depth * 60
                                )
                            },
                        }
                    }
                }
                for node in nodes.iter() {
                    WheelMark {
                        key: "{node.id}",
                        node: node.clone(),
                        uid: uid.clone(),
                        lit: lit.contains(&node.id),
                        dimmed: !lit.is_empty() && !lit.contains(&node.id),
                        reduce: reduce_value,
                        delay: enter_delay(node.depth),
                    }
                }
                if champion.is_some() {
                    g { transform: "translate({CENTER - TROPHY_SIZE / 2.0}, {trophy_y})",
                        path { d: TROPHY_PATH, class: "fill-amber-400" }
                    }
                }
            }
            for anchor in anchors {
                {anchor}
            }
            if let Some(node) = tapped.clone() {
                if node.y > CENTER {
                    span { class: "pointer-events-none absolute left-1/2 max-w-[20rem] -translate-x-1/2 translate-y-2 whitespace-normal text-balance break-words text-center text-xs text-muted-foreground",
                        style: "top:{pct(node.y + node.r)}",
                        "{node.label}"
                    }
                }
            }
        }
    }
}

/// One node: its disc, artwork and ring.
#[component]
fn WheelMark(
    node: WheelNode,
    uid: String,
    lit: bool,
    dimmed: bool,
    reduce: bool,
    delay: usize,
) -> Element {
    let mut failed = use_signal(|| None::<String>);
    let resolved = node.team.as_ref().and_then(crest_src);
    let src = resolved
        .clone()
        .filter(|src| failed().as_deref() != Some(src.as_str()));
    let fade = if dimmed { 0.38 } else { 1.0 };
    let clip_id = format!("{uid}-clip-{}", node.id);
    let is_logo = node.team.as_ref().is_some_and(|team| team.logo.is_some());
    let (box_w, box_h, fit) = if is_logo {
        (node.r * 1.44, node.r * 1.44, "xMidYMid meet")
    } else {
        (node.r * 2.68, node.r * 2.0, "xMidYMid slice")
    };
    let mark_style = if reduce {
        String::new()
    } else {
        format!(
            "animation:beui-wheel-in 320ms cubic-bezier(0.16, 1, 0.3, 1) {delay}ms both;transform-origin:{}px {}px",
            node.x, node.y
        )
    };
    let label = node.label.clone();
    let shield_transform = format!(
        "translate({}, {}) scale({})",
        node.x - node.r * 0.7,
        node.y - node.r * 0.7,
        (node.r * 1.4) / 24.0
    );

    rsx! {
        g { style: mark_style,
            title { "{label}" }
            // Opaque at every state: the links are routed underneath.
            circle { cx: "{node.x}", cy: "{node.y}", r: "{node.r}", class: "fill-card" }
            if let Some(src) = src.clone() {
                clipPath { id: "{clip_id}",
                    circle { cx: "{node.x}", cy: "{node.y}", r: "{node.r}" }
                }
                image {
                    href: "{src}",
                    x: "{node.x - box_w / 2.0}",
                    y: "{node.y - box_h / 2.0}",
                    width: "{box_w}",
                    height: "{box_h}",
                    clip_path: "url(#{clip_id})",
                    preserve_aspect_ratio: fit,
                    opacity: "{fade}",
                    onerror: {
                        let src = src.clone();
                        move |_| failed.set(Some(src.clone()))
                    },
                }
            } else if let Some(team) = node.team.clone() {
                text {
                    x: "{node.x}",
                    y: "{node.y}",
                    text_anchor: "middle",
                    dominant_baseline: "central",
                    font_size: "{(node.r * 0.8).max(INITIALS_MIN)}",
                    opacity: "{fade}",
                    class: "fill-muted-foreground font-semibold",
                    "{initials(&team.name)}"
                }
            } else {
                // The icon is a 24-unit box; scale it into the disc.
                g {
                    transform: shield_transform,
                    Shield { class: Some("fill-current text-muted-foreground/50".to_string()) }
                }
            }
            circle {
                cx: "{node.x}",
                cy: "{node.y}",
                r: "{node.r}",
                fill: "none",
                stroke_width: if lit { "2" } else { "1" },
                class: if lit { "stroke-foreground" } else { "stroke-border" },
            }
        }
    }
}

/// The invisible hit area over one node: hover, focus, tap and arrows.
#[component]
fn WheelAnchor(
    uid: String,
    node_id: String,
    x: f64,
    y: f64,
    r: f64,
    caption: String,
    is_tab_stop: bool,
    on_hover: EventHandler<Option<String>>,
    on_focus: EventHandler<Option<String>>,
    on_toggle: EventHandler<String>,
    on_key: EventHandler<(String, Key)>,
) -> Element {
    let style = format!(
        "left:{};top:{};width:{};height:{}",
        pct(x - r),
        pct(y - r),
        pct(r * 2.0),
        pct(r * 2.0)
    );
    let id = format!("{uid}-{node_id}");
    let class = "absolute rounded-full outline-none focus-visible:ring-2 focus-visible:ring-foreground focus-visible:ring-offset-2 focus-visible:ring-offset-background";

    rsx! {
        button {
            r#type: "button",
            id: "{id}",
            tabindex: if is_tab_stop { 0 } else { -1 },
            aria_label: "{caption}",
            title: "{caption}",
            class,
            style,
            onpointerenter: {
                let id = node_id.clone();
                move |event: Event<PointerData>| {
                    if event.pointer_type() == "touch" {
                        return;
                    }
                    on_hover.call(Some(id.clone()));
                }
            },
            onpointerleave: move |event: Event<PointerData>| {
                if event.pointer_type() == "touch" {
                    return;
                }
                on_hover.call(None);
            },
            onfocus: {
                let id = node_id.clone();
                move |_| on_focus.call(Some(id.clone()))
            },
            onblur: move |_| on_focus.call(None),
            onclick: {
                let id = node_id.clone();
                move |_| on_toggle.call(id.clone())
            },
            onkeydown: {
                let id = node_id.clone();
                move |event: Event<KeyboardData>| {
                    let key = event.key();
                    if !matches!(
                        key,
                        Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight
                    ) {
                        return;
                    }
                    // Arrows drive the wheel here, so they must not scroll.
                    event.prevent_default();
                    on_key.call((id.clone(), key));
                }
            },
        }
    }
}
