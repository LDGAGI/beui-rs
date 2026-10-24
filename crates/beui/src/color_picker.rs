//! `motion/color-picker.tsx` — the color picker and its parts.
//!
//! The colour maths is ported verbatim (`hsvaToRgba` / `rgbaToHsva` / `parseHex`
//! / `formatHex`); the root owns the HSV state and hands it down through
//! context, and every part drags on a shared 2D rail.
//!
//! Differences from the source : the panel is
//! anchored inside the root rather than viewport-positioned through a portal,
//! and the loupe / 3D thumb tilt / morphing area and the panel's staggered
//! entrance are not ported. The eyedropper button is present but does not call
//! the EyeDropper API.

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::ease::SPRING_PRESS;
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

use crate::icons::IconPipette;
use crate::motion::{animate_number, number, scale_style, spring, tween, use_number_motion, Ease};
use keyboard_types::{Key, Modifiers};

// --- colour maths (verbatim from the source) --------------------------------

/// Hue 0-360, saturation/value 0-100, alpha 0-1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsva {
    pub h: f64,
    pub s: f64,
    pub v: f64,
    pub a: f64,
}

/// Channels 0-255, alpha 0-1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

const BLACK: Rgba = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

fn clamp(value: f64, lo: f64, hi: f64) -> f64 {
    value.min(hi).max(lo)
}

fn round_to(value: f64, places: i32) -> f64 {
    let f = 10f64.powi(places);
    (value * f).round() / f
}

