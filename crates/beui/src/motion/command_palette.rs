//! `components/motion/command-palette.tsx` — Cmd-K search over a command list.
//!
//! The palette is portalled and mounted only while open; the backdrop and the
//! panel layer are two keyed presence children, so both release interaction in
//! the commit the exit starts. The list is grouped, and every consumer of "which
//! row" (the highlight, the ids, Enter, the scroll) reads one flattened `rows`
//! array so grouping cannot make them drift.
//!
//! Deviations from the source:
//!
//! - `searchCommands` is ported without NFKD mark stripping (no
//!   `unicode-normalization` in the workspace), so accented labels fold less;
//! - items carry an optional `badge` string, not a node, and no icon;
//! - the global shortcut is one JS listener with a `dioxus.send` bridge instead
//!   of a `window` event listener bound in Rust.

use dioxus::prelude::*;
use dioxus_motion::prelude::AnimationConfig;

use crate::icons::SearchIcon;
use crate::motion::PortalHost;
use beui_core::ease::tween_ease_out;
use beui_core::hooks::{use_on_open, use_row_cursor, RowStep};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};
use beui_core::{cn, use_reduced_motion};

/// One command.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandItem {
    pub id: String,
    pub label: String,
    pub group: Option<String>,
    pub hint: Option<String>,
    pub keywords: Vec<String>,
    pub badge: Option<String>,
    pub on_select: EventHandler<()>,
}

impl CommandItem {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        on_select: EventHandler<()>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            group: None,
            hint: None,
            keywords: Vec::new(),
            badge: None,
            on_select,
        }
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords.push(keywords.into());
        self
    }
}

/// Fold camelCase, lowercase, and collapse everything that is not a letter or
/// number to single spaces.
fn normalize(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut spaced = String::new();
    for (index, ch) in chars.iter().enumerate() {
        if index > 0 && ch.is_uppercase() && chars[index - 1].is_lowercase() {
            spaced.push(' ');
        }
        spaced.push(*ch);
    }
    let lowered = spaced.to_lowercase();
    let mut result = String::new();
    let mut previous_was_space = true;
    for ch in lowered.chars() {
        if ch.is_alphanumeric() {
            result.push(ch);
            previous_was_space = false;
        } else if !previous_was_space {
            result.push(' ');
            previous_was_space = true;
        }
    }
    result.trim().to_string()
}

fn token_score(token: &str, field: &str) -> i64 {
    let words: Vec<&str> = field.split(' ').collect();
    if words.contains(&token) {
        return 100;
    }
    if words.iter().any(|word| word.starts_with(token)) {
        return 80;
    }
    if field.contains(token) {
        return 60;
    }
    let token_chars: Vec<char> = token.chars().collect();
    if token_chars.len() < 3 {
        return 0;
    }
    for word in &words {
        let word_chars: Vec<char> = word.chars().collect();
        if word_chars.len() as i64 - token_chars.len() as i64 > 2 {
            continue;
        }
        let mut matched = 0usize;
        for ch in &word_chars {
            if matched < token_chars.len() && *ch == token_chars[matched] {
                matched += 1;
            }
        }
        if matched == token_chars.len() {
            return 20;
        }
    }
    0
}

/// `searchCommands` — every query word must match some field, direct name
/// matches first.
pub fn search_commands(items: &[CommandItem], query: &str) -> Vec<CommandItem> {
    let normalized = normalize(query);
    if normalized.is_empty() {
        return items.to_vec();
    }
    let tokens: Vec<&str> = normalized.split(' ').filter(|t| !t.is_empty()).collect();
    let mut scored: Vec<(i64, usize, CommandItem)> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let label = normalize(&item.label);
        let mut fields = vec![label.clone()];
        fields.push(normalize(item.group.as_deref().unwrap_or("")));
        for keyword in &item.keywords {
            fields.push(normalize(keyword));
        }
        let mut score = if label == normalized {
            10000
        } else if label.starts_with(&normalized) {
            2000
        } else {
            0
        };
        let mut matched_all = true;
        for token in &tokens {
            let best = fields
                .iter()
                .enumerate()
                .map(|(field_index, field)| {
                    let value = token_score(token, field);
                    if value > 0 {
                        value + if field_index == 0 { 40 } else { 0 }
                    } else {
                        0
                    }
                })
                .max()
                .unwrap_or(0);
            if best == 0 {
                matched_all = false;
                break;
            }
            score += best;
        }
        if matched_all {
            scored.push((score, index, item.clone()));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, _, item)| item).collect()
}

