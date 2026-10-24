//! `components/motion/context-menu.tsx` — a menu at the pointer, with a fold.
//!
//! The menu opens at the invocation point, clamped 8 px inside the viewport,
//! and unfolds out of that point: the clip morphs from a tiny inset around the
//! cursor to the full rounded rect. The trigger opens on right-click, on a
//! keyboard ContextMenu key / Shift+F10, and on a 520 ms touch or pen hold.
//!
//! Deviations from the source:
//!
//! - the trigger renders its own `<div>` wrapper (no `cloneElement`);
//! - `holdSelection` is not ported — `TOUCH_GESTURE_CONTENT_CLASS` already
//!   suppresses selection on coarse pointers;
//! - focus movement, Home/End and typeahead are one-line JS focus calls rather
//!   than Rust state.

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

use crate::icons::CheckIcon;
use crate::motion::PortalHost;
use beui_core::ease::{EASE_OUT_CSS, SPRING_LAYOUT};
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform;
use beui_core::touch::TOUCH_GESTURE_CONTENT_CLASS;
use beui_core::{cn, use_reduced_motion};

const VIEWPORT_PADDING: f64 = 8.0;
const LONG_PRESS_DELAY_MS: u64 = 520;
const LONG_PRESS_TOLERANCE: f64 = 10.0;

/// A point in viewport coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContextMenuPoint {
    pub x: f64,
    pub y: f64,
}

/// How the menu was summoned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContextMenuModality {
    #[default]
    Pointer,
    Keyboard,
    Touch,
}

#[derive(Clone)]
struct ContextMenuContext {
    open: Signal<bool>,
    set_open: EventHandler<bool>,
    open_at: EventHandler<(ContextMenuPoint, ContextMenuModality)>,
    point: Signal<ContextMenuPoint>,
    modality: Signal<ContextMenuModality>,
    invocation: Signal<u32>,
    menu_id: String,
    active_id: Signal<Option<String>>,
    reduce: bool,
}

