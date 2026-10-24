//! `components/motion/multi-select/` — a searchable listbox that keeps a set.
//!
//! The same panel, measurement and render-time active-option resolution as
//! `combobox`, but the value is a list: selecting toggles, the trigger shows
//! removable chips, and Backspace on an empty query removes the last one.
//!
//! Deviations from the source:
//!
//! - the chip's entrance uses the presence style for opacity and the exit clip,
//!   while its *layout* position is a `flip` — the enter transform the source
//!   gives it (translateY + scale) is dropped because presence and flip both
//!   write `transform` on the same element;
//! - `duration`+`bounce` springs go through [`spring_from_visual_duration`].

use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus_motion::prelude::{
    use_motion, AnimationConfig, AnimationManager, Duration, Time, TimeProvider,
};

use crate::icons::{CheckIcon, ChevronsUpDownIcon, SearchIcon, XIcon};
use crate::motion::{spring_from_visual_duration, MoveDir, PortalHost};
use beui_core::ease::{tween_ease_out, EASE_OUT_CSS, SPRING_LAYOUT};
use beui_core::flip::{use_flip, FlipMode, FlipOptions};
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceMode,
};
use beui_core::{cn, use_reduced_motion};

const VIEWPORT_PADDING: f64 = 8.0;

/// `(value, query, keywords) -> visible`.
pub type MultiSelectFilter = fn(&str, &str, &[String]) -> bool;

/// The default filter: a case-insensitive subsequence match over value+keywords.
pub fn default_multi_select_filter(value: &str, query: &str, keywords: &[String]) -> bool {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    let haystack = format!("{} {}", value, keywords.join(" ")).to_lowercase();
    let needle_chars: Vec<char> = needle.chars().collect();
    let mut query_index = 0usize;
    for character in haystack.chars() {
        if query_index < needle_chars.len() && character == needle_chars[query_index] {
            query_index += 1;
        }
        if query_index == needle_chars.len() {
            return true;
        }
    }
    false
}

#[derive(Clone, Copy)]
pub struct MultiSelectFilterFn(pub fn(&str, &str, &[String]) -> bool);