/// `hsvaToRgba`.
pub fn hsva_to_rgba(hsva: Hsva) -> Rgba {
    let sat = hsva.s / 100.0;
    let val = hsva.v / 100.0;
    let f = |n: f64| {
        let k = (n + hsva.h / 60.0) % 6.0;
        val - val * sat * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    Rgba {
        r: (f(5.0) * 255.0).round(),
        g: (f(3.0) * 255.0).round(),
        b: (f(1.0) * 255.0).round(),
        a: hsva.a,
    }
}

/// `rgbaToHsva`; `previous` keeps hue for greys and saturation for black.
pub fn rgba_to_hsva(rgba: Rgba, previous: Option<Hsva>) -> Hsva {
    let rn = rgba.r / 255.0;
    let gn = rgba.g / 255.0;
    let bn = rgba.b / 255.0;
    let max = rn.max(gn).max(bn);
    let d = max - rn.min(gn).min(bn);
    let mut h = previous.map(|p| p.h).unwrap_or(0.0);
    if d != 0.0 {
        if max == rn {
            h = ((gn - bn) / d) % 6.0;
        } else if max == gn {
            h = (bn - rn) / d + 2.0;
        } else {
            h = (rn - gn) / d + 4.0;
        }
        h = (h * 60.0 + 360.0) % 360.0;
    }
    let v = max * 100.0;
    let s = if max != 0.0 {
        (d / max) * 100.0
    } else {
        previous.map(|p| p.s).unwrap_or(0.0)
    };
    Hsva { h, s, v, a: rgba.a }
}

/// `parseHex`: `#rgb`, `#rgba`, `#rrggbb` and `#rrggbbaa`.
pub fn parse_hex(input: &str) -> Option<Rgba> {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if !(body.len() == 3 || body.len() == 4 || body.len() == 6 || body.len() == 8) {
        return None;
    }
    if !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let expanded: String = if body.len() <= 4 {
        body.chars().flat_map(|c| [c, c]).collect()
    } else {
        body.to_string()
    };
    let n = |i: usize| {
        u8::from_str_radix(&expanded[i..i + 2], 16)
            .ok()
            .map(f64::from)
    };
    let a = if expanded.len() == 8 {
        round_to(n(6)? / 255.0, 2)
    } else {
        1.0
    };
    Some(Rgba {
        r: n(0)?,
        g: n(2)?,
        b: n(4)?,
        a,
    })
}

/// `formatHex`: lowercase `#rrggbb`, or `#rrggbbaa` when translucent.
pub fn format_hex(rgba: Rgba) -> String {
    let pair = |n: f64| format!("{:02x}", n.round().clamp(0.0, 255.0) as u8);
    let base = format!("#{}{}{}", pair(rgba.r), pair(rgba.g), pair(rgba.b));
    if rgba.a < 1.0 {
        format!("{base}{}", pair(rgba.a * 255.0))
    } else {
        base
    }
}

// --- context ----------------------------------------------------------------

#[derive(Clone, Copy)]
struct PickerContext {
    hsva: Signal<Hsva>,
    alpha: bool,
    disabled: bool,
    on_change: Option<EventHandler<String>>,
    on_commit: Option<EventHandler<String>>,
    open: Signal<bool>,
}

impl PickerContext {
    fn hex(&self) -> String {
        format_hex(hsva_to_rgba((self.hsva)()))
    }

    fn update(&self, next: Hsva) {
        let clean = Hsva {
            h: clamp(next.h, 0.0, 360.0),
            s: clamp(next.s, 0.0, 100.0),
            v: clamp(next.v, 0.0, 100.0),
            a: if self.alpha {
                clamp(next.a, 0.0, 1.0)
            } else {
                1.0
            },
        };
        let mut hsva = self.hsva;
        hsva.set(clean);
        if let Some(on_change) = self.on_change {
            on_change.call(format_hex(hsva_to_rgba(clean)));
        }
    }

    fn commit(&self) {
        if let Some(on_commit) = self.on_commit {
            on_commit.call(self.hex());
        }
    }
}

/// The panel has one identity; the key must be a formatted string in `rsx!`.
const PANEL_KEY: &str = "panel";

/// Give it an `aria_label`, or a legend through children.
#[component]
pub fn ColorPicker(
    #[props(default)] value: Option<String>,
    #[props(default = "#3478f6".to_string())] default_value: String,
    #[props(default)] on_value_change: Option<EventHandler<String>>,
    #[props(default)] on_value_commit: Option<EventHandler<String>>,
    #[props(default = true)] alpha: bool,
    #[props(default)] disabled: bool,
    #[props(default)] name: Option<String>,
    #[props(default)] open: Option<bool>,
    #[props(default)] default_open: bool,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let root_id = use_hook(platform::next_scope_id);
    let controlled = value.is_some();
    let incoming = value.clone().unwrap_or_else(|| default_value.clone());
    let parsed = parse_hex(&incoming).unwrap_or(BLACK);
    let hsva = use_signal(|| rgba_to_hsva(parsed, None));

    // Mirror an external value that we did not emit ourselves.
    use_effect(use_reactive!(|value| {
        if let Some(value) = value {
            if let Some(parsed) = parse_hex(&value) {
                let previous = *hsva.peek();
                if format_hex(hsva_to_rgba(previous)) != value.to_lowercase() {
                    let mut hsva = hsva;
                    hsva.set(rgba_to_hsva(parsed, Some(previous)));
                }
            }
        }
    }));

    let internal_open = use_signal(|| default_open);
    let open_signal = use_signal(|| open.unwrap_or(default_open));
    use_effect(use_reactive!(|open| {
        if let Some(open) = open {
            let mut signal = open_signal;
            signal.set(open);
        }
    }));
    let _ = internal_open;
    let _ = on_open_change;

    let context = PickerContext {
        hsva,
        alpha,
        disabled,
        on_change: on_value_change,
        on_commit: on_value_commit,
        open: open_signal,
    };
    use_context_provider(|| context);

    rsx! {
        fieldset {
            "data-color-picker-root": "{root_id}",
            "aria-label": aria_label,
            disabled,
            class: cn!(
                "relative flex w-full min-w-0 max-w-72 flex-col gap-3 border-0 p-0",
                if controlled { "w-fit self-start" } else { "" },
                if disabled { "opacity-50" } else { "" },
                class,
            ),
            onmousedown: move |event| {
                // Keep the value out of the root's own dismissal.
                let _ = event;
            },
            {children}
            if let Some(name) = name {
                input { r#type: "hidden", name, value: "{context.hex()}" }
            }
        }
    }
}

#[component]
pub fn ColorPickerTrigger(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<PickerContext>();
    let hex = context.hex();
    let is_open = (context.open)();

    rsx! {
        button {
            r#type: "button",
            "data-color-picker-trigger": "true",
            "aria-haspopup": "dialog",
            "aria-expanded": if is_open { "true" } else { "false" },
            disabled: context.disabled,
            class: cn!(
                "inline-flex w-fit items-center gap-2 rounded-lg border border-border bg-background px-2 py-1 text-sm outline-none transition-colors hover:border-border-strong focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            onclick: move |_| {
                let mut open = context.open;
                open.toggle();
            },
            span {
                class: "size-5 shrink-0 rounded-md border border-border",
                style: "background-color:{hex}",
            }
            span { class: "font-mono text-xs text-muted-foreground", "{hex}" }
        }
    }
}

#[component]
pub fn ColorPickerContent(
    #[props(default)] aria_label: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_context::<PickerContext>();
    let reduce = use_reduced_motion()();
    let panels: Vec<()> = if (context.open)() {
        vec![()]
    } else {
        Vec::new()
    };

    rsx! {
        AnimatePresence {
            for _ in panels {
                ColorPickerPanel { key: "{PANEL_KEY}", aria_label: aria_label.clone(), class: class.clone(), reduce, children: children.clone() }
            }
        }
    }
}

#[component]
fn ColorPickerPanel(
    #[props(default)] aria_label: Option<String>,
    #[props(default)] class: Option<String>,
    reduce: bool,
    children: Element,
) -> Element {
    let gate = use_gate();
    let config = if reduce {
        PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        )
    } else {
        PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0, scale: 0.96, y: -4.0 },
            motion_style! { opacity: 1.0, scale: 1.0, y: 0.0 },
            motion_style! { opacity: 0.0, scale: 0.96, y: -4.0 },
            spring(beui_core::ease::SPRING_PANEL),
            tween(140, Ease::Out),
        )
    };
    let style = use_presence_style(config).expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        div {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            "aria-label": aria_label,
            class: cn!(
                "absolute left-0 top-full z-50 mt-2 flex w-64 origin-top flex-col gap-3 rounded-2xl border border-border bg-background p-3 shadow-xl",
                class,
            ),
            {children}
        }
    }
}