fn use_context_menu() -> ContextMenuContext {
    use_context::<ContextMenuContext>()
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

/// The collapsed clip: a small rounded rect centred on the invocation point.
fn collapsed_clip(origin: ContextMenuPoint, width: f64, height: f64) -> String {
    let half = 8.0;
    let top = clamp(origin.y - half, 0.0, height);
    let right = clamp(width - origin.x - half, 0.0, width);
    let bottom = clamp(height - origin.y - half, 0.0, height);
    let left = clamp(origin.x - half, 0.0, width);
    format!("inset({top}px {right}px {bottom}px {left}px round 10px)")
}

fn enabled_items_js(menu_id: &str) -> String {
    let selector = platform::js_string(&format!(
        "#{menu_id} [data-context-menu-item=\"true\"]:not([data-disabled=\"true\"])"
    ));
    format!("Array.from(document.querySelectorAll({selector}))")
}

/// `ContextMenu`.
#[component]
pub fn ContextMenu(
    #[props(default = None)] open: Option<bool>,
    #[props(default = false)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let mut internal_open = use_signal(|| default_open);
    let controlled = open.is_some();
    let fallback = internal_open();
    let current_open = open.unwrap_or(fallback);
    let open_signal = use_signal(|| current_open);
    let mut open_sync = open_signal;
    if *open_sync.peek() != current_open {
        open_sync.set(current_open);
    }
    let point = use_signal(ContextMenuPoint::default);
    let modality = use_signal(ContextMenuModality::default);
    let invocation = use_signal(|| 0u32);
    let active_id = use_signal(|| None::<String>);
    let uid = use_hook(platform::next_scope_id);
    let menu_id = format!("beui-ctx-{uid}");

    let set_open = EventHandler::new(move |next: bool| {
        if !controlled {
            internal_open.set(next);
        }
        if let Some(callback) = on_open_change {
            callback.call(next);
        }
        if !next {
            let mut active = active_id;
            active.set(None);
        }
    });
    let open_at = EventHandler::new(
        move |(next_point, next_modality): (ContextMenuPoint, ContextMenuModality)| {
            let mut p = point;
            p.set(next_point);
            let mut m = modality;
            m.set(next_modality);
            let mut inv = invocation;
            inv += 1;
            let mut active = active_id;
            active.set(None);
            let mut open = open_signal;
            open.set(true);
        },
    );

    // Outside pointer / Escape. (Scroll and resize dismissal is not ported.)
    let close = EventHandler::new(move |_| set_open.call(false));
    let menu_selector = format!("#{menu_id}");
    let scope = use_dismiss(
        open_signal,
        close,
        DismissOptions::default().ignore(menu_selector),
    );

    use_context_provider(|| ContextMenuContext {
        open: open_signal,
        set_open,
        open_at,
        point,
        modality,
        invocation,
        menu_id: menu_id.clone(),
        active_id,
        reduce,
    });

    rsx! {
        div {
            "data-beui-scope": scope.attr(),
            class: cn!("contents", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `ContextMenuTrigger`.
#[component]
pub fn ContextMenuTrigger(
    #[props(default = false)] disabled: bool,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_context_menu();
    let open = (context.open)();
    let press_token = use_signal(|| 0u32);
    let origin = use_signal(ContextMenuPoint::default);
    let trigger_node: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

    let cancel_press = {
        move || {
            let mut token = press_token;
            token += 1;
        }
    };
    let open_at = context.open_at;

    rsx! {
        div {
            tabindex: 0,
            "aria-haspopup": "menu",
            "aria-expanded": open,
            "aria-controls": if open { Some(context.menu_id.clone()) } else { None },
            class: cn!("outline-none", TOUCH_GESTURE_CONTENT_CLASS, class.as_deref().unwrap_or("")),
            onmounted: move |event| {
                let mut node = trigger_node;
                node.set(Some(event.data()));
            },
            oncontextmenu: move |event: MouseEvent| {
                event.prevent_default();
                if disabled {
                    return;
                }
                cancel_press();
                let coords = event.client_coordinates();
                open_at.call((ContextMenuPoint { x: coords.x, y: coords.y }, ContextMenuModality::Pointer));
            },
            onpointerdown: move |event: Event<PointerData>| {
                if disabled {
                    return;
                }
                let pointer_type = event.pointer_type();
                if pointer_type != "touch" && pointer_type != "pen" {
                    return;
                }
                let coords = event.client_coordinates();
                let start = ContextMenuPoint { x: coords.x, y: coords.y };
                let mut origin_signal = origin;
                origin_signal.set(start);
                let mut token = press_token;
                token += 1;
                let current = *token.peek();
                spawn(async move {
                    if Time::delay(Duration::from_millis(LONG_PRESS_DELAY_MS)).await.is_ok()
                        && *token.peek() == current
                    {
                        open_at.call((start, ContextMenuModality::Touch));
                    }
                });
            },
            onpointermove: move |event: Event<PointerData>| {
                let start = (origin)();
                let coords = event.client_coordinates();
                if (coords.x - start.x).hypot(coords.y - start.y) > LONG_PRESS_TOLERANCE {
                    let mut token = press_token;
                    token += 1;
                }
            },
            onpointerup: move |_| cancel_press(),
            onpointercancel: move |_| cancel_press(),
            onkeydown: {
                let node = trigger_node;
                move |event: KeyboardEvent| {
                    if disabled {
                        return;
                    }
                    let key = event.key().to_string();
                    let context_key = key == "ContextMenu" || (event.modifiers().shift() && key == "F10");
                    if !context_key {
                        return;
                    }
                    event.prevent_default();
                    let Some(mounted) = node() else {
                        return;
                    };
                    let open_callback = open_at;
                    spawn(async move {
                        if let Some(rect) = platform::client_rect(&mounted).await {
                            open_callback.call((
                                ContextMenuPoint {
                                    x: rect.left + 24.0_f64.min(rect.width / 2.0),
                                    y: rect.top + rect.height / 2.0,
                                },
                                ContextMenuModality::Keyboard,
                            ));
                        }
                    });
                }
            },
            {children}
        }
    }
}

/// `ContextMenuContent`.
#[component]
pub fn ContextMenuContent(
    #[props(default = None)] class: Option<String>,
    #[props(default = "Context menu")] aria_label: &'static str,
    children: Element,
) -> Element {
    let context = use_context_menu();
    let open = (context.open)();
    let reduce = context.reduce;
    let menu_id = context.menu_id.clone();

    let position = use_signal(ContextMenuPoint::default);
    let origin = use_signal(ContextMenuPoint::default);
    let size = use_signal(|| (0.0_f64, 0.0_f64));
    let morph_ready = use_signal(|| false);

    // Measure the menu, clamp it into the viewport, and prepare the collapsed
    // clip one frame before expanding it.
    let modality = context.modality;
    let invocation = context.invocation;
    let open_signal = context.open;
    let menu_id_effect = menu_id.clone();
    use_effect(move || {
        if !open_signal() {
            let mut ready = morph_ready;
            ready.set(false);
            return;
        }
        let _ = (invocation)();
        let current_modality = (modality)();
        let selector = platform::js_string(&format!("#{menu_id_effect}"));
        let script = format!(
            "(() => {{ const el = document.querySelector({selector}); \
             if (!el) {{ dioxus.send('0|0|0|0'); return; }} const r = el.getBoundingClientRect(); \
             dioxus.send(r.width + '|' + r.height + '|' + window.innerWidth + '|' + window.innerHeight); }})()"
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        let point = context.point;
        let mut position_signal = position;
        let mut origin_signal = origin;
        let mut size_signal = size;
        let mut ready = morph_ready;
        ready.set(false);
        spawn(async move {
            if let Ok(message) = eval.recv::<String>().await {
                let mut parts = message.split('|');
                let parse =
                    |value: Option<&str>| value.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                let width = parse(parts.next());
                let height = parse(parts.next());
                let viewport_width = parse(parts.next());
                let viewport_height = parse(parts.next());
                let point = *point.peek();
                let left = clamp(
                    point.x.max(VIEWPORT_PADDING),
                    VIEWPORT_PADDING,
                    (viewport_width - width - VIEWPORT_PADDING).max(VIEWPORT_PADDING),
                );
                let top = clamp(
                    point.y.max(VIEWPORT_PADDING),
                    VIEWPORT_PADDING,
                    (viewport_height - height - VIEWPORT_PADDING).max(VIEWPORT_PADDING),
                );
                position_signal.set(ContextMenuPoint { x: left, y: top });
                size_signal.set((width, height));
                origin_signal.set(ContextMenuPoint {
                    x: clamp(point.x - left, 12.0, (width - 12.0).max(12.0)),
                    y: clamp(point.y - top, 12.0, (height - 12.0).max(12.0)),
                });
            }
            if reduce || current_modality == ContextMenuModality::Keyboard {
                ready.set(true);
                return;
            }
            if Time::delay(Duration::from_millis(32)).await.is_ok() {
                ready.set(true);
            }
        });
    });

    // Focus the first item when the menu opens.
    let menu_id_focus = menu_id.clone();
    use_effect(move || {
        if !open_signal() {
            return;
        }
        let items = enabled_items_js(&menu_id_focus);
        platform::fire(&format!("({items})[0]?.focus({{ preventScroll: true }})"));
    });

    let is_open = open;
    let ready = (morph_ready)();
    let (width, height) = (size)();
    let collapsed = collapsed_clip((origin)(), width, height);
    let clip = if reduce || (context.modality)() == ContextMenuModality::Keyboard || ready {
        "inset(0px 0px 0px 0px round 12px)".to_string()
    } else {
        collapsed
    };
    let position = (position)();
    let style = format!(
        "clip-path:{clip};opacity:{};transition:clip-path 300ms {EASE_OUT_CSS},opacity 300ms {EASE_OUT_CSS}",
        if is_open { 1 } else { 0 },
    );
    let content = children;

    // Keyboard navigation lives in small JS focus calls.
    let items_js = enabled_items_js(&context.menu_id);
    let set_open = context.set_open;

    rsx! {
        PortalHost {
            div {
                "data-context-menu-portal": "",
                "aria-hidden": if is_open { None } else { Some("true") },
                inert: if is_open { None } else { Some("true") },
                style: format!("left:{}px;top:{}px", position.x, position.y),
                class: cn!(
                    "fixed z-[100] [filter:drop-shadow(0_18px_28px_rgba(0,0,0,0.2))]",
                    if is_open { "pointer-events-auto" } else { "pointer-events-none" },
                ),
                div {
                    id: "{menu_id}",
                    role: "menu",
                    "aria-label": aria_label,
                    "data-morph-ready": if ready { "true" } else { "false" },
                    tabindex: -1,
                    style,
                    class: cn!(
                        "min-w-56 overflow-hidden rounded-xl border border-border bg-card p-1.5 text-foreground outline-none",
                        class.as_deref().unwrap_or(""),
                    ),
                    oncontextmenu: move |event: MouseEvent| event.prevent_default(),
                    onkeydown: {
                        let items_js = items_js.clone();
                        move |event: KeyboardEvent| {
                            let key = event.key().to_string();
                            match key.as_str() {
                                "Escape" => {
                                    event.prevent_default();
                                    set_open.call(false);
                                }
                                "Tab" => set_open.call(false),
                                "ArrowDown" => {
                                    event.prevent_default();
                                    platform::fire(&format!(
                                        "(() => {{ const items = {items_js}; if (!items.length) return; \
                                         let i = items.indexOf(document.activeElement); i = (i + 1 + items.length) % items.length; items[i].focus(); }})()"
                                    ));
                                }
                                "ArrowUp" => {
                                    event.prevent_default();
                                    platform::fire(&format!(
                                        "(() => {{ const items = {items_js}; if (!items.length) return; \
                                         let i = items.indexOf(document.activeElement); i = (i - 1 + items.length) % items.length; items[i].focus(); }})()"
                                    ));
                                }
                                "Home" => {
                                    event.prevent_default();
                                    platform::fire(&format!("({items_js})[0]?.focus()"));
                                }
                                "End" => {
                                    event.prevent_default();
                                    platform::fire(&format!("{{ const items = {items_js}; items[items.length - 1]?.focus(); }}"));
                                }
                                _ => {
                                    if key.chars().count() == 1 && !event.modifiers().ctrl() && !event.modifiers().meta() && !event.modifiers().alt() {
                                        platform::fire(&format!(
                                            "(() => {{ const q = {}; const items = {items_js}; \
                                             const match = items.find(el => ((el.dataset.label || el.textContent || '').trim().toLowerCase().startsWith(q))); \
                                             match?.focus(); }})()",
                                            platform::js_string(&key.to_lowercase()),
                                        ));
                                    }
                                }
                            }
                        }
                    },
                    {content}
                }
            }
        }
    }
}

/// The shared body of every menu item.
#[component]
#[allow(clippy::too_many_arguments)]
fn ContextMenuItemBase(
    #[props(default = "menuitem")] role: &'static str,
    #[props(default = None)] aria_checked: Option<bool>,
    #[props(default = false)] disabled: bool,
    #[props(default = true)] close_on_select: bool,
    #[props(default = false)] destructive: bool,
    #[props(default = false)] inset: bool,
    #[props(default = None)] text_value: Option<String>,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] on_select: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let context = use_context_menu();
    let id = use_hook(|| format!("beui-ctx-item-{}", platform::next_scope_id()));
    let active = (context.active_id)().as_deref() == Some(id.as_str());
    let mut active_signal = context.active_id;
    let set_open = context.set_open;
    let id_focus = id.clone();
    let id_hover = id.clone();

    rsx! {
        button {
            r#type: "button",
            id: "{id}",
            role,
            "aria-checked": aria_checked,
            disabled,
            "data-context-menu-item": "true",
            "data-disabled": if disabled { Some("true") } else { None },
            "data-label": text_value,
            tabindex: -1,
            class: cn!(
                "relative isolate flex w-full select-none items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-[13px] outline-none",
                "focus-visible:ring-2 focus-visible:ring-foreground/15 disabled:pointer-events-none disabled:opacity-40",
                if inset { "pl-8" } else { "" },
                if destructive { "text-destructive" } else { "text-foreground" },
                class.as_deref().unwrap_or(""),
            ),
            onfocus: move |_| active_signal.set(Some(id_focus.clone())),
            onpointermove: {
                // Pointer move focuses the row so the highlight follows the mouse.
                move |event: Event<PointerData>| {
                    if disabled || event.pointer_type() == "touch" {
                        return;
                    }
                    platform::fire(&format!(
                        "document.getElementById({})?.focus()",
                        platform::js_string(&id_hover)
                    ));
                }
            },
            onclick: move |_| {
                if disabled {
                    return;
                }
                if let Some(callback) = on_select {
                    callback.call(());
                }
                if close_on_select {
                    set_open.call(false);
                }
            },
            if active {
                MenuHighlight { menu_id: context.menu_id.clone(), destructive }
            }
            {children}
        }
    }
}

#[component]
fn MenuHighlight(menu_id: String, destructive: bool) -> Element {
    let flip = use_flip(
        destructive,
        FlipOptions::default()
            .layout_id(format!("{menu_id}-active"))
            .transition(dioxus_motion::prelude::AnimationConfig::spring(
                SPRING_LAYOUT,
            ))
            .border_radius(8.0),
    );
    rsx! {
        span {
            "data-beui-flip": flip.scope(),
            "aria-hidden": "true",
            style: flip.style_with(""),
            class: cn!(
                "absolute inset-0 -z-10 rounded-lg",
                if destructive { "bg-destructive/10" } else { "bg-foreground/[0.065]" },
            ),
        }
    }
}

/// `ContextMenuItem`.
#[component]
pub fn ContextMenuItem(
    #[props(default = None)] class: Option<String>,
    #[props(default = false)] disabled: bool,
    #[props(default = true)] close_on_select: bool,
    #[props(default = false)] destructive: bool,
    #[props(default = false)] inset: bool,
    #[props(default = None)] text_value: Option<String>,
    #[props(default = None)] on_select: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    rsx! {
        ContextMenuItemBase {
            disabled,
            close_on_select,
            destructive,
            inset,
            text_value,
            class,
            on_select,
            {children}
        }
    }
}

/// `ContextMenuCheckboxItem`.
#[component]
pub fn ContextMenuCheckboxItem(
    checked: bool,
    #[props(default = None)] on_checked_change: Option<EventHandler<bool>>,
    #[props(default = None)] class: Option<String>,
    #[props(default = false)] disabled: bool,
    #[props(default = true)] close_on_select: bool,
    #[props(default = None)] text_value: Option<String>,
    children: Element,
) -> Element {
    let callback = on_checked_change;
    rsx! {
        ContextMenuItemBase {
            role: "menuitemcheckbox",
            aria_checked: checked,
            disabled,
            close_on_select,
            text_value,
            class,
            on_select: move |_| {
                if let Some(callback) = callback {
                    callback.call(!checked);
                }
            },
            span { class: "flex h-4 w-4 shrink-0 items-center justify-center",
                if checked {
                    CheckIcon { class: "h-3.5 w-3.5" }
                }
            }
            {children}
        }
    }
}

/// `ContextMenuRadioGroup`.
#[derive(Clone)]
struct ContextMenuRadioGroupContext {
    value: String,
    on_change: Option<EventHandler<String>>,
}

#[component]
pub fn ContextMenuRadioGroup(
    value: String,
    #[props(default = None)] on_value_change: Option<EventHandler<String>>,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    use_context_provider(|| ContextMenuRadioGroupContext {
        value,
        on_change: on_value_change,
    });
    rsx! {
        div { class: class.clone().unwrap_or_default(), {children} }
    }
}

/// `ContextMenuRadioItem`.
#[component]
pub fn ContextMenuRadioItem(
    value: String,
    #[props(default = None)] class: Option<String>,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] text_value: Option<String>,
    children: Element,
) -> Element {
    let group = use_context::<ContextMenuRadioGroupContext>();
    let checked = group.value == value;
    let on_change = group.on_change;
    rsx! {
        ContextMenuItemBase {
            role: "menuitemradio",
            aria_checked: checked,
            disabled,
            text_value,
            class,
            on_select: move |_| {
                if let Some(callback) = on_change {
                    callback.call(value.clone());
                }
            },
            span { class: "flex h-4 w-4 shrink-0 items-center justify-center",
                span {
                    class: cn!(
                        "h-1.5 w-1.5 rounded-full bg-current transition-opacity",
                        if checked { "opacity-100" } else { "opacity-0" },
                    ),
                }
            }
            {children}
        }
    }
}

/// `ContextMenuLabel`.
#[component]
pub fn ContextMenuLabel(
    #[props(default = false)] inset: bool,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: cn!(
                "px-2.5 pb-1 pt-1.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground",
                if inset { "pl-8" } else { "" },
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `ContextMenuSeparator`.
#[component]
pub fn ContextMenuSeparator(#[props(default = None)] class: Option<String>) -> Element {
    rsx! {
        hr { class: cn!("-mx-1 my-1 h-px border-0 bg-border", class.as_deref().unwrap_or("")) }
    }
}

/// `ContextMenuShortcut`.
#[component]
pub fn ContextMenuShortcut(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        span {
            "aria-hidden": "true",
            class: cn!(
                "ml-auto pl-4 text-[10px] font-medium tracking-wide text-muted-foreground",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}