impl PartialEq for MultiSelectFilterFn {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RegisteredItem {
    value: String,
    label: String,
    keywords: Vec<String>,
    disabled: bool,
    group_id: Option<String>,
    id: String,
}

#[derive(Clone, PartialEq, Eq)]
struct MultiSelectGroupId(Option<String>);

#[derive(Clone, Debug, Default, PartialEq)]
struct MultiSelectDerived {
    visible: HashSet<String>,
    visible_groups: HashSet<String>,
    count: usize,
    enabled: Vec<String>,
    active_value: Option<String>,
    active_item_id: Option<String>,
}

#[derive(Clone)]
struct MultiSelectContext {
    open: Signal<bool>,
    set_open: EventHandler<(bool, bool)>,
    values: Signal<Vec<String>>,
    toggle: EventHandler<String>,
    remove: EventHandler<String>,
    query: Signal<String>,
    set_query: EventHandler<String>,
    set_active_value: EventHandler<Option<String>>,
    move_active: EventHandler<MoveDir>,
    toggle_active: EventHandler<()>,
    register_item: EventHandler<RegisteredItem>,
    unregister_item: EventHandler<String>,
    items: Signal<Vec<RegisteredItem>>,
    derived: Signal<MultiSelectDerived>,
    trigger_id: String,
    list_id: String,
    input_id: String,
    panel_id: String,
    measure_id: String,
    active_layout_id: String,
    disabled: bool,
    reduce: bool,
}

fn use_multi_select() -> MultiSelectContext {
    use_context::<MultiSelectContext>()
}

/// `MultiSelect`.
#[component]
pub fn MultiSelect(
    #[props(default = None)] value: Option<Vec<String>>,
    #[props(default = None)] default_value: Option<Vec<String>>,
    #[props(default = None)] on_value_change: Option<EventHandler<Vec<String>>>,
    #[props(default = None)] open: Option<bool>,
    #[props(default = false)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = None)] query: Option<String>,
    #[props(default = None)] default_query: Option<String>,
    #[props(default = None)] on_query_change: Option<EventHandler<String>>,
    #[props(default = MultiSelectFilterFn(default_multi_select_filter))]
    filter: MultiSelectFilterFn,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let uid = use_hook(platform::next_scope_id);
    let input_id = format!("beui-multi-select-{uid}-input");
    let trigger_id = format!("beui-multi-select-{uid}-trigger");
    let list_id = format!("beui-multi-select-{uid}-list");
    let panel_id = format!("beui-multi-select-{uid}-panel");
    let measure_id = format!("beui-multi-select-{uid}-measure");
    let active_layout_id = format!("beui-multi-select-{uid}-active");

    let mut internal_value = use_signal(|| default_value.clone().unwrap_or_default());
    let mut internal_open = use_signal(|| default_open);
    let mut internal_query = use_signal(|| default_query.clone().unwrap_or_default());
    let value_controlled = value.is_some();
    let open_controlled = open.is_some();
    let query_controlled = query.is_some();
    let fallback_value = internal_value();
    let current_values = value.clone().unwrap_or(fallback_value);
    let fallback_open = internal_open();
    let current_open = open.unwrap_or(fallback_open);
    let fallback_query = internal_query();
    let current_query = query.clone().unwrap_or(fallback_query);

    let values_signal = use_signal(|| current_values.clone());
    let mut values_sync = values_signal;
    if *values_sync.peek() != current_values {
        values_sync.set(current_values.clone());
    }
    let open_signal = use_signal(|| current_open);
    let mut open_sync = open_signal;
    if *open_sync.peek() != current_open {
        open_sync.set(current_open);
    }
    let query_signal = use_signal(|| current_query.clone());
    let mut query_sync = query_signal;
    if *query_sync.peek() != current_query {
        query_sync.set(current_query.clone());
    }

    let list_query = use_signal(|| current_query.clone());
    let open_query = use_signal(|| current_query.clone());
    if current_open && *open_query.peek() != current_query {
        let mut slot = open_query;
        slot.set(current_query.clone());
    }
    {
        let snapshot = (open_query)();
        let target = if current_open {
            current_query.clone()
        } else {
            snapshot
        };
        let mut slot = list_query;
        if *slot.peek() != target {
            slot.set(target);
        }
    }

    let opened = use_signal(|| current_open);
    let items = use_signal(Vec::<RegisteredItem>::new);
    let derived = use_signal(MultiSelectDerived::default);
    let active_cursor = use_signal(|| None::<(String, String)>);

    let input_id_for_restore = input_id.clone();
    let update_open = EventHandler::new(move |(next, restore_focus): (bool, bool)| {
        if disabled && next {
            return;
        }
        if !open_controlled {
            internal_open.set(next);
        }
        if let Some(callback) = on_open_change {
            callback.call(next);
        }
        if !next {
            if !query_controlled {
                internal_query.set(String::new());
            }
            if let Some(callback) = on_query_change {
                callback.call(String::new());
            }
        }
        if restore_focus {
            platform::fire(&format!(
                "requestAnimationFrame(() => document.getElementById({})?.focus({{ preventScroll: true }}))",
                platform::js_string(&input_id_for_restore)
            ));
        }
    });

    let update_query = EventHandler::new(move |next: String| {
        if !query_controlled {
            internal_query.set(next.clone());
        }
        if let Some(callback) = on_query_change {
            callback.call(next);
        }
    });

    let commit_value = EventHandler::new(move |next: Vec<String>| {
        if !value_controlled {
            internal_value.set(next.clone());
        }
        if let Some(callback) = on_value_change {
            callback.call(next);
        }
    });

    let input_id_for_focus = input_id.clone();
    let toggle = EventHandler::new(move |next: String| {
        if (items)()
            .iter()
            .any(|item| item.value == next && item.disabled)
        {
            return;
        }
        let current = (values_signal)();
        let mut next_values = current.clone();
        if let Some(index) = next_values.iter().position(|value| *value == next) {
            next_values.remove(index);
        } else {
            next_values.push(next);
        }
        commit_value.call(next_values);
        update_query.call(String::new());
        platform::fire(&format!(
            "requestAnimationFrame(() => document.getElementById({})?.focus({{ preventScroll: true }}))",
            platform::js_string(&input_id_for_focus)
        ));
    });

    let remove = EventHandler::new(move |next: String| {
        let current = (values_signal)();
        if !current.contains(&next) {
            return;
        }
        let next_values: Vec<String> = current.into_iter().filter(|value| *value != next).collect();
        commit_value.call(next_values);
    });

    let register_item = EventHandler::new(move |item: RegisteredItem| {
        let mut slot = items;
        let mut snapshot = slot.write();
        if let Some(existing) = snapshot.iter_mut().find(|entry| entry.value == item.value) {
            *existing = item;
        } else {
            snapshot.push(item);
        }
    });
    let unregister_item = EventHandler::new(move |value: String| {
        let mut slot = items;
        slot.write().retain(|item| item.value != value);
    });

    let set_active_value = EventHandler::new(move |next: Option<String>| {
        let query = (list_query)();
        let mut cursor = active_cursor;
        cursor.set(next.map(|value| (value, query)));
    });

    let move_active = EventHandler::new(move |direction: MoveDir| {
        if !(open_signal)() {
            return;
        }
        let snapshot = (derived)();
        let rows = snapshot.enabled;
        if rows.is_empty() {
            let mut cursor = active_cursor;
            cursor.set(None);
            return;
        }
        let at = snapshot
            .active_value
            .and_then(|value| rows.iter().position(|row| *row == value))
            .map(|index| index as isize)
            .unwrap_or(-1);
        let len = rows.len() as isize;
        let index = match direction {
            MoveDir::First => 0,
            MoveDir::Last => len - 1,
            MoveDir::Next => (at + 1).rem_euclid(len),
            MoveDir::Previous => (at - 1).rem_euclid(len),
        };
        let next = rows[index as usize].clone();
        let query = (list_query)();
        let mut cursor = active_cursor;
        cursor.set(Some((next, query)));
    });

    let toggle_active = EventHandler::new(move |_| {
        if let Some(active) = (derived)().active_value {
            toggle.call(active);
        }
    });

    use_context_provider(|| MultiSelectContext {
        open: open_signal,
        set_open: update_open,
        values: values_signal,
        toggle,
        remove,
        query: query_signal,
        set_query: update_query,
        set_active_value,
        move_active,
        toggle_active,
        register_item,
        unregister_item,
        items,
        derived,
        trigger_id: trigger_id.clone(),
        list_id: list_id.clone(),
        input_id: input_id.clone(),
        panel_id,
        measure_id,
        active_layout_id,
        disabled,
        reduce,
    });

    // --- render-time resolution --------------------------------------------
    let items_snapshot = (items)();
    let list_query_value = (list_query)();
    let visible: Vec<RegisteredItem> = items_snapshot
        .iter()
        .filter(|item| {
            let mut keywords = vec![item.label.clone()];
            keywords.extend(item.keywords.iter().cloned());
            (filter.0)(&item.value, &list_query_value, &keywords)
        })
        .cloned()
        .collect();
    let visible_set: HashSet<String> = visible.iter().map(|item| item.value.clone()).collect();
    let visible_groups: HashSet<String> = visible
        .iter()
        .filter_map(|item| item.group_id.clone())
        .collect();
    let enabled: Vec<String> = visible
        .iter()
        .filter(|item| !item.disabled)
        .map(|item| item.value.clone())
        .collect();

    let cursor_value = active_cursor.peek().clone();
    let live = match &cursor_value {
        Some((value, cursor_query)) if *cursor_query == list_query_value => {
            enabled.contains(value).then(|| value.clone())
        }
        _ => None,
    };
    if cursor_value.is_some() && live.is_none() {
        let mut cursor = active_cursor;
        cursor.set(None);
    }
    let derived_active = live.or_else(|| {
        let selected = (values_signal)().into_iter().next();
        if let Some(value) = &selected {
            if enabled.contains(value) {
                return Some(value.clone());
            }
        }
        enabled.first().cloned()
    });
    if current_open && !(opened)() {
        let mut slot = opened;
        slot.set(true);
    }
    let active_value = if (opened)() { derived_active } else { None };
    let active_item_id = active_value.as_ref().and_then(|value| {
        items_snapshot
            .iter()
            .find(|item| item.value == *value)
            .map(|item| item.id.clone())
    });
    let next_derived = MultiSelectDerived {
        visible: visible_set,
        visible_groups,
        count: visible.len(),
        enabled,
        active_value,
        active_item_id,
    };
    let mut derived_signal = derived;
    if *derived_signal.peek() != next_derived {
        derived_signal.set(next_derived);
    }

    let close = EventHandler::new(move |_| update_open.call((false, false)));
    let _scope = use_dismiss(
        open_signal,
        close,
        DismissOptions::default().ignore(format!("#beui-multi-select-{uid}-panel")),
    );

    rsx! {
        div { class: cn!("relative w-full", class.as_deref().unwrap_or("")), {children} }
    }
}

/// `MultiSelectTrigger`.
#[component]
pub fn MultiSelectTrigger(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_multi_select();
    let open = (context.open)();
    rsx! {
        div {
            id: "{context.trigger_id}",
            "data-state": if open { "open" } else { "closed" },
            class: cn!(
                "relative z-20 flex min-h-11 w-full min-w-52 cursor-text items-center gap-2 rounded-xl border border-border bg-transparent px-2.5 py-1.5 text-sm text-foreground transition-[border-color] hover:border-(--color-border-strong)",
                "focus-within:ring-2 focus-within:ring-foreground/20",
                if context.disabled { "pointer-events-none opacity-50" } else { "" },
                class.as_deref().unwrap_or(""),
            ),
            onpointerdown: move |event: Event<PointerData>| {
                if context.disabled {
                    return;
                }
                event.prevent_default();
                platform::fire(&format!(
                    "document.getElementById({})?.focus({{ preventScroll: true }})",
                    platform::js_string(&context.input_id)
                ));
                context.set_open.call((true, false));
            },
            div { class: "flex min-w-0 flex-1 flex-wrap items-center gap-1.5", {children} }
            ChevronsUpDownIcon { class: "size-4 shrink-0 text-muted-foreground" }
        }
    }
}

/// `MultiSelectValue`.
#[component]
pub fn MultiSelectValue(
    #[props(default = "Select options")] placeholder: &'static str,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let context = use_multi_select();
    let values = (context.values)();
    let open = (context.open)();
    let show_placeholder = values.is_empty() && !open;
    let items_snapshot = (context.items)();
    let chips: Vec<(String, String)> = values
        .iter()
        .map(|value| {
            let label = items_snapshot
                .iter()
                .find(|item| item.value == *value)
                .map(|item| item.label.clone())
                .unwrap_or_else(|| value.clone());
            (value.clone(), label)
        })
        .collect();
    let reduce = context.reduce;
    let chip_context = context.clone();

    rsx! {
        div { class: cn!("contents", class.as_deref().unwrap_or("")),
            if show_placeholder {
                span { class: "text-muted-foreground", "{placeholder}" }
            }
            AnimatePresence {
                initial: false,
                mode: PresenceMode::PopLayout,
                for (value, label) in chips {
                    MultiSelectChip {
                        key: "multi-select-value-{value}",
                        value,
                        label,
                        reduce,
                        on_remove: chip_context.remove,
                        disabled: chip_context.disabled,
                        input_id: chip_context.input_id.clone(),
                    }
                }
            }
        }
    }
}

#[component]
fn MultiSelectChip(
    value: String,
    label: String,
    reduce: bool,
    disabled: bool,
    input_id: String,
    on_remove: EventHandler<String>,
) -> Element {
    let context = use_multi_select();
    let gate = use_gate();
    let handle = use_presence_style(chip_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);
    let flip = use_flip(
        (context.values)(),
        FlipOptions::default()
            .mode(FlipMode::Position)
            .transition(AnimationConfig::spring(SPRING_LAYOUT)),
    );
    let style = format!("{};{}", gate.style_with(&css), flip.style_with(""));

    rsx! {
        span {
            inert: gate.inert_attr(),
            style,
            class: "inline-flex h-7 max-w-full items-center gap-1 rounded-lg bg-muted px-2 text-xs font-medium text-foreground",
            span { class: "truncate", "{label}" }
            button {
                r#type: "button",
                "data-multi-select-remove": "",
                "aria-label": "Remove {label}",
                disabled,
                class: "-mr-1 grid size-5 shrink-0 place-items-center rounded-md text-muted-foreground outline-none transition-colors hover:bg-foreground/10 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                onpointerdown: move |event: Event<PointerData>| event.stop_propagation(),
                onclick: {
                    let remove = on_remove;
                    let input_id = input_id.clone();
                    move |event: MouseEvent| {
                        event.stop_propagation();
                        remove.call(value.clone());
                        platform::fire(&format!(
                            "document.getElementById({})?.focus({{ preventScroll: true }})",
                            platform::js_string(&input_id)
                        ));
                    }
                },
                XIcon { class: "size-3" }
            }
        }
    }
}

fn chip_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(150),
            tween_ease_out(150),
        );
    }
    let mut enter = motion_style! { opacity: 0.0 };
    enter.add_css_property("clip-path", "inset(0 0 0 0% round 0.5rem)");
    let mut center = motion_style! { opacity: 1.0 };
    center.add_css_property("clip-path", "inset(0 0 0 0% round 0.5rem)");
    let mut exit = motion_style! { opacity: 1.0 };
    exit.add_css_property("clip-path", "inset(0 0 0 100% round 0.5rem)");
    PresenceConfig::with_transitions(
        enter,
        center,
        exit,
        tween_ease_out(180),
        tween_ease_out(160),
    )
}

