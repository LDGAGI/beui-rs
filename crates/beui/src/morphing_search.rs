//! `motion/morphing-search.tsx` — a search field that morphs into a
//! command-palette panel anchored over it.
//!
//! The pieces are the source's: the `SEARCH_MORPH` spring (`duration 0.58,
//! bounce 0.22`, taken through [`spring_from_visual_duration`] so the numbers
//! match motion's own conversion), the `SEARCH_CLIP_TRANSITION` unfold
//! (0.32s `EASE_OUT`), the trigger → shell handoff, the results list's
//! `opacity 0 → 1` with a 6px lift, the cursor stamped with the query, the
//! `f` shortcut, Escape, the arrow/Enter grammar and the panel geometry
//! (`max(anchorWidth, min(448, innerWidth - left - 16))` wide, results at
//! `max(96, min(288, innerHeight - top - 80))`).
//!
//! Differences, both deliberate: the overlay renders in place
//! rather than through `createPortal` (so the panel is `fixed` against the
//! viewport, which needs no transformed ancestor), and the source's manual
//! wheel/touch scroll lock is replaced by `use_modal_scope`, which also brings
//! the focus trap, Escape and focus return.

use crate::icons::Search;
use crate::prelude::*;
use crate::spring::spring_from_visual_duration;
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::{
    use_dismiss, use_modal_scope, use_on_open, use_row_cursor, DismissOptions, RowStep,
};
use beui_core::platform::{js_string, json, next_scope_id};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

const SEARCH_CLIP_DURATION: u64 = 320;
const PANEL_HEIGHT_HEADER: f64 = 48.0;

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// One result.
#[derive(Clone, PartialEq)]
pub struct MorphingSearchItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    /// Extra terms the filter looks at.
    pub keywords: Vec<String>,
    pub icon: Option<Element>,
    /// Called when the row is chosen.
    pub on_select: Option<EventHandler<()>>,
}

impl MorphingSearchItem {
    /// An item with a title.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            keywords: Vec::new(),
            icon: None,
            on_select: None,
        }
    }

    /// A secondary line.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Extra terms the filter matches on.
    pub fn keywords(mut self, keywords: &[&str]) -> Self {
        self.keywords = keywords.iter().map(|term| term.to_string()).collect();
        self
    }

    /// A leading icon.
    pub fn icon(mut self, icon: Element) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A per-item handler.
    pub fn on_select(mut self, handler: EventHandler<()>) -> Self {
        self.on_select = Some(handler);
        self
    }
}

/// Props for [`MorphingSearch`].
#[derive(Props, Clone, PartialEq)]
pub struct MorphingSearchProps {
    pub items: Vec<MorphingSearchItem>,
    #[props(default = "Search".to_string())]
    pub placeholder: String,
    /// Key that opens the panel from anywhere. Defaults to `"f"`.
    #[props(default = Some("f".to_string()))]
    pub shortcut: Option<String>,
    /// Render the closed trigger as a compact search icon.
    #[props(default)]
    pub icon_only: bool,
    #[props(default = "No results found.".to_string())]
    pub empty_message: String,
    #[props(default)]
    pub open: Option<bool>,
    #[props(default)]
    pub default_open: bool,
    #[props(default)]
    pub on_open_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub on_query_change: Option<EventHandler<String>>,
    /// Called with the chosen item's id.
    #[props(default)]
    pub on_select: Option<EventHandler<String>>,
    #[props(default)]
    pub class: Option<String>,
}

/// The anchor rectangle the panel is placed from.
#[derive(Clone, Copy, PartialEq)]
struct PanelBox {
    top: f64,
    left: f64,
    width: f64,
    viewport_width: f64,
    viewport_height: f64,
}

impl PanelBox {
    fn panel_width(self) -> f64 {
        self.width
            .max((448.0f64).min(self.viewport_width - self.left - 16.0))
    }

