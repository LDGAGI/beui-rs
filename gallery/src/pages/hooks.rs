//! The four remaining `lib/hooks/` primitives, each with the state it owns on
//! screen: the row cursor, the open transition, the slider and the modal scope.
//!
//! There is no beUI page for these — beUI's hooks are only visible through the
//! components that use them — so each panel is the smallest markup that makes
//! the hook's contract observable, in the same visual language as the component
//! previews. The verification script drives all four.

use beui::prelude::*;
use beui_core::hooks::{
    use_modal_scope, use_on_open, use_row_cursor, use_slider, DismissOptions, RowStep, SliderConfig,
};

/// The list the row cursor walks.
const ROWS: [&str; 5] = ["Alpha", "Bravo", "Charlie", "Delta", "Echo"];

#[component]
pub fn Hooks() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Foundations"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                    "Shared input primitives"
                }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "useRowCursor, useOnOpen, useSlider and useModalScope: the four lib/hooks/ modules every form control, listbox and overlay is built on."
                }
            }
            RowCursorPanel {}
            OnOpenPanel {}
            SliderPanel {}
            ModalPanel {}
        }
    }
}

#[component]
fn Panel(title: String, note: String, children: Element) -> Element {
    rsx! {
        section { class: "flex flex-col gap-4",
            div { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold tracking-tight", "{title}" }
                p { class: "max-w-3xl text-sm text-muted-foreground", "{note}" }
            }
            {children}
        }
    }
}

#[component]
fn Probe(name: &'static str, value: String) -> Element {
    rsx! {
        span {
            "data-probe": "{name}",
            "data-probe-value": "{value}",
            class: "inline-flex items-center gap-1 rounded-full border border-border bg-card px-2.5 py-1 font-mono text-[11px] text-muted-foreground",
            "{name} = {value}"
        }
    }
}

// --- row cursor --------------------------------------------------------------

#[component]
fn RowCursorPanel() -> Element {
    let query = use_signal(String::new);
    let chosen = use_signal(|| Option::<String>::None);

    let needle = query().to_lowercase();
    let rows: Vec<String> = ROWS
        .iter()
        .filter(|row| row.to_lowercase().contains(&needle))
        .map(|row| (*row).to_string())
        .collect();
    let cursor = use_row_cursor(rows.clone(), &query());
    let active = cursor.active_id();

    rsx! {
        Panel {
            title: "useRowCursor",
            note: "Arrow keys walk the list, typing re-filters it. The cursor holds a row id, not a position, and drops itself when the query moves on — so Enter can never commit a row the user did not aim at.",
            div { class: "flex flex-col gap-3 rounded-xl border border-border bg-card p-4",
                input {
                    "data-cursor-input": "true",
                    class: "h-9 w-full rounded-lg border border-border bg-background px-3 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-primary",
                    placeholder: "Filter rows…",
                    value: "{query}",
                    oninput: move |event| {
                        let mut signal = query;
                        signal.set(event.value());
                    },
                    onkeydown: {
                        let cursor = cursor.clone();
                        move |event| match event.key() {
                            Key::ArrowDown => cursor.move_active(RowStep::Next),
                            Key::ArrowUp => cursor.move_active(RowStep::Previous),
                            Key::Enter => {
                                let value = cursor.active_id();
                                let mut signal = chosen;
                                signal.set(value);
                            }
                            _ => {}
                        }
                    },
                }
                div {
                    "data-cursor-list": "true",
                    role: "listbox",
                    class: "flex flex-col gap-1",
                    for row in rows.iter() {
                        div {
                            key: "{row}",
                            id: "row-{row}",
                            role: "option",
                            "data-cursor-row": "{row}",
                            "data-cursor-active": if Some(row) == active.as_ref() { "true" } else { "false" },
                            "aria-selected": if Some(row) == active.as_ref() { "true" } else { "false" },
                            class: cn!(
                                "cursor-pointer rounded-lg px-3 py-2 text-sm",
                                if Some(row) == active.as_ref() { "bg-primary text-primary-foreground" } else { "text-muted-foreground" },
                            ),
                            onclick: {
                                let cursor = cursor.clone();
                                let row = row.clone();
                                move |_| cursor.move_to(Some(&row))
                            },
                            "{row}"
                        }
                    }
                }
                div { class: "flex flex-wrap items-center gap-2",
                    Probe { name: "rows", value: rows.len().to_string() }
                    Probe { name: "active", value: active.clone().unwrap_or_else(|| "none".into()) }
                    Probe { name: "chosen", value: chosen().unwrap_or_else(|| "none".into()) }
                }
            }
        }
    }
}