/// `MultiSelectInput`.
#[component]
pub fn MultiSelectInput(
    #[props(default = "Search options")] aria_label: &'static str,
    #[props(default = "Search…")] placeholder: &'static str,
    #[props(default = false)] show_icon: bool,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let context = use_multi_select();
    let open = (context.open)();
    let query = (context.query)();
    let values = (context.values)();
    let active_item_id = (context.derived)().active_item_id;
    let display_placeholder = if values.is_empty() { placeholder } else { "" };

    rsx! {
        div { class: "flex min-w-20 flex-1 items-center gap-1.5",
            if show_icon {
                SearchIcon { class: "size-3.5 text-muted-foreground" }
            }
            input {
                id: "{context.input_id}",
                role: "combobox",
                "aria-label": aria_label,
                "aria-autocomplete": "list",
                "aria-expanded": open,
                "aria-controls": "{context.list_id}",
                "aria-activedescendant": if open { active_item_id } else { None },
                autocomplete: "off",
                disabled: context.disabled,
                value: "{query}",
                placeholder: display_placeholder,
                class: cn!(
                    "h-7 min-w-12 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed",
                    class.as_deref().unwrap_or(""),
                ),
                onfocus: move |_| context.set_open.call((true, false)),
                onclick: move |_| context.set_open.call((true, false)),
                oninput: move |event| {
                    context.set_open.call((true, false));
                    context.set_query.call(event.value());
                },
                onkeydown: move |event: KeyboardEvent| {
                    match event.key() {
                        Key::Backspace => {
                            if (context.query)().is_empty() {
                                if let Some(last) = (context.values)().last().cloned() {
                                    event.prevent_default();
                                    context.remove.call(last);
                                }
                            }
                        }
                        Key::ArrowDown | Key::ArrowUp => {
                            event.prevent_default();
                            if !(context.open)() {
                                context.set_open.call((true, false));
                                return;
                            }
                            context.move_active.call(if event.key() == Key::ArrowDown {
                                MoveDir::Next
                            } else {
                                MoveDir::Previous
                            });
                        }
                        Key::Home => {
                            if (context.open)() {
                                event.prevent_default();
                                context.move_active.call(MoveDir::First);
                            }
                        }
                        Key::End => {
                            if (context.open)() {
                                event.prevent_default();
                                context.move_active.call(MoveDir::Last);
                            }
                        }
                        Key::Enter => {
                            event.prevent_default();
                            if (context.open)() {
                                context.toggle_active.call(());
                            } else {
                                context.set_open.call((true, false));
                            }
                        }
                        Key::Escape if (context.open)() => {
                            event.prevent_default();
                            context.set_open.call((false, true));
                        }
                        _ => {}
                    }
                },
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct MultiSelectLayout {
    trigger_left: f64,
    trigger_top: f64,
    trigger_width: f64,
    trigger_height: f64,
    content_width: f64,
    content_height: f64,
    viewport_width: f64,
    viewport_height: f64,
    measured: bool,
}

fn parse_layout(message: &str) -> Option<MultiSelectLayout> {
    let mut parts = message.split('|');
    let mut next = || parts.next()?.parse::<f64>().ok();
    Some(MultiSelectLayout {
        trigger_left: next()?,
        trigger_top: next()?,
        trigger_width: next()?,
        trigger_height: next()?,
        content_width: next()?,
        content_height: next()?,
        viewport_width: next()?,
        viewport_height: next()?,
        measured: true,
    })
}

/// Which side of the trigger the panel opens on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MultiSelectSide {
    #[default]
    Bottom,
    Top,
}

/// Alignment along the trigger's edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MultiSelectAlign {
    #[default]
    Start,
    Center,
    End,
}

/// `MultiSelectContent`.
#[component]
pub fn MultiSelectContent(
    #[props(default)] side: MultiSelectSide,
    #[props(default)] align: MultiSelectAlign,
    #[props(default = 6.0)] side_offset: f64,
    #[props(default = true)] avoid_collisions: bool,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_multi_select();
    let reduce = context.reduce;
    let uid = use_hook(platform::next_scope_id);
    let layout_signal = use_signal(MultiSelectLayout::default);
    let actual_side = use_signal(|| side);
    let morph_ready = use_signal(|| false);

    let trigger_id = context.trigger_id.clone();
    let measure_id = context.measure_id.clone();
    use_effect(move || {
        let script = format!(
            "{}\nwindow.__beuiPopoverStart({uid}, {}, {});",
            include_str!("../js/popover.js"),
            platform::js_string(&format!("#{trigger_id}")),
            platform::js_string(&format!("#{measure_id}")),
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        let mut layout = layout_signal;
        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                if let Some(next) = parse_layout(&message) {
                    if *layout.peek() != next {
                        layout.set(next);
                    }
                }
            }
        });
    });
    use_drop(move || {
        platform::fire(&format!(
            "window.__beuiPopoverStop && window.__beuiPopoverStop({uid})"
        ));
    });

    let mut ready = morph_ready;
    use_effect(move || {
        spawn(async move {
            if Time::delay(Duration::from_millis(16)).await.is_ok() {
                ready.set(true);
            }
        });
    });

    let layout = (layout_signal)();
    let open = (context.open)();

    if open && layout.measured {
        let resolved = if !avoid_collisions {
            side
        } else {
            let trigger_bottom = layout.trigger_top + layout.trigger_height;
            let below = layout.viewport_height - trigger_bottom;
            let above = layout.trigger_top;
            if side == MultiSelectSide::Bottom
                && below < layout.content_height + side_offset
                && above > below
            {
                MultiSelectSide::Top
            } else if side == MultiSelectSide::Top
                && above < layout.content_height + side_offset
                && below > above
            {
                MultiSelectSide::Bottom
            } else {
                side
            }
        };
        let mut slot = actual_side;
        if *slot.peek() != resolved {
            slot.set(resolved);
        }
    }
    let resolved_side = (actual_side)();

    let content_width = if layout.content_width > 0.0 {
        layout.content_width
    } else {
        layout.trigger_width
    };
    let desired_left = match align {
        MultiSelectAlign::End => layout.trigger_left + layout.trigger_width - content_width,
        MultiSelectAlign::Center => {
            layout.trigger_left + (layout.trigger_width - content_width) / 2.0
        }
        MultiSelectAlign::Start => layout.trigger_left,
    };
    let max_left = (layout.viewport_width - content_width - VIEWPORT_PADDING).max(VIEWPORT_PADDING);
    let left = desired_left.max(VIEWPORT_PADDING).min(max_left);

    let height_motion = use_motion(0.0f32).expect("finite");
    let opacity_motion = use_motion(0.0f32).expect("finite");
    let y_motion = use_motion(0.0f32).expect("finite");
    let mut height_signal = height_motion;
    let mut opacity_signal = opacity_motion;
    let mut y_signal = y_motion;
    let layout_for_motion = layout_signal;
    let side_for_motion = actual_side;
    let ready_for_motion = morph_ready;
    use_effect(move || {
        let is_open = (context.open)();
        let current_layout = (layout_for_motion)();
        let current_side = (side_for_motion)();
        let target_height = if is_open {
            current_layout.content_height
        } else {
            0.0
        };
        let target_opacity = if is_open { 1.0 } else { 0.0 };
        let target_y = if is_open {
            if current_side == MultiSelectSide::Bottom {
                side_offset
            } else {
                -side_offset
            }
        } else {
            0.0
        };
        let config = if reduce || !(ready_for_motion)() {
            tween_ease_out(1)
        } else {
            AnimationConfig::spring(spring_from_visual_duration(0.5, 0.22))
        };
        let _ = height_signal.animate_to(target_height as f32, config.clone());
        let _ = opacity_signal.animate_to(target_opacity as f32, config.clone());
        let _ = y_signal.animate_to(target_y as f32, config);
    });

    let height = f64::from(height_motion.get_value());
    let opacity = f64::from(opacity_motion.get_value());
    let y = f64::from(y_motion.get_value());
    let mut style = format!(
        "left:{left}px;min-width:{}px;height:{height}px;opacity:{opacity};transform:translateY({y}px);\
         transform-origin:{};pointer-events:{};visibility:{};--multi-select-trigger-width:{}px;will-change:height,transform",
        layout.trigger_width,
        if resolved_side == MultiSelectSide::Bottom { "top" } else { "bottom" },
        if open { "auto" } else { "none" },
        if layout.measured { "visible" } else { "hidden" },
        layout.trigger_width,
    );
    if resolved_side == MultiSelectSide::Bottom {
        style.push_str(&format!(
            ";top:{}px",
            layout.trigger_top + layout.trigger_height
        ));
    } else {
        style.push_str(&format!(
            ";bottom:{}px",
            layout.viewport_height - layout.trigger_top
        ));
    }
    let content = children;

    rsx! {
        PortalHost {
            div {
                id: "{context.panel_id}",
                "data-multi-select-content": "",
                "data-side": if resolved_side == MultiSelectSide::Bottom { "bottom" } else { "top" },
                "aria-hidden": if open { None } else { Some("true") },
                inert: if open { None } else { Some("true") },
                style,
                class: cn!(
                    "fixed z-[9999] w-(--multi-select-trigger-width) overflow-hidden rounded-xl border border-border bg-background text-popover-foreground outline-none",
                    class.as_deref().unwrap_or(""),
                ),
                div { id: "{context.measure_id}", {content} }
            }
        }
    }
}