// --- area -------------------------------------------------------------------

#[component]
pub fn ColorPickerArea(
    #[props(default)] label: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let context = use_context::<PickerContext>();
    let _ = &label;
    let mut rect = use_signal(|| None::<beui_core::platform::Rect>);
    let mut dragging = use_signal(|| false);
    let hsva = (context.hsva)();

    let hue_color = format!("hsl({} 100% 50%)", hsva.h);

    rsx! {
        div {
            role: "slider",
            tabindex: if context.disabled { -1 } else { 0 },
            "aria-label": label.unwrap_or_else(|| "Saturation and brightness".to_string()),
            "aria-valuetext": "Saturation {hsva.s}%, brightness {hsva.v}%",
            class: cn!(
                "relative h-40 w-full cursor-crosshair touch-none overflow-hidden rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            style: "background-image:linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, {hue_color})",
            onmounted: move |data| {
                spawn(async move {
                    if let Some(measured) = platform::client_rect(&data).await {
                        rect.set(Some(measured));
                    }
                });
            },
            onpointerdown: move |event| {
                if context.disabled {
                    return;
                }
                event.prevent_default();
                dragging.set(true);
                platform::capture_pointer(&event);
                if let Some(measured) = *rect.peek() {
                    let point = event.client_coordinates();
                    let s = clamp((point.x - measured.left) / measured.width, 0.0, 1.0) * 100.0;
                    let v = (1.0 - clamp((point.y - measured.top) / measured.height, 0.0, 1.0)) * 100.0;
                    let current = (context.hsva)();
                    context.update(Hsva { s, v, ..current });
                }
            },
            onpointermove: move |event| {
                if !dragging() || context.disabled {
                    return;
                }
                if let Some(measured) = *rect.peek() {
                    let point = event.client_coordinates();
                    let s = clamp((point.x - measured.left) / measured.width, 0.0, 1.0) * 100.0;
                    let v = (1.0 - clamp((point.y - measured.top) / measured.height, 0.0, 1.0)) * 100.0;
                    let current = (context.hsva)();
                    context.update(Hsva { s, v, ..current });
                }
            },
            onpointerup: move |event| {
                platform::release_pointer(&event);
                dragging.set(false);
                context.commit();
            },
            onpointercancel: move |event| {
                platform::release_pointer(&event);
                dragging.set(false);
            },
            onkeydown: move |event| {
                let step = if event.modifiers().contains(Modifiers::SHIFT) { 10.0 } else { 1.0 };
                let current = (context.hsva)();
                let next = match event.key() {
                    Key::ArrowRight => Hsva { s: current.s + step, ..current },
                    Key::ArrowLeft => Hsva { s: current.s - step, ..current },
                    Key::ArrowUp => Hsva { v: current.v + step, ..current },
                    Key::ArrowDown => Hsva { v: current.v - step, ..current },
                    _ => return,
                };
                event.prevent_default();
                context.update(next);
                context.commit();
            },
            span {
                class: "pointer-events-none absolute size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.35)]",
                style: "left:{hsva.s}%;top:{100.0 - hsva.v}%;background-color:{hue_color}",
            }
        }
    }
}