// --- on open ----------------------------------------------------------------

#[component]
fn OnOpenPanel() -> Element {
    let open = use_signal(|| false);
    let sessions = use_signal(|| 0u32);
    let draft = use_signal(String::new);

    // Opening starts a fresh session: the draft is cleared and the counter ticks.
    use_on_open(open(), move || {
        let mut count = sessions;
        count += 1;
        let mut text = draft;
        text.set(String::new());
    });

    rsx! {
        Panel {
            title: "useOnOpen",
            note: "Opening the panel resets the draft and ticks the session counter. Mounting it open does not fire — the call is for a transition, not a state.",
            div { class: "flex flex-col gap-3 rounded-xl border border-border bg-card p-4",
                div { class: "flex items-center gap-2",
                    button {
                        "data-on-open-toggle": "true",
                        r#type: "button",
                        class: "press h-9 rounded-lg border border-border bg-background px-3 text-sm",
                        onclick: move |_| {
                            let next = !open();
                            let mut signal = open;
                            signal.set(next);
                        },
                        if open() { "Close" } else { "Open" }
                    }
                    Probe { name: "sessions", value: sessions().to_string() }
                    Probe { name: "open", value: open().to_string() }
                }
                input {
                    "data-on-open-draft": "true",
                    class: "h-9 rounded-lg border border-border bg-background px-3 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-primary",
                    placeholder: "Draft — cleared on each open",
                    value: "{draft}",
                    oninput: move |event| {
                        let mut signal = draft;
                        signal.set(event.value());
                    },
                }
                if open() {
                    p { class: "text-sm text-muted-foreground",
                        "Session {sessions()}: started fresh. Anything typed before this open is gone."
                    }
                }
            }
        }
    }
}

// --- slider -----------------------------------------------------------------

#[component]
fn SliderPanel() -> Element {
    // Uncontrolled: the hook owns the number.
    let uncontrolled = use_slider(
        SliderConfig {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            default_value: 25.0,
            value_text: Some(Callback::new(|value| format!("{value:.0}%"))),
            ..Default::default()
        },
        None,
        None,
    );

    // Controlled: the caller owns it, the hook only reports changes.
    let bound = use_signal(|| 6.0);
    let controlled = use_slider(
        SliderConfig {
            min: 0.0,
            max: 10.0,
            step: 0.5,
            ..Default::default()
        },
        Some(bound()),
        Some(EventHandler::new(move |value: f64| {
            let mut signal = bound;
            signal.set(value);
        })),
    );

    rsx! {
        Panel {
            title: "useSlider",
            note: "Value, step snapping, pointer capture and the arrow-key map. One instance owns its value, the other is controlled from outside; both announce themselves the way the source's sliderProps do.",
            div { class: "flex flex-col gap-6 rounded-xl border border-border bg-card p-4",
                SliderTrack { slider: uncontrolled, label: "Uncontrolled · 0–100 step 1" }
                SliderTrack { slider: controlled, label: "Controlled · 0–10 step 0.5" }
            }
        }
    }
}

