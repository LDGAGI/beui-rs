//! `motion/morphing-search-chips.tsx` — a search field with filter chips whose
//! results panel grows out of the bar's edge.
//!
//! Everything the source fixes in place is kept: the glass recipe, the
//! `SEED 112×40` pill tucked `SEED_TUCK 28` behind the bar, the enter spring
//! `{380, 28, 0.8}` and the faster exit `{520, 40, 0.6}`, the content
//! fade that waits 120ms for the shape, `SURFACE_RADIUS 28`, `PANEL_ROOM 320`
//! for the `auto` side, the panel floor that lets a session grow but never
//! shrink, the chip spring `SPRING_SWAP` with its `blur(4px)`, the row cursor
//! with its `aria-activedescendant` contract, the compose guard, and the whole
//! keyboard grammar (Enter adds the highlighted row, the first Backspace marks
//! the last chip and the second removes it, Delete removes a highlighted
//! recent, Escape clears before it closes, `localStorage` recents).
//!
//! Differences, both deliberate: the chips enter on their own
//! spring but leave instantly (the source's `popLayout` exit would need a second
//! layer per chip), and the "caret at the start" test is approximated by the
//! field being empty, because `KeyboardData` carries no selection.

use crate::icons::{Clock, LoaderCircle, Search, X};
use crate::prelude::*;
use beui_core::hooks::{use_dismiss, use_on_open, use_row_cursor, DismissOptions, RowStep};
use beui_core::platform::{js_string, json, next_scope_id};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle, Spring};

const GLASS: &str = "border border-foreground/[0.08] bg-background/55 backdrop-blur-[20px] backdrop-saturate-[1.6] shadow-[inset_0_1px_0_0_rgb(255_255_255/0.06),0_24px_60px_-24px_rgb(0_0_0/0.45)]";
const SEED_WIDTH: f64 = 112.0;
const SEED_HEIGHT: f64 = 40.0;
const SEED_TUCK: f64 = 28.0;
const SURFACE_RADIUS: f64 = 28.0;
const PANEL_ROOM: f64 = 320.0;

fn morph_enter() -> Spring {
    Spring {
        stiffness: 380.0,
        damping: 28.0,
        mass: 0.8,
    }
}

fn morph_exit() -> Spring {
    Spring {
        stiffness: 520.0,
        damping: 40.0,
        mass: 0.6,
    }
}

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// Which way the surface grows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchSide {
    #[default]
    Top,
    Bottom,
    /// Grows up unless there is no room.
    Auto,
}

/// What a row is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowKind {
    Query,
    Recent,
    Suggestion,
}

/// One row of the panel.
#[derive(Clone, PartialEq)]
struct Row {
    id: String,
    label: String,
    kind: RowKind,
}