// --- channel sliders --------------------------------------------------------

#[component]
pub fn ColorPickerHue(
    #[props(default)] label: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        ChannelSlider {
            kind: ChannelKind::Hue,
            label: label.unwrap_or_else(|| "Hue".to_string()),
            class: class,
        }
    }
}

#[component]
pub fn ColorPickerAlpha(
    #[props(default)] label: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        ChannelSlider {
            kind: ChannelKind::Alpha,
            label: label.unwrap_or_else(|| "Opacity".to_string()),
            class: class,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChannelKind {
    Hue,
    Alpha,
}

#[component]
fn ChannelSlider(
    kind: ChannelKind,
    label: String,
    #[props(default)] class: Option<String>,
) -> Element {
    let context = use_context::<PickerContext>();
    let mut rect = use_signal(|| None::<beui_core::platform::Rect>);
    let mut dragging = use_signal(|| false);
    let hsva = (context.hsva)();
    let hex = context.hex();

    let (value, rail) = match kind {
        ChannelKind::Hue => (
            hsva.h / 360.0,
            "linear-gradient(to right, #f00 0%, #ff0 17%, #0f0 33%, #0ff 50%, #00f 67%, #f0f 83%, #f00 100%)".to_string(),
        ),
        ChannelKind::Alpha => (
            hsva.a,
            format!("linear-gradient(to right, transparent, {hex})"),
        ),
    };

    rsx! {
        div {
            role: "slider",
            tabindex: if context.disabled { -1 } else { 0 },
            "aria-label": label,
            "aria-valuenow": "{value}",
            class: cn!(
                "relative h-3 w-full cursor-pointer touch-none rounded-full outline-none focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            style: "background-image:{rail}",
            onmounted: move |data| {
                spawn(async move {
                    if let Some(measured) = platform::client_rect(&data).await {
                        rect.set(Some(measured));
                    }
                });
            },
            onpointerdown: move |event| {
                if context.disabled {
                    return;
                }
                event.prevent_default();
                dragging.set(true);
                platform::capture_pointer(&event);
                apply_channel(&context, kind, &rect, &event);
            },
            onpointermove: move |event| {
                if !dragging() || context.disabled {
                    return;
                }
                apply_channel(&context, kind, &rect, &event);
            },
            onpointerup: move |event| {
                platform::release_pointer(&event);
                dragging.set(false);
                context.commit();
            },
            onpointercancel: move |event| {
                platform::release_pointer(&event);
                dragging.set(false);
            },
            span {
                class: "pointer-events-none absolute top-1/2 size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.35)]",
                style: "left:{value * 100.0}%;background-color:{hex}",
            }
        }
    }
}

fn apply_channel(
    context: &PickerContext,
    kind: ChannelKind,
    rect: &Signal<Option<beui_core::platform::Rect>>,
    event: &Event<PointerData>,
) {
    let Some(measured) = *rect.peek() else {
        return;
    };
    let ratio = clamp(
        (event.client_coordinates().x - measured.left) / measured.width,
        0.0,
        1.0,
    );
    let current = (context.hsva)();
    let next = match kind {
        ChannelKind::Hue => Hsva {
            h: ratio * 360.0,
            ..current
        },
        ChannelKind::Alpha => Hsva {
            a: ratio,
            ..current
        },
    };
    context.update(next);
}

// --- swatch / hex / channels ------------------------------------------------

#[component]
pub fn ColorPickerSwatch(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<PickerContext>();
    rsx! {
        span {
            class: cn!("size-9 shrink-0 rounded-lg border border-border", class),
            style: "background-color:{context.hex()}",
        }
    }
}

#[component]
pub fn ColorPickerHexInput(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<PickerContext>();
    let hex = context.hex();
    let mut draft = use_signal(|| hex.clone());

    // Follow the live color unless the field itself is being edited.
    let live = hex.clone();
    use_effect(use_reactive!(|live| {
        let mut draft = draft;
        if parse_hex(&draft.peek().clone()).map(format_hex).as_deref() != Some(live.as_str()) {
            draft.set(live.clone());
        }
    }));

    rsx! {
        input {
            r#type: "text",
            value: "{draft()}",
            "aria-label": "Hex color",
            spellcheck: "false",
            class: cn!(
                "h-9 w-full min-w-0 rounded-lg border border-border bg-background px-2 font-mono text-xs uppercase outline-none focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            oninput: move |event| {
                let next = event.value();
                draft.set(next.clone());
                if let Some(parsed) = parse_hex(&next) {
                    let current = (context.hsva)();
                    context.update(rgba_to_hsva(parsed, Some(current)));
                }
            },
            onblur: move |_| context.commit(),
        }
    }
}

/// `rgb` or `hsl`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChannelFormat {
    Rgb,
    Hsl,
}

#[component]
pub fn ColorPickerChannels(
    #[props(default)] format: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let context = use_context::<PickerContext>();
    let rgba = hsva_to_rgba((context.hsva)());
    let format = match format.as_deref() {
        Some("hsl") => ChannelFormat::Hsl,
        _ => ChannelFormat::Rgb,
    };
    let (a, b, c) = match format {
        ChannelFormat::Rgb => (
            rgba.r.round().to_string(),
            rgba.g.round().to_string(),
            rgba.b.round().to_string(),
        ),
        ChannelFormat::Hsl => {
            let hsva = (context.hsva)();
            (
                format!("{}", hsva.h.round()),
                format!("{}", hsva.s.round()),
                format!("{}", hsva.v.round()),
            )
        }
    };

    rsx! {
        div { class: cn!("flex min-w-0 flex-1 gap-1.5", class),
            ChannelField { value: a, channel: 0, format }
            ChannelField { value: b, channel: 1, format }
            ChannelField { value: c, channel: 2, format }
        }
    }
}

#[component]
fn ChannelField(value: String, channel: u8, format: ChannelFormat) -> Element {
    let context = use_context::<PickerContext>();
    rsx! {
        input {
            r#type: "text",
            value: "{value}",
            inputmode: "numeric",
            class: "h-9 w-full min-w-0 rounded-lg border border-border bg-background px-2 text-center font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring",
            onchange: move |event| {
                let Ok(parsed) = event.value().parse::<f64>() else {
                    return;
                };
                let current = (context.hsva)();
                let next = match format {
                    ChannelFormat::Rgb => {
                        let rgba = hsva_to_rgba(current);
                        let rgba = match channel {
                            0 => Rgba { r: parsed, ..rgba },
                            1 => Rgba { g: parsed, ..rgba },
                            _ => Rgba { b: parsed, ..rgba },
                        };
                        rgba_to_hsva(rgba, Some(current))
                    }
                    ChannelFormat::Hsl => match channel {
                        0 => Hsva { h: parsed, ..current },
                        1 => Hsva { s: parsed, ..current },
                        _ => Hsva { v: parsed, ..current },
                    },
                };
                context.update(next);
                context.commit();
            },
        }
    }
}

// --- eyedropper / presets ---------------------------------------------------

#[component]
pub fn ColorPickerEyeDropper(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<PickerContext>();
    rsx! {
        button {
            r#type: "button",
            "aria-label": "Pick a color from the screen",
            disabled: context.disabled,
            class: cn!(
                "grid size-9 shrink-0 place-items-center rounded-lg border border-border bg-background text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            IconPipette { class: "size-4" }
        }
    }
}

#[component]
pub fn ColorPickerPresets(#[props(default)] class: Option<String>, children: Element) -> Element {
    rsx! {
        div { class: cn!("flex flex-wrap items-center gap-1.5", class), {children} }
    }
}

#[component]
pub fn ColorPickerPreset(
    value: String,
    label: String,
    #[props(default)] class: Option<String>,
) -> Element {
    let context = use_context::<PickerContext>();
    let mut scale = use_number_motion(1.0);
    let reduced = use_reduced_motion()();

    rsx! {
        button {
            r#type: "button",
            "aria-label": label,
            disabled: context.disabled,
            class: cn!("size-6 shrink-0 rounded-full border border-border outline-none focus-visible:ring-2 focus-visible:ring-ring", class),
            style: "background-color:{value};{scale_style(number(&scale))}",
            onpointerdown: move |_| {
                if !reduced {
                    animate_number(&mut scale, 0.9, spring(SPRING_PRESS));
                }
            },
            onpointerup: move |_| {
                animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
            },
            onpointercancel: move |_| {
                animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
            },
            onclick: move |_| {
                if let Some(parsed) = parse_hex(&value) {
                    let current = (context.hsva)();
                    context.update(rgba_to_hsva(parsed, Some(current)));
                    context.commit();
                }
            },
        }
    }
}