use beui_core::flip::{use_flip, FlipOptions};

fn backdrop_config() -> PresenceConfig {
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0 },
        motion_style! { opacity: 1.0 },
        motion_style! { opacity: 0.0 },
        tween_ease_out(180),
        tween_ease_out(120),
    )
}

fn panel_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(100),
            tween_ease_out(120),
        );
    }
    // PANEL_SPRING: explicit physics, not a visual form.
    let spring = dioxus_motion::prelude::Spring {
        stiffness: 560.0,
        damping: 40.0,
        mass: 0.5,
    };
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, y: -8.0, scale: 0.97 },
        motion_style! { opacity: 1.0, y: 0.0, scale: 1.0 },
        motion_style! { opacity: 0.0, y: -8.0, scale: 0.97 },
        AnimationConfig::spring(spring),
        tween_ease_out(120),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Backdrop,
    Panel,
}

impl Layer {
    const fn key(self) -> &'static str {
        match self {
            Self::Backdrop => "backdrop",
            Self::Panel => "panel-layer",
        }
    }
}

/// `CommandPalette`.
#[component]
pub fn CommandPalette(
    items: Vec<CommandItem>,
    /// Opens with Cmd/Ctrl + this key.
    #[props(default = "k")]
    shortcut: &'static str,
    #[props(default = "Type a command or search…")] placeholder: &'static str,
    #[props(default = "No results found.")] empty_message: &'static str,
    #[props(default = None)] open: Option<bool>,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
) -> Element {
    let reduce = use_reduced_motion()();
    let mut internal_open = use_signal(|| false);
    let controlled = open.is_some();
    let fallback = internal_open();
    let current_open = open.unwrap_or(fallback);
    let open_signal = use_signal(|| current_open);
    let mut open_sync = open_signal;
    if *open_sync.peek() != current_open {
        open_sync.set(current_open);
    }
    let query = use_signal(String::new);

    let set_open = EventHandler::new(move |next: bool| {
        if !controlled {
            internal_open.set(next);
        }
        if let Some(callback) = on_open_change {
            callback.call(next);
        }
    });
    let toggle = EventHandler::new(move |_| set_open.call(!*open_signal.peek()));
    let uid = use_hook(platform::next_scope_id);

    // Global shortcut + Escape.
    let toggle_signal = toggle;
    use_effect(move || {
        let script = format!(
            "{}\nwindow.__beuiCommandStart({uid}, {});",
            include_str!("../js/command.js"),
            platform::js_string(shortcut),
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                if message == "toggle" {
                    toggle_signal.call(());
                } else if message == "escape" {
                    // `command-palette.tsx` closes on Escape only while open — the
                    // listener fires on every Escape in the document, so opening
                    // it here would turn Escape into a way to summon the palette.
                    set_open.call(false);
                }
            }
        });
    });
    use_drop(move || {
        platform::fire(&format!(
            "window.__beuiCommandStop && window.__beuiCommandStop({uid})"
        ));
    });

    // Scroll lock while open.
    use_effect(move || {
        if open_signal() {
            platform::fire(
                "document.documentElement.style.overflow='hidden';document.body.style.overflow='hidden';",
            );
        } else {
            platform::fire(
                "document.documentElement.style.overflow='';document.body.style.overflow='';",
            );
        }
    });

    // Search, then group; `rows` is the one flattened order everyone reads.
    let filtered = search_commands(&items, &query());
    let mut rows: Vec<CommandItem> = Vec::new();
    let mut grouped: Vec<(String, Vec<(usize, CommandItem)>)> = Vec::new();
    for item in filtered {
        let group = item.group.clone().unwrap_or_else(|| "Results".to_string());
        let index = rows.len();
        rows.push(item.clone());
        let entry = (index, item);
        if let Some((_, list)) = grouped.iter_mut().find(|(name, _)| *name == group) {
            list.push(entry);
        } else {
            grouped.push((group, vec![entry]));
        }
    }
    let row_ids: Vec<String> = rows.iter().map(|item| item.id.clone()).collect();
    let cursor = use_row_cursor(row_ids, &query());
    let active = if rows.is_empty() {
        None
    } else {
        Some(cursor.active_index())
    };
    let is_open = open_signal();

    // Reset query + cursor on open, and focus the field.
    let cursor_for_open = cursor.clone();
    let mut query_for_open = query;
    use_on_open(is_open, move || {
        query_for_open.set(String::new());
        cursor_for_open.move_to(None);
    });
    let input_id = format!("beui-command-{uid}-input");
    let input_id_effect = input_id.clone();
    let open_for_focus = open_signal;
    use_effect(move || {
        if open_for_focus() {
            let id = platform::js_string(&input_id_effect);
            platform::fire(&format!(
                "requestAnimationFrame(() => document.getElementById({id})?.focus())"
            ));
        }
    });

    // Keep the active row visible.
    let cursor_for_scroll = cursor.clone();
    use_effect(move || {
        let index = cursor_for_scroll.active_index();
        platform::fire(&format!(
            "document.querySelector('[data-index=\"{index}\"]')?.scrollIntoView({{ block: 'nearest' }})"
        ));
    });

    let layers: Vec<Layer> = if is_open {
        vec![Layer::Backdrop, Layer::Panel]
    } else {
        Vec::new()
    };
    let rows_for_panel = rows.clone();
    let on_move = EventHandler::new({
        let cursor = cursor.clone();
        move |step: RowStep| cursor.move_active(step)
    });
    let on_hover = EventHandler::new({
        let cursor = cursor.clone();
        move |id: String| cursor.move_to(Some(&id))
    });

    rsx! {
        PortalHost {
            AnimatePresence {
                initial: false,
                for layer in layers {
                    PaletteLayer {
                        key: "{layer.key()}",
                        layer,
                        uid,
                        reduce,
                        input_id: input_id.clone(),
                        placeholder,
                        empty_message,
                        query,
                        grouped: grouped.clone(),
                        rows: rows_for_panel.clone(),
                        active,
                        on_move,
                        on_hover,
                        on_close: set_open,
                        shortcut,
                    }
                }
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn PaletteLayer(
    layer: Layer,
    uid: u32,
    reduce: bool,
    input_id: String,
    placeholder: &'static str,
    empty_message: &'static str,
    query: Signal<String>,
    grouped: Vec<(String, Vec<(usize, CommandItem)>)>,
    rows: Vec<CommandItem>,
    active: Option<usize>,
    on_move: EventHandler<RowStep>,
    on_hover: EventHandler<String>,
    on_close: EventHandler<bool>,
    shortcut: &'static str,
) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(match layer {
        Layer::Backdrop => backdrop_config(),
        Layer::Panel => panel_config(reduce),
    })
    .expect("static presence config");
    let css = presence_css(&handle);

    match layer {
        Layer::Backdrop => rsx! {
            button {
                r#type: "button",
                "aria-label": "Close command palette",
                inert: gate.inert_attr(),
                style: gate.style_with(&css),
                class: "pointer-events-auto fixed inset-0 z-[100] bg-background/5 [backdrop-filter:blur(12px)_saturate(140%)] [-webkit-backdrop-filter:blur(12px)_saturate(140%)]",
                onclick: move |_| on_close.call(false),
            }
        },
        Layer::Panel => {
            let rows_for_keys = rows.clone();
            let on_move = on_move;
            rsx! {
                div {
                    inert: gate.inert_attr(),
                    class: "pointer-events-none fixed inset-x-4 bottom-4 top-[18vh] z-[100] flex items-start justify-center",
                    div {
                        role: "dialog",
                        "aria-modal": "true",
                        "aria-label": "Command palette",
                        inert: gate.inert_attr(),
                        style: gate.style_with(&format!("{css};will-change:transform")),
                        class: "pointer-events-auto w-full max-w-xl overflow-hidden rounded-2xl border border-border bg-background shadow-2xl",
                        onkeydown: move |event: KeyboardEvent| match event.key() {
                            Key::ArrowDown => {
                                event.prevent_default();
                                on_move.call(RowStep::Next);
                            }
                            Key::ArrowUp => {
                                event.prevent_default();
                                on_move.call(RowStep::Previous);
                            }
                            Key::Enter => {
                                event.prevent_default();
                                if let Some(index) = active {
                                    if let Some(item) = rows_for_keys.get(index) {
                                        item.on_select.call(());
                                        on_close.call(false);
                                    }
                                }
                            }
                            _ => {}
                        },
                        div { class: "flex items-center gap-3 border-b border-border px-4",
                            SearchIcon { class: "h-4 w-4 text-muted-foreground" }
                            input {
                                id: "{input_id}",
                                value: "{query}",
                                oninput: move |event| {
                                    let mut q = query;
                                    q.set(event.value());
                                },
                                placeholder,
                                role: "combobox",
                                "aria-expanded": "true",
                                "aria-controls": "beui-command-{uid}-list",
                                "aria-activedescendant": active.map(|index| format!("beui-command-{uid}-opt-{index}")),
                                "aria-autocomplete": "list",
                                class: "h-12 flex-1 bg-transparent text-sm text-foreground placeholder:text-muted-foreground outline-none",
                            }
                            kbd { class: "hidden rounded border border-border bg-background px-1.5 py-0.5 text-[10px] text-muted-foreground sm:inline-block",
                                "{shortcut.to_uppercase()} / ESC"
                            }
                        }
                        div {
                            id: "beui-command-{uid}-list",
                            role: "listbox",
                            "aria-label": "Commands",
                            class: "max-h-[60vh] overflow-y-auto overscroll-contain p-2 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden",
                            if rows.is_empty() {
                                div { class: "p-8 text-center text-sm text-muted-foreground", "{empty_message}" }
                            } else {
                                for (group, list) in grouped {
                                    div { key: "{group}", class: "mb-1 last:mb-0",
                                        div { "aria-hidden": "true", class: "px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground",
                                            "{group}"
                                        }
                                        for (index, item) in list {
                                            PaletteRow {
                                                key: "{item.id}",
                                                index,
                                                active,
                                                uid,
                                                on_hover,
                                                on_close,
                                                item,
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PaletteRow(
    item: CommandItem,
    index: usize,
    active: Option<usize>,
    uid: u32,
    on_hover: EventHandler<String>,
    on_close: EventHandler<bool>,
) -> Element {
    let is_active = active == Some(index);
    rsx! {
        button {
            r#type: "button",
            id: "beui-command-{uid}-opt-{index}",
            role: "option",
            "aria-selected": is_active,
            "data-index": "{index}",
            class: cn!(
                "relative isolate flex w-full items-center gap-3 rounded-md px-2 py-2 text-left text-sm transition-colors",
                if is_active { "text-foreground" } else { "text-muted-foreground" },
            ),
            onmouseenter: {
                let id = item.id.clone();
                move |_| on_hover.call(id.clone())
            },
            onclick: {
                let on_select = item.on_select;
                move |_| {
                    on_select.call(());
                    on_close.call(false);
                }
            },
            if is_active {
                RowHighlight { active, uid }
            }
            span { class: "relative z-10 flex-1 truncate", "{item.label}" }
            if let Some(badge) = &item.badge {
                span { class: "relative z-10 shrink-0 rounded-full bg-muted px-2 py-0.5 text-[10px]", "{badge}" }
            }
            if let Some(hint) = &item.hint {
                kbd { class: "relative z-10 rounded border border-border bg-background px-1.5 py-0.5 text-[10px] text-muted-foreground", "{hint}" }
            }
        }
    }
}

#[component]
fn RowHighlight(active: Option<usize>, uid: u32) -> Element {
    let flip = use_flip(
        active,
        FlipOptions::default()
            .layout_id(format!("beui-command-{uid}-active"))
            .transition(AnimationConfig::spring(dioxus_motion::prelude::Spring {
                stiffness: 480.0,
                damping: 38.0,
                mass: 1.0,
            }))
            .border_radius(6.0),
    );
    rsx! {
        span {
            "aria-hidden": "true",
            "data-beui-flip": flip.scope(),
            style: flip.style_with(""),
            class: "absolute inset-0 z-0 rounded-md bg-muted",
        }
    }
}