#[component]
fn SliderTrack(slider: beui_core::Slider, label: String) -> Element {
    let (min, max) = slider.range();
    let percent = slider.percent();

    rsx! {
        div { class: "flex flex-col gap-2",
            div { class: "flex items-center justify-between text-xs text-muted-foreground",
                span { "{label}" }
                span { "data-slider-value": slider.scope(), "{slider.current()}" }
            }
            div {
                "data-beui-slider": slider.scope(),
                class: "relative flex h-8 w-full cursor-pointer touch-none items-center select-none [-webkit-touch-callout:none]",
                onmounted: move |data| slider.measure(data.data()),
                onpointerdown: move |event| slider.on_pointer_down(&event),
                onpointermove: move |event| slider.on_pointer_move(&event),
                onpointerup: move |event| slider.end_drag(&event),
                onpointercancel: move |event| slider.end_drag(&event),
                onlostpointercapture: move |event| slider.end_drag(&event),
                div { class: "h-2 w-full overflow-hidden rounded-full bg-border",
                    div {
                        "data-slider-fill": slider.scope(),
                        class: "h-full rounded-full bg-primary",
                        style: "width:{percent}%",
                    }
                }
                button {
                    "data-beui-slider-handle": slider.scope(),
                    r#type: "button",
                    role: "slider",
                    tabindex: if slider.disabled() { -1 } else { 0 },
                    "aria-label": "{label}",
                    "aria-valuemin": "{min}",
                    "aria-valuemax": "{max}",
                    "aria-valuenow": "{slider.current()}",
                    "aria-valuetext": slider.aria_value_text(),
                    "aria-disabled": if slider.disabled() { "true" } else { "false" },
                    class: "absolute top-1/2 size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-primary bg-background",
                    style: "left:{percent}%",
                    onkeydown: move |event| slider.on_key_down(&event),
                }
            }
        }
    }
}

// --- modal scope ------------------------------------------------------------

#[component]
fn ModalPanel() -> Element {
    let open = use_signal(|| false);
    let on_close = EventHandler::new(move |_| {
        let mut signal = open;
        signal.set(false);
    });
    // Escape belongs to the scope (it swallows the key); outside clicks still
    // dismiss, so the dismiss scope runs with `escape` off.
    let scope =
        beui_core::hooks::use_dismiss(open, on_close, DismissOptions::default().without_escape());
    use_modal_scope(open, on_close, scope);

    rsx! {
        Panel {
            title: "useModalScope",
            note: "inert on everything behind the panel, a locked body scroll, focus trapped and returned, Escape to close. The counters live on window, so two overlapping modals restore what each found.",
            div {
                "data-modal-sibling": "true",
                class: "flex flex-wrap items-center gap-3 rounded-xl border border-border bg-card p-4",
                button {
                    "data-modal-open": "true",
                    r#type: "button",
                    class: "press h-9 rounded-lg bg-primary px-3 text-sm text-primary-foreground",
                    onclick: move |_| {
                        let mut signal = open;
                        signal.set(true);
                    },
                    "Open modal"
                }
                span { class: "text-sm text-muted-foreground",
                    "Focus starts on the first action and comes back here on close."
                }
                Probe { name: "modal-open", value: open().to_string() }
            }

            if open() {
                // The backdrop is decoration: the scope marker goes on the panel,
                // so a click on the backdrop is outside the scope and dismisses.
                div {
                    class: "fixed inset-0 z-50 grid place-items-center bg-black/40 p-6",
                    div {
                        "data-beui-scope": scope.attr(),
                        role: "dialog",
                        "aria-modal": "true",
                        "aria-label": "Modal scope demo",
                        class: "glass flex w-full max-w-sm flex-col gap-3 rounded-2xl p-5",
                        h3 { class: "text-base font-semibold", "Siblings are inert" }
                        p { class: "text-pretty text-sm leading-6 text-muted-foreground",
                            "Tab cycles between these two actions; the page behind cannot be reached, clicked or read out."
                        }
                        div { class: "flex justify-end gap-2",
                            button {
                                "data-modal-action": "first",
                                r#type: "button",
                                class: "press h-9 rounded-lg border border-border px-3 text-sm",
                                onclick: move |_| {
                                    let mut signal = open;
                                    signal.set(false);
                                },
                                "Dismiss"
                            }
                            button {
                                "data-modal-action": "second",
                                r#type: "button",
                                class: "press h-9 rounded-lg bg-primary px-3 text-sm text-primary-foreground",
                                "Confirm"
                            }
                        }
                    }
                }
            }
        }
    }
}