/// `MultiSelectList`.
#[component]
pub fn MultiSelectList(
    #[props(default = "Options")] aria_label: &'static str,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_multi_select();
    rsx! {
        div {
            id: "{context.list_id}",
            role: "listbox",
            "aria-label": aria_label,
            "aria-multiselectable": "true",
            class: cn!(
                "relative isolate max-h-64 overflow-y-auto overscroll-contain p-1.5 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `MultiSelectGroup`.
#[component]
pub fn MultiSelectGroup(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_multi_select();
    let group_id = use_hook(|| format!("beui-multi-select-group-{}", platform::next_scope_id()));
    use_context_provider(|| MultiSelectGroupId(Some(group_id.clone())));
    let hidden = !(context.derived)().visible_groups.contains(&group_id);
    rsx! {
        fieldset {
            hidden,
            class: cn!("m-0 min-w-0 border-0 p-0 py-0.5", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `MultiSelectLabel`.
#[component]
pub fn MultiSelectLabel(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let in_group = try_consume_context::<MultiSelectGroupId>()
        .map(|group| group.0.is_some())
        .unwrap_or(false);
    let label_class = cn!(
        "w-full px-2 py-1.5 text-[0.68rem] font-medium uppercase tracking-[0.12em] text-muted-foreground",
        class.as_deref().unwrap_or(""),
    );
    if in_group {
        rsx! { legend { class: label_class, {children} } }
    } else {
        rsx! { div { class: label_class, {children} } }
    }
}

/// `MultiSelectItem`.
#[component]
pub fn MultiSelectItem(
    value: String,
    #[props(default = None)] text_value: Option<String>,
    #[props(default = None)] keywords: Option<Vec<String>>,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] on_select: Option<EventHandler<String>>,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_multi_select();
    let id = use_hook(|| format!("beui-multi-select-item-{}", platform::next_scope_id()));
    let group_id = try_consume_context::<MultiSelectGroupId>().and_then(|group| group.0);
    let label = text_value.unwrap_or_else(|| value.clone());
    let keywords = keywords.unwrap_or_default();

    let register = context.register_item;
    let item_value = value.clone();
    let item_label = label.clone();
    let item_keywords = keywords.clone();
    let item_id = id.clone();
    let item_group = group_id.clone();
    use_effect(move || {
        register.call(RegisteredItem {
            value: item_value.clone(),
            label: item_label.clone(),
            keywords: item_keywords.clone(),
            disabled,
            group_id: item_group.clone(),
            id: item_id.clone(),
        });
    });
    let unregister = context.unregister_item;
    let drop_value = value.clone();
    use_drop(move || unregister.call(drop_value));

    let derived = (context.derived)();
    if !derived.visible.contains(&value) {
        return rsx! {};
    }
    let active = derived.active_value.as_deref() == Some(value.as_str());
    let selected = (context.values)().iter().any(|entry| entry == &value);

    rsx! {
        button {
            id: "{id}",
            r#type: "button",
            role: "option",
            "aria-selected": selected,
            disabled,
            tabindex: -1,
            "data-multi-select-item": "",
            "data-active": if active { Some("true") } else { None },
            "data-selected": if selected { Some("true") } else { None },
            class: cn!(
                "relative flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-sm outline-none transition-colors duration-150",
                if active || selected { "text-foreground" } else { "text-muted-foreground" },
                "disabled:pointer-events-none disabled:opacity-45",
                class.as_deref().unwrap_or(""),
            ),
            onpointermove: {
                let set_active = context.set_active_value;
                let value = value.clone();
                move |_| {
                    if !disabled {
                        set_active.call(Some(value.clone()));
                    }
                }
            },
            onpointerdown: move |event: Event<PointerData>| event.prevent_default(),
            onclick: {
                let toggle = context.toggle;
                let value = value.clone();
                move |_| {
                    if disabled {
                        return;
                    }
                    if let Some(callback) = on_select {
                        callback.call(value.clone());
                    }
                    toggle.call(value.clone());
                }
            },
            if active {
                MultiSelectHighlight { layout_id: context.active_layout_id.clone(), state: value.clone() }
            }
            span { class: "min-w-0 flex-1", {children} }
            span {
                "aria-hidden": "true",
                style: format!(
                    "opacity:{};transform:scale({});transition:opacity 140ms {EASE_OUT_CSS},transform 140ms {EASE_OUT_CSS}",
                    if selected { 1 } else { 0 },
                    if selected { 1.0 } else { 0.82 },
                ),
                class: "grid size-5 shrink-0 place-items-center text-foreground",
                CheckIcon { class: "size-4" }
            }
        }
    }
}

#[component]
fn MultiSelectHighlight(layout_id: String, state: String) -> Element {
    let flip = use_flip(
        state,
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(AnimationConfig::spring(SPRING_LAYOUT)),
    );
    rsx! {
        span {
            "aria-hidden": "true",
            "data-beui-flip": flip.scope(),
            style: flip.style_with(""),
            class: "absolute inset-0 -z-10 rounded-lg bg-muted",
        }
    }
}

/// `MultiSelectEmpty`.
#[component]
pub fn MultiSelectEmpty(
    #[props(default = "No options found.")] message: &'static str,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let context = use_multi_select();
    if (context.derived)().count > 0 {
        return rsx! {};
    }
    rsx! {
        div {
            role: "status",
            class: cn!("px-3 py-8 text-center text-sm text-muted-foreground", class.as_deref().unwrap_or("")),
            "{message}"
        }
    }
}

/// `MultiSelectSeparator`.
#[component]
pub fn MultiSelectSeparator(#[props(default = None)] class: Option<String>) -> Element {
    rsx! {
        div { "aria-hidden": "true", class: cn!("-mx-1 my-1 h-px bg-border", class.as_deref().unwrap_or("")) }
    }
}