    fn results_height(self) -> f64 {
        (288.0f64)
            .min(self.viewport_height - self.top - 80.0)
            .max(96.0)
    }
}

/// A morphing search.
#[component]
pub fn MorphingSearch(props: MorphingSearchProps) -> Element {
    let MorphingSearchProps {
        items,
        placeholder,
        shortcut,
        icon_only,
        empty_message,
        open: controlled_open,
        default_open,
        on_open_change,
        on_query_change,
        on_select,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let scope = use_hook(next_scope_id);
    let shell_layout_id = format!("morphing-search-shell-{scope}");
    let listbox_id = format!("morphing-search-results-{scope}");

    let mut internal_open = use_signal(|| default_open);
    let open = controlled_open.unwrap_or(internal_open());
    let controlled = controlled_open.is_some();
    let query = use_signal(String::new);
    let panel = use_signal(|| PanelBox {
        top: 16.0,
        left: 16.0,
        width: 288.0,
        viewport_width: 1280.0,
        viewport_height: 800.0,
    });

    let set_open = EventHandler::new(move |next: bool| {
        if !controlled {
            internal_open.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    });
    let close = EventHandler::new({
        let mut query = query;
        move |_| {
            query.set(String::new());
            if let Some(handler) = on_query_change {
                handler.call(String::new());
            }
            set_open.call(false);
        }
    });

    // The filtered rows, and the cursor stamped with the query.
    let needle = query().trim().to_lowercase();
    let filtered: Vec<MorphingSearchItem> = if needle.is_empty() {
        items.clone()
    } else {
        items
            .iter()
            .filter(|item| {
                let mut haystack = format!(
                    "{} {}",
                    item.title,
                    item.description.clone().unwrap_or_default()
                );
                for keyword in &item.keywords {
                    haystack.push(' ');
                    haystack.push_str(keyword);
                }
                haystack.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect()
    };
    let row_ids: Vec<String> = filtered.iter().map(|item| item.id.clone()).collect();
    let cursor = std::rc::Rc::new(use_row_cursor(row_ids, &query()));
    let active_index = cursor.active_index();

    use_on_open(open, {
        let mut query = query;
        let cursor = cursor.clone();
        move || {
            query.set(String::new());
            cursor.move_to(None);
        }
    });

    // Measure the anchor and open: the panel is fixed over it.
    let open_search = EventHandler::new({
        let mut panel = panel;
        let selector = format!("[data-beui-search-anchor=\"{scope}\"]");
        move |_| {
            let selector = selector.clone();
            spawn(async move {
                let script = format!(
                    "const el = document.querySelector({selector});\n\
                     if (!el) return null;\n\
                     const r = el.getBoundingClientRect();\n\
                     return [r.top, r.left, r.width, window.innerWidth, window.innerHeight];",
                    selector = js_string(&selector),
                );
                if let Some(values) = json::<Vec<f64>>(&script).await {
                    if values.len() >= 5 && values[2] > 0.0 {
                        panel.set(PanelBox {
                            top: values[0],
                            left: values[1],
                            width: values[2],
                            viewport_width: values[3],
                            viewport_height: values[4],
                        });
                    }
                }
            });
            set_open.call(true);
        }
    });

    // The `f` shortcut, from anywhere that is not an editable target.
    use_effect({
        let shortcut_for_effect = shortcut.clone();
        move || {
            let shortcut = shortcut_for_effect.clone().unwrap_or_default();
            if shortcut.is_empty() {
                return;
            }
            let script = format!(
                "const key = {};\n\
                 const onKey = (event) => {{\n\
                   const target = event.target;\n\
                   const editable = target instanceof HTMLElement && (target.isContentEditable || target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement);\n\
                   if (editable || event.metaKey || event.ctrlKey || event.altKey || event.shiftKey || event.repeat) return;\n\
                   if (event.key.toLowerCase() !== key.toLowerCase()) return;\n\
                   event.preventDefault();\n\
                   dioxus.send(true);\n\
                 }};\n\
                 window.addEventListener('keydown', onKey);\n\
                 for (;;) {{ const message = await dioxus.recv(); if (message === 'close') break; }}\n\
                 window.removeEventListener('keydown', onKey);",
                js_string(&shortcut),
            );
            let Some(mut handle) = beui_core::platform::eval(&script) else {
                return;
            };
            spawn(async move {
                while let Ok(true) = handle.recv::<bool>().await {
                    open_search.call(());
                }
            });
        }
    });

    // The dialog's scope: Escape, the focus trap, the scroll lock and focus
    // return come from the shared modal scope.
    let mut open_signal = use_signal(|| open);
    use_effect(move || open_signal.set(open));
    let close_handler = close;
    let dismiss = use_dismiss(
        open_signal,
        close_handler,
        DismissOptions::default().without_escape(),
    );
    use_modal_scope(open_signal, close_handler, dismiss);

    // The unfold: the shell spring is motion's `duration 0.58, bounce 0.22`;
    // the clip-path uses the 0.32s EASE_OUT tween the source calls out.
    let mut unfold = use_motion(if open { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    let mut mounted = use_signal(|| open);
    let mut open_flag = use_signal(|| open);
    use_effect(use_reactive((&open,), move |(open,)| open_flag.set(open)));
    use_effect(move || {
        if open_flag() {
            mounted.set(true);
            let config = if reduce_value {
                beui_core::ease::tween_ease_out(0)
            } else {
                beui_core::ease::tween_ease_out(SEARCH_CLIP_DURATION)
            };
            let _ = unfold.animate_to(1.0, config);
        } else {
            let _ = unfold.animate_to(0.0, beui_core::ease::tween_ease_out(SEARCH_CLIP_DURATION));
            spawn(async move {
                let _ = json::<bool>(
                    "await new Promise((resolve) => setTimeout(resolve, 380));\nreturn true;",
                )
                .await;
                if !open_flag() {
                    mounted.set(false);
                }
            });
        }
    });
    let progress = f64::from(current(unfold).clamp(0.0, 1.0));

    let box_now = panel();
    let panel_width = box_now.panel_width();
    let results_height = box_now.results_height();
    let collapsed_clip = format!(
        "inset(0px {}px {}px 0px round 12px)",
        (panel_width - box_now.width).max(0.0),
        results_height
    );
    // The clip unfolds, then the list arrives on its 0.16s tween.
    let clip_right = (panel_width - box_now.width).max(0.0) * (1.0 - progress);
    let clip_bottom = results_height * (1.0 - progress);
    let dialog_style = format!(
        "top:{}px;left:{}px;width:{}px;opacity:{progress};clip-path:inset(0px {clip_right}px {clip_bottom}px 0px round 12px)",
        box_now.top, box_now.left, panel_width
    );
    let _ = collapsed_clip;
    let list_progress = ((progress - 0.2) / 0.8).clamp(0.0, 1.0);
    let list_style = format!(
        "max-height:{results_height}px;opacity:{list_progress};transform:translateY({}px)",
        6.0 * (1.0 - list_progress)
    );

    // The trigger morphs into the shell through a shared layout id, exactly as
    // the source hands the shape over with `layoutId`.
    let shell_spring = AnimationConfig::spring(spring_from_visual_duration(0.58, 0.22));
    let trigger_flip = use_flip(
        open,
        FlipOptions::default()
            .layout_id(shell_layout_id.clone())
            .transition(shell_spring.clone()),
    );
    let shell_flip = use_flip(
        open,
        FlipOptions::default()
            .layout_id(shell_layout_id.clone())
            .transition(shell_spring.clone()),
    );

    let root_class = cn!(
        "relative",
        if icon_only {
            "size-12"
        } else {
            "h-12 w-72 max-w-full"
        },
        class,
    );
    let trigger_class = cn!(
        "flex size-full items-center rounded-xl bg-background/60 text-left backdrop-blur-md outline-none [--search-trigger-stroke:var(--color-border)] hover:[--search-trigger-stroke:var(--color-border-strong)] focus-visible:ring-2 focus-visible:ring-ring",
        if icon_only {
            "cursor-pointer justify-center"
        } else {
            "cursor-text px-3.5"
        },
    );
    let placeholder_class = cn!(
        "pointer-events-none absolute inset-0 flex items-center",
        if icon_only {
            "justify-center"
        } else {
            "gap-2.5 px-3.5"
        },
    );
    let placeholder_opacity = if open { 0.0 } else { 1.0 };
    let list_class = "overscroll-contain overflow-y-auto p-2";

    let trigger_key = "trigger";
    let shortcut_for_kbd = shortcut.clone();

    rsx! {
        div { class: root_class,
            div { "data-beui-search-anchor": "{scope}", class: "absolute inset-0",
                if !open {
                    button {
                        key: "{trigger_key}",
                        r#type: "button",
                        "data-beui-flip": trigger_flip.scope(),
                        aria_haspopup: "dialog",
                        aria_expanded: "false",
                        aria_label: "{placeholder}",
                        class: trigger_class,
                        style: trigger_flip.style_with("box-shadow:inset 0 0 0 1px var(--search-trigger-stroke)"),
                        onclick: move |_| open_search.call(()),
                        span { class: "sr-only", "{placeholder}" }
                    }
                }
                // The placeholder reads through the trigger while it is closed.
                div {
                    aria_hidden: "true",
                    class: placeholder_class,
                    style: "opacity:{placeholder_opacity};transition:opacity 100ms ease-out",
                    Search { class: Some("size-4 shrink-0 text-muted-foreground".to_string()) }
                    if !icon_only {
                        span { class: "min-w-0 flex-1 truncate text-sm text-muted-foreground", "{placeholder}" }
                        if let Some(shortcut) = shortcut_for_kbd.clone() {
                            kbd { class: "flex h-7 min-w-7 shrink-0 items-center justify-center rounded-md border border-border px-2 text-xs text-muted-foreground",
                                "{shortcut.to_uppercase()}"
                            }
                        }
                    }
                }
            }

            if mounted() {
                // The shell: the shape the trigger hands over to.
                div {
                    "data-beui-flip": shell_flip.scope(),
                    aria_hidden: "true",
                    class: "fixed z-10 rounded-xl bg-background/90 backdrop-blur-xl",
                    style: shell_flip.style_with(&format!(
                        "top:{}px;left:{}px;width:{}px;height:{}px;box-shadow:inset 0 0 0 1px var(--color-border)",
                        box_now.top,
                        box_now.left,
                        panel_width,
                        PANEL_HEIGHT_HEADER + results_height,
                    )),
                }
                div {
                    "data-beui-scope": dismiss.attr(),
                    role: "dialog",
                    aria_modal: "true",
                    aria_label: "Search",
                    class: "fixed z-20 overflow-hidden rounded-xl",
                    style: dialog_style,
                    onkeydown: {
                        let cursor = cursor.clone();
                        let filtered = filtered.clone();
                        move |event: Event<KeyboardData>| {
                            match event.key() {
                                Key::ArrowDown => {
                                    event.prevent_default();
                                    cursor.move_active(RowStep::Next);
                                }
                                Key::ArrowUp => {
                                    event.prevent_default();
                                    cursor.move_active(RowStep::Previous);
                                }
                                Key::Enter => {
                                    event.prevent_default();
                                    if let Some(item) = filtered.get(cursor.active_index()) {
                                        if let Some(handler) = item.on_select {
                                            handler.call(());
                                        }
                                        if let Some(handler) = on_select {
                                            handler.call(item.id.clone());
                                        }
                                        close.call(());
                                    }
                                }
                                _ => {}
                            }
                        }
                    },
                    div { class: cn!("flex h-12 items-center gap-2.5 border-b border-border", if icon_only { "px-4" } else { "px-3.5" }),
                        span { class: "shrink-0",
                            Search { class: Some("size-4 text-muted-foreground".to_string()) }
                        }
                        div { class: "flex h-10 min-w-0 flex-1 items-center",
                            input {
                                value: "{query()}",
                                role: "combobox",
                                aria_label: "{placeholder}",
                                aria_expanded: "true",
                                aria_controls: "{listbox_id}",
                                aria_autocomplete: "list",
                                aria_activedescendant: if !filtered.is_empty() {
                                    Some(format!("morphing-search-option-{scope}-{active_index}"))
                                } else {
                                    None
                                },
                                placeholder,
                                class: "size-full bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground",
                                oninput: {
                                    let mut query = query;
                                    move |event: Event<FormData>| {
                                        query.set(event.value());
                                        if let Some(handler) = on_query_change {
                                            handler.call(event.value());
                                        }
                                    }
                                },
                                onmounted: {
                                    move |_| {
                                        let selector = format!("[data-beui-search-input=\"{scope}\"]");
                                        spawn(async move {
                                            let script = format!(
                                                "await new Promise((resolve) => requestAnimationFrame(resolve));\n\
                                                 const el = document.querySelector({selector});\n\
                                                 if (el) el.focus();\n\
                                                 return true;",
                                                selector = js_string(&selector),
                                            );
                                            let _ = json::<bool>(&script).await;
                                        });
                                    }
                                },
                                "data-beui-search-input": "{scope}",
                            }
                        }
                        kbd { class: "flex h-7 shrink-0 items-center rounded-md border border-border px-2 text-xs text-muted-foreground",
                            "Esc"
                        }
                    }
                    div {
                        id: "{listbox_id}",
                        role: "listbox",
                        aria_label: "Search results",
                        class: list_class,
                        style: list_style,
                        if !filtered.is_empty() {
                            for (index, item) in filtered.iter().enumerate() {
                                button {
                                    key: "{item.id}",
                                    id: "morphing-search-option-{scope}-{index}",
                                    r#type: "button",
                                    role: "option",
                                    aria_selected: if index == active_index { "true" } else { "false" },
                                    "data-index": "{index}",
                                    class: "relative flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
                                    onpointermove: {
                                        let cursor = cursor.clone();
                                        let id = item.id.clone();
                                        move |_| cursor.move_to(Some(&id))
                                    },
                                    onfocus: {
                                        let cursor = cursor.clone();
                                        let id = item.id.clone();
                                        move |_| cursor.move_to(Some(&id))
                                    },
                                    onclick: {
                                        let item = item.clone();
                                        move |_| {
                                            if let Some(handler) = item.on_select {
                                                handler.call(());
                                            }
                                            if let Some(handler) = on_select {
                                                handler.call(item.id.clone());
                                            }
                                            close.call(());
                                        }
                                    },
                                    if index == active_index {
                                        span { class: "absolute inset-0 rounded-lg bg-foreground/5" }
                                    }
                                    if let Some(icon) = item.icon.clone() {
                                        span { class: "relative inline-flex size-4 shrink-0 items-center justify-center text-muted-foreground [&>svg]:size-4",
                                            {icon}
                                        }
                                    }
                                    span { class: "relative min-w-0",
                                        span { class: "block truncate text-sm font-medium text-foreground", "{item.title}" }
                                        if let Some(description) = item.description.clone() {
                                            span { class: "block truncate text-xs text-muted-foreground", "{description}" }
                                        }
                                    }
                                }
                            }
                        } else {
                            p { class: "px-3 py-8 text-center text-sm text-muted-foreground", "{empty_message}" }
                        }
                    }
                }
            }
        }
    }
}