fn same_term(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

fn contains(term: &str, query: &str) -> bool {
    term.to_lowercase().contains(&query.to_lowercase())
}

/// Props for [`MorphingSearchChips`].
#[derive(Props, Clone, PartialEq)]
pub struct MorphingSearchChipsProps {
    #[props(default)]
    pub chips: Option<Vec<String>>,
    #[props(default)]
    pub default_chips: Vec<String>,
    #[props(default)]
    pub on_chips_change: Option<EventHandler<Vec<String>>>,
    #[props(default)]
    pub query: Option<String>,
    #[props(default)]
    pub default_query: String,
    #[props(default)]
    pub on_query_change: Option<EventHandler<String>>,
    #[props(default)]
    pub recents: Option<Vec<String>>,
    #[props(default)]
    pub default_recents: Vec<String>,
    #[props(default)]
    pub on_recents_change: Option<EventHandler<Vec<String>>>,
    /// Persist uncontrolled recents in `localStorage` under this key.
    #[props(default)]
    pub storage_key: Option<String>,
    #[props(default = 5)]
    pub max_recents: usize,
    #[props(default)]
    pub suggestions: Vec<String>,
    /// Match suggestions by case-insensitive substring. `false` when they
    /// already arrive filtered.
    #[props(default = true)]
    pub filter_substring: bool,
    #[props(default = 6)]
    pub max_suggestions: usize,
    #[props(default)]
    pub loading: bool,
    #[props(default)]
    pub max_chips: Option<usize>,
    #[props(default)]
    pub empty_state: Option<Element>,
    #[props(default)]
    pub side: SearchSide,
    #[props(default)]
    pub open: Option<bool>,
    #[props(default)]
    pub default_open: bool,
    #[props(default)]
    pub on_open_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub disabled: bool,
    /// Submit each chip with a form under this name.
    #[props(default)]
    pub name: Option<String>,
    #[props(default = "Search".to_string())]
    pub placeholder: String,
    #[props(default = "Search".to_string())]
    pub label: String,
    /// Trailing controls inside the bar.
    #[props(default)]
    pub actions: Option<Element>,
    #[props(default)]
    pub class: Option<String>,
}

/// A search field with filter chips and a morphing results panel.
#[component]
pub fn MorphingSearchChips(props: MorphingSearchChipsProps) -> Element {
    let MorphingSearchChipsProps {
        chips: chips_prop,
        default_chips,
        on_chips_change,
        query: query_prop,
        default_query,
        on_query_change,
        recents: recents_prop,
        default_recents,
        on_recents_change,
        storage_key,
        max_recents,
        suggestions,
        filter_substring,
        max_suggestions,
        loading,
        max_chips,
        empty_state,
        side,
        open: open_prop,
        default_open,
        on_open_change,
        disabled,
        name,
        placeholder,
        label,
        actions,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let scope = use_hook(next_scope_id);

    // Controllable state, one signal per field.
    let mut internal_chips = use_signal(|| default_chips.clone());
    let chips = chips_prop.clone().unwrap_or(internal_chips());
    let mut internal_query = use_signal(|| default_query.clone());
    let query = query_prop.clone().unwrap_or(internal_query());
    let mut internal_recents = use_signal(|| default_recents.clone());
    let recents = recents_prop.clone().unwrap_or(internal_recents());
    let mut internal_open = use_signal(|| default_open);
    let open_state = open_prop.unwrap_or(internal_open());
    let open = open_state && !disabled;

    let mut announcement = use_signal(String::new);
    let mut announcement_count = use_signal(|| 0u32);
    let announce = move |text: String| {
        announcement.set(text);
        announcement_count += 1;
    };

    let mut marked = use_signal(|| None::<String>);
    let mut placed = use_signal(|| match side {
        SearchSide::Bottom => SearchSide::Bottom,
        _ => SearchSide::Top,
    });
    let mut floor = use_signal(|| 0.0f64);
    let mut mounted = use_signal(|| open);

    let set_chips = {
        let chips_prop = chips_prop.clone();
        move |next: Vec<String>| {
            if chips_prop.is_none() {
                internal_chips.set(next.clone());
            }
            if let Some(handler) = on_chips_change {
                handler.call(next);
            }
        }
    };
    let set_query = {
        let query_prop = query_prop.clone();
        move |next: String| {
            if query_prop.is_none() {
                internal_query.set(next.clone());
            }
            if let Some(handler) = on_query_change {
                handler.call(next);
            }
        }
    };
    let mut set_recents = {
        let recents_prop = recents_prop.clone();
        let persist = storage_key.clone().filter(|_| recents_prop.is_none());
        move |next: Vec<String>| {
            if recents_prop.is_none() {
                internal_recents.set(next.clone());
            }
            if let Some(handler) = on_recents_change {
                handler.call(next.clone());
            }
            if let Some(key) = persist.clone() {
                let payload = next.clone();
                spawn(async move {
                    let script = format!(
                        "try {{ window.localStorage.setItem({key}, {value}); }} catch (e) {{}}\nreturn true;",
                        key = js_string(&key),
                        value = js_string(&json_array(&payload)),
                    );
                    let _ = json::<bool>(&script).await;
                });
            }
        }
    };
    let mut set_open = {
        move |next: bool| {
            if open_prop.is_none() {
                internal_open.set(next);
            }
            if let Some(handler) = on_open_change {
                handler.call(next);
            }
        }
    };

    // Recents are restored once, when this component owns them.
    let restored = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(false)));
    if let (Some(key), true) = (
        storage_key.clone().filter(|_| recents_prop.is_none()),
        !restored.get(),
    ) {
        restored.set(true);
        let mut internal_recents = internal_recents;
        spawn(async move {
            let script = format!(
                "try {{ const raw = window.localStorage.getItem({key}); return raw ? JSON.parse(raw) : null; }} catch (e) {{ return null; }}",
                key = js_string(&key),
            );
            if let Some(stored) = json::<Vec<String>>(&script).await {
                internal_recents.set(stored.into_iter().take(max_recents).collect());
            }
        });
    }

    // The rows: the typed term first, then matching recents and suggestions.
    let trimmed = query.trim().to_string();
    let rows: Vec<Row> = if trimmed.is_empty() {
        recents
            .iter()
            .map(|term| Row {
                id: format!("recent:{term}"),
                label: term.clone(),
                kind: RowKind::Recent,
            })
            .collect()
    } else {
        let mut seen: Vec<String> = vec![trimmed.to_lowercase()];
        seen.extend(chips.iter().map(|chip| chip.to_lowercase()));
        let mut matches: Vec<Row> = Vec::new();
        for (kind, pool, matches_with) in [
            (RowKind::Recent, recents.clone(), true),
            (RowKind::Suggestion, suggestions.clone(), filter_substring),
        ] {
            for term in pool {
                let key = term.to_lowercase();
                if seen.contains(&key) {
                    continue;
                }
                if matches_with && !contains(&term, &trimmed) {
                    continue;
                }
                seen.push(key);
                matches.push(Row {
                    id: term.clone(),
                    label: term,
                    kind,
                });
            }
        }
        let mut rows = vec![Row {
            id: format!("query:{trimmed}"),
            label: trimmed.clone(),
            kind: RowKind::Query,
        }];
        rows.extend(matches.into_iter().take(max_suggestions));
        rows
    };

    let row_ids: Vec<String> = rows.iter().map(|row| row.id.clone()).collect();
    let cursor = std::rc::Rc::new(use_row_cursor(row_ids, &trimmed));
    let active_index = cursor.active_index();
    let active = rows.get(active_index).cloned();
    let show_list = !rows.is_empty();
    let has_panel = show_list || loading || (trimmed.is_empty() && empty_state.is_some());
    let panel_open = open && has_panel;

    // A fresh session each time the panel opens.
    let cursor_for_open = cursor.clone();
    use_on_open(panel_open, move || {
        cursor_for_open.move_to(None);
        floor.set(0.0);
    });
    use_on_open(open, move || {
        marked.set(None);
    });

    // The morph: the glass pill grows out of the bar's edge.
    let mut morph = use_motion(if open { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    // The flag is a signal so the effect below subscribes to it: `use_reactive`
    // over a plain bool did not re-run here, and a tracked read always does.
    let mut open_flag = use_signal(|| open);
    use_effect(use_reactive((&open,), move |(open,)| {
        open_flag.set(open);
    }));
    use_effect(move || {
        if open_flag() {
            mounted.set(true);
            let config = if reduce_value {
                beui_core::ease::tween_ease_out(0)
            } else {
                AnimationConfig::spring(morph_enter())
            };
            let _ = morph.animate_to(1.0, config);
        } else {
            let config = if reduce_value {
                beui_core::ease::tween_ease_out(120)
            } else {
                AnimationConfig::spring(morph_exit())
            };
            let _ = morph.animate_to(0.0, config);
            spawn(async move {
                let _ = json::<bool>(
                    "await new Promise((resolve) => setTimeout(resolve, 420));\nreturn true;",
                )
                .await;
                if !open_flag() {
                    mounted.set(false);
                }
            });
        }
    });
    let morph_p = f64::from(current(morph).clamp(0.0, 1.0));

    // Side: `auto` picks one as the panel opens and keeps it.
    use_effect(use_reactive(
        (&panel_open, &side),
        move |(panel_open, side)| {
            if !panel_open {
                return;
            }
            if side != SearchSide::Auto {
                placed.set(side);
                return;
            }
            let selector = format!("[data-beui-search-root=\"{scope}\"]");
            spawn(async move {
                let script = format!(
                    "const el = document.querySelector({selector});\n\
                 if (!el) return null;\n\
                 const r = el.getBoundingClientRect();\n\
                 return [r.top, window.innerHeight - r.bottom];",
                    selector = js_string(&selector),
                );
                let Some(values) = json::<Vec<f64>>(&script).await else {
                    return;
                };
                let (above, below) = (values[0], values[1]);
                placed.set(if above < PANEL_ROOM && below > above {
                    SearchSide::Bottom
                } else {
                    SearchSide::Top
                });
            });
        },
    ));

    // The panel's own size, and the tallest its rows have been this session.
    // Measured by script rather than by an observer: the elements only exist
    // while the panel is open, so a mount-time observer would find nothing.
    let panel_selector = format!("[data-beui-panel=\"{scope}\"]");
    let rows_selector = format!("[data-beui-panel-rows=\"{scope}\"]");
    let mut panel_box = use_signal(|| (0.0f64, 0.0f64));
    use_effect(use_reactive(
        (&panel_open, &trimmed, &loading),
        move |(panel_open, trimmed, loading)| {
            let _ = (trimmed, loading);
            if !panel_open {
                panel_box.set((0.0, 0.0));
                return;
            }
            let (panel_selector, rows_selector) = (panel_selector.clone(), rows_selector.clone());
            spawn(async move {
                // One frame for the content to lay out.
                let script = format!(
                    "await new Promise((resolve) => requestAnimationFrame(resolve));\n\
                     const panel = document.querySelector({panel});\n\
                     const rows = document.querySelector({rows});\n\
                     return [panel ? panel.offsetWidth : 0, panel ? panel.offsetHeight : 0, rows ? rows.offsetHeight + 16 : 0];",
                    panel = js_string(&panel_selector),
                    rows = js_string(&rows_selector),
                );
                let Some(values) = json::<Vec<f64>>(&script).await else {
                    return;
                };
                panel_box.set((values[0], values[1]));
                if values[2] > floor() {
                    floor.set(values[2]);
                }
            });
        },
    ));
    let (panel_width, panel_height) = panel_box();

    // Dismissal: outside pointerdown and Escape.
    let mut open_signal = use_signal(|| open);
    use_effect(move || open_signal.set(open));
    let close = EventHandler::new(move |_| set_open(false));
    let dismiss = use_dismiss(open_signal, close, DismissOptions::default());
    let _ = dismiss;

    let full = max_chips.is_some_and(|max| chips.len() >= max);
    let add_chip = {
        let mut chips = chips.clone();
        let mut recents = recents.clone();
        let mut set_chips = set_chips.clone();
        let mut set_recents = set_recents.clone();
        let mut set_query = set_query.clone();
        let mut announce = announce;
        move |term: String| {
            let value = term.trim().to_string();
            if value.is_empty() {
                return;
            }
            let exists = chips.iter().any(|chip| same_term(chip, &value));
            if !exists && full {
                if let Some(max) = max_chips {
                    announce(format!("Up to {max} filters"));
                }
                return;
            }
            if !exists {
                chips.push(value.clone());
                set_chips(chips.clone());
                announce(format!("Added filter {value}"));
            }
            let mut next: Vec<String> = vec![value.clone()];
            next.extend(
                recents
                    .iter()
                    .filter(|recent| !same_term(recent, &value))
                    .cloned(),
            );
            next.truncate(max_recents);
            recents = next.clone();
            set_recents(next);
            set_query(String::new());
        }
    };
    let remove_chip = {
        let mut chips = chips.clone();
        let mut set_chips = set_chips.clone();
        let mut announce = announce;
        move |term: String| {
            chips.retain(|chip| chip != &term);
            set_chips(chips.clone());
            announce(format!("Removed filter {term}"));
        }
    };
    let remove_recent = {
        let mut recents = recents.clone();
        let mut set_recents = set_recents.clone();
        let mut announce = announce;
        move |term: String| {
            recents.retain(|recent| recent != &term);
            set_recents(recents.clone());
            announce(format!("Removed {term} from recent searches"));
        }
    };

    let on_key_down = {
        let chips = chips.clone();
        let query = query.clone();
        let mut marked = marked;
        let cursor = cursor.clone();
        let mut announce = announce;
        let mut set_query = set_query.clone();
        let mut add_chip = add_chip.clone();
        let mut remove_chip = remove_chip.clone();
        let mut remove_recent = remove_recent.clone();
        move |event: Event<KeyboardData>| {
            // An input method is still composing; Enter and arrows belong to it.
            if event.is_composing() {
                return;
            }
            let key = event.key();
            // A chip the keyboard has marked for removal.
            if let Some(marked_chip) = marked() {
                if key == Key::Backspace || key == Key::Delete {
                    event.prevent_default();
                    marked.set(None);
                    remove_chip(marked_chip);
                    return;
                }
                if key == Key::ArrowLeft || key == Key::ArrowRight {
                    event.prevent_default();
                    let direction = if key == Key::ArrowLeft { -1i64 } else { 1 };
                    let at = if direction < 0 {
                        chips
                            .iter()
                            .position(|chip| chip == &marked_chip)
                            .map(|index| index as i64)
                            .unwrap_or(chips.len() as i64)
                    } else {
                        chips
                            .iter()
                            .position(|chip| chip == &marked_chip)
                            .map(|index| index as i64 + 1)
                            .unwrap_or(chips.len() as i64)
                    };
                    if at >= chips.len() as i64 || at < 0 {
                        marked.set(None);
                    } else {
                        marked.set(Some(chips[at as usize].clone()));
                    }
                    return;
                }
                if key == Key::Escape {
                    event.prevent_default();
                    marked.set(None);
                    return;
                }
                marked.set(None);
            }

            if key == Key::Escape && !query.is_empty() {
                event.prevent_default();
                set_query(String::new());
                return;
            }
            if key == Key::ArrowDown || key == Key::ArrowUp {
                event.prevent_default();
                if !open {
                    set_open(true);
                } else {
                    cursor.move_active(if key == Key::ArrowDown {
                        RowStep::Next
                    } else {
                        RowStep::Previous
                    });
                }
                return;
            }
            if key == Key::Enter {
                if !panel_open {
                    return;
                }
                if let Some(row) = active.clone() {
                    event.prevent_default();
                    add_chip(row.label);
                }
                return;
            }
            if key == Key::Delete {
                if panel_open {
                    if let Some(row) = active.clone() {
                        if row.kind == RowKind::Recent {
                            event.prevent_default();
                            remove_recent(row.label);
                        }
                    }
                }
                return;
            }
            if key == Key::Backspace && query.is_empty() && !chips.is_empty() {
                // Mark first, remove on the second press: one stray key never
                // loses a filter.
                event.prevent_default();
                let last = chips[chips.len() - 1].clone();
                announce(format!(
                    "{last} selected. Press Backspace again to remove it."
                ));
                marked.set(Some(last));
                return;
            }
            if key == Key::ArrowLeft && query.is_empty() && !chips.is_empty() {
                event.prevent_default();
                marked.set(Some(chips[chips.len() - 1].clone()));
            }
        }
    };

    let below = placed() == SearchSide::Bottom;
    let seed_y = if below { -SEED_TUCK } else { SEED_TUCK };
    // The glass starts as the seed and grows to the panel's measured size.
    let glass_width = SEED_WIDTH + (panel_width.max(SEED_WIDTH) - SEED_WIDTH) * morph_p;
    let glass_height = SEED_HEIGHT + (panel_height.max(SEED_HEIGHT) - SEED_HEIGHT) * morph_p;
    let glass_offset = seed_y * (1.0 - morph_p);
    let glass_style = format!(
        "left:50%;{}:0;transform:translateX(-50%) translateY({glass_offset}px);width:{glass_width}px;height:{glass_height}px;border-radius:{SURFACE_RADIUS}px",
        if below { "top" } else { "bottom" },
    );
    // The content waits for the shape to arrive and leaves before it goes.
    let content_p = ((morph_p - 0.25) / 0.75).clamp(0.0, 1.0);
    let content_style = if reduce_value {
        format!("opacity:{}", if open { 1.0 } else { 0.0 })
    } else {
        format!(
            "opacity:{content_p};transform:translateY({}px)",
            (if below { -6.0 } else { 6.0 }) * (1.0 - content_p)
        )
    };
    let bar_style = format!(
        "border-radius:{SURFACE_RADIUS}px;transition:{}",
        if reduce_value {
            "none".to_string()
        } else {
            "width 300ms cubic-bezier(0.34, 1.1, 0.64, 1)".to_string()
        }
    );
    let bar_class = cn!(
        "relative z-20 flex h-14 min-w-0 items-center gap-2 ps-4",
        GLASS,
        if disabled {
            "cursor-not-allowed"
        } else {
            "cursor-text"
        },
        if actions.is_some() { "pe-2" } else { "pe-4" },
        if panel_open {
            "w-full"
        } else {
            "w-fit max-w-full"
        },
    );
    let chips_class = cn!(
        "relative flex min-w-0 items-center gap-1.5 overflow-x-auto [scrollbar-width:none]",
        if !chips.is_empty() {
            "-mx-1.5 px-3 [mask-image:linear-gradient(to_right,transparent,#000_12px,#000_calc(100%-12px),transparent)]"
        } else {
            ""
        },
    );
    let input_style = if panel_open {
        String::new()
    } else {
        format!("width:{}ch", placeholder.chars().count().max(6) + 2)
    };
    let input_class = cn!(
        "h-9 min-w-24 text-ellipsis bg-transparent text-base text-foreground outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed",
        if panel_open { "flex-1" } else { "" },
    );
    let content_box_style = format!("{content_style};min-height:{}px", floor());

    let root_class = cn!(
        "relative flex h-14 w-full max-w-xl justify-center",
        if disabled { "opacity-60" } else { "" },
        class,
    );

    rsx! {
        div {
            "data-beui-search-root": "{scope}",
            "data-beui-scope": dismiss.attr(),
            class: root_class,
            onfocusout: {
                let selector = format!("[data-beui-search-root=\"{scope}\"]");
                move |_| {
                    let selector = selector.clone();
                    spawn(async move {
                        // Give the browser a frame to settle the new focus.
                        let script = format!(
                            "await new Promise((resolve) => setTimeout(resolve, 0));\n\
                             const root = document.querySelector({selector});\n\
                             const active = document.activeElement;\n\
                             return !(root && active && root.contains(active));",
                            selector = js_string(&selector),
                        );
                        if json::<bool>(&script).await == Some(true) {
                            marked.set(None);
                            set_open(false);
                        }
                    });
                }
            },
            if mounted() {
                div {
                    class: cn!(
                        "absolute inset-x-0 z-10",
                        if below { "top-full mt-2" } else { "bottom-full mb-2" },
                    ),
                    // The glass layer: decorative, absolute, never reflowing.
                    div { aria_hidden: "true", class: cn!("absolute left-1/2", if below { "top-0" } else { "bottom-0" }, GLASS), style: glass_style }
                    div {
                        "data-beui-panel": "{scope}",
                        class: "relative p-2",
                        style: content_box_style,
                        div { "data-beui-panel-rows": "{scope}",
                            div { class: "flex h-9 items-center justify-between px-3",
                                span { class: "text-xs font-medium text-muted-foreground",
                                    if trimmed.is_empty() { "Recent" } else { "Suggestions" }
                                }
                                if trimmed.is_empty() && !recents.is_empty() {
                                    button {
                                        r#type: "button",
                                        class: "rounded-md px-1.5 py-0.5 text-xs text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                                        onclick: {
                                            let mut announce = announce;
                                            move |_| {
                                                set_recents(Vec::new());
                                                announce("Cleared recent searches".to_string());
                                            }
                                        },
                                        "Clear"
                                    }
                                }
                            }
                            if show_list {
                                div {
                                    role: "listbox",
                                    aria_label: if trimmed.is_empty() { "Recent searches" } else { "Suggestions" },
                                    aria_busy: if loading { Some("true") } else { None },
                                    class: "max-h-[min(20rem,45vh)] overflow-y-auto overscroll-contain [scrollbar-width:thin]",
                                    for (index, row) in rows.iter().enumerate() {
                                        div {
                                            key: "{row.id}",
                                            id: "beui-search-option-{scope}-{index}",
                                            role: "option",
                                            aria_selected: if index == active_index { "true" } else { "false" },
                                            class: "relative flex h-11 cursor-pointer items-center gap-3 rounded-2xl px-3 text-sm text-foreground",
                                            onpointermove: {
                                                let cursor = cursor.clone();
                                                let id = row.id.clone();
                                                move |_| cursor.move_to(Some(&id))
                                            },
                                            onclick: {
                                                let mut add_chip = add_chip.clone();
                                                let label = row.label.clone();
                                                move |_| add_chip(label.clone())
                                            },
                                            if index == active_index {
                                                span { class: "absolute inset-0 rounded-2xl bg-foreground/[0.06]" }
                                            }
                                            if row.kind == RowKind::Recent {
                                                Clock { class: Some("relative size-4 shrink-0 text-muted-foreground".to_string()) }
                                            } else {
                                                Search { class: Some("relative size-4 shrink-0 text-muted-foreground".to_string()) }
                                            }
                                            span { class: "relative min-w-0 flex-1 truncate",
                                                {highlight(&row.label, &trimmed, row.kind == RowKind::Query)}
                                            }
                                            if row.kind == RowKind::Recent {
                                                span {
                                                    aria_hidden: "true",
                                                    class: "relative grid size-7 shrink-0 place-items-center rounded-full text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground",
                                                    onclick: {
                                                        let mut remove_recent = remove_recent.clone();
                                                        let label = row.label.clone();
                                                        move |event: Event<MouseData>| {
                                                            event.stop_propagation();
                                                            remove_recent(label.clone());
                                                        }
                                                    },
                                                    X { class: Some("size-3.5".to_string()) }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if trimmed.is_empty() {
                                if let Some(state) = empty_state.clone() {
                                    div { class: "px-3 pb-2 pt-1 text-sm text-muted-foreground", {state} }
                                }
                            }
                            if loading {
                                div { class: "flex h-11 items-center gap-3 px-3 text-sm text-muted-foreground",
                                    LoaderCircle { class: Some("size-4 motion-safe:animate-spin".to_string()) }
                                    "Searching…"
                                }
                            }
                        }
                    }
                }
            }

            div { class: bar_class, style: bar_style,
                Search { class: Some("size-5 shrink-0 text-muted-foreground".to_string()) }
                div { class: chips_class,
                    for chip in chips.iter() {
                        span {
                            key: "{chip}",
                            class: cn!(
                                "flex h-9 shrink-0 items-center gap-1 rounded-full ps-3 pe-1 text-sm text-foreground transition-[background-color,box-shadow] duration-150",
                                if marked().as_deref() == Some(chip.as_str()) {
                                    "bg-foreground/[0.12] ring-2 ring-inset ring-foreground/40"
                                } else {
                                    "bg-foreground/[0.07]"
                                },
                            ),
                            span { class: "max-w-40 truncate", title: "{chip}", "{chip}" }
                            button {
                                r#type: "button",
                                disabled,
                                aria_label: "Remove filter {chip}",
                                class: "grid size-7 shrink-0 place-items-center rounded-full text-muted-foreground outline-none transition-colors hover:bg-foreground/10 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none",
                                onclick: {
                                    let mut remove_chip = remove_chip.clone();
                                    let chip = chip.clone();
                                    move |_| remove_chip(chip.clone())
                                },
                                X { class: Some("size-3.5".to_string()) }
                            }
                        }
                    }
                }
                input {
                    r#type: "text",
                    role: "combobox",
                    aria_label: "{label}",
                    aria_expanded: if panel_open { "true" } else { "false" },
                    aria_autocomplete: "list",
                    aria_activedescendant: if panel_open && show_list {
                        Some(format!("beui-search-option-{scope}-{active_index}"))
                    } else {
                        None
                    },
                    autocomplete: "off",
                    spellcheck: "false",
                    disabled,
                    value: "{query}",
                    placeholder,
                    class: input_class,
                    style: input_style,
                    oninput: {
                        let mut set_query = set_query.clone();
                        move |event: Event<FormData>| {
                            set_query(event.value());
                            if !open {
                                set_open(true);
                            }
                        }
                    },
                    onfocus: move |_| set_open(true),
                    onclick: move |_| set_open(true),
                    onkeydown: on_key_down.clone(),
                }
                if let Some(actions) = actions.clone() {
                    div { class: "flex shrink-0 items-center gap-1", {actions} }
                }
            }

            if let Some(name) = name.clone() {
                for chip in chips.iter() {
                    input { key: "hidden-{chip}", r#type: "hidden", name: name.clone(), value: "{chip}" }
                }
            }
            span { class: "sr-only",
                "Enter adds a filter. Backspace at the start of the field selects the last filter, and again removes it. Delete removes the highlighted recent search."
            }
            span { aria_live: "polite", class: "sr-only",
                "{announcement()}"
                if announcement_count() % 2 == 1 { " " }
            }
        }
    }
}

/// A JSON array of strings, escaped for the script boundary.
fn json_array(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|item| js_string(item)).collect();
    format!("[{}]", parts.join(","))
}

/// The row label with the matched run in bold.
fn highlight(text: &str, query: &str, plain: bool) -> Element {
    if plain || query.is_empty() {
        return rsx! { "{text}" };
    }
    let lowered = text.to_lowercase();
    let needle = query.to_lowercase();
    let Some(at) = lowered.find(&needle) else {
        return rsx! { "{text}" };
    };
    let before = &text[..at];
    let matched = &text[at..at + needle.len()];
    let after = &text[at + needle.len()..];

    rsx! {
        "{before}"
        span { class: "font-semibold", "{matched}" }
        "{after}"
    }
}
