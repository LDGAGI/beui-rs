//! Batch 0, on screen: tokens, palettes, dark mode, fonts, utilities.
//!
//! There is no beUI page to mirror for this one — beUI's foundation *is*
//! `app/globals.css` — so the job here is to make every value that file defines
//! visible, next to the control that switches it. If a token is missing from the
//! Tailwind build or a `data-theme` block did not apply, it shows up as a
//! missing swatch or an uncoloured card on this page.

use beui::prelude::*;
use beui_theme::{theme_list, ColorMode, ThemeId};

/// The palette swatches, each one a token `globals.css` defines that has no
/// shadcn equivalent.
const TOKENS: [(&str, &str, &str); 9] = [
    ("background", "bg-background", "oklch(99% 0 0)"),
    ("card", "bg-card", "oklch(97% 0 0)"),
    ("primary", "bg-primary", "#0285f7"),
    ("accent", "bg-accent", "oklch(72% 0.18 195)"),
    ("violet", "bg-violet", "oklch(68% 0.22 295)"),
    ("neon", "bg-neon", "oklch(80% 0.22 145)"),
    ("success", "bg-success", "oklch(70% 0.18 155)"),
    ("warning", "bg-warning", "oklch(78% 0.18 75)"),
    ("destructive", "bg-destructive", "oklch(62% 0.22 25)"),
];

/// `lib/ease.ts`, in the order the file declares them.
const SPRINGS: [(&str, f32, f32, f32); 6] = [
    ("SPRING_PRESS", 500.0, 30.0, 0.6),
    ("SPRING_SWAP", 460.0, 30.0, 0.55),
    ("SPRING_PANEL", 420.0, 40.0, 0.5),
    ("SPRING_LAYOUT", 360.0, 32.0, 0.6),
    ("SPRING_GLIDE", 700.0, 50.0, 0.5),
    ("SPRING_MOUSE", 200.0, 15.0, 0.3),
];

const EASINGS: [(&str, &str, &str); 3] = [
    (
        "EASE_OUT",
        "[0.16, 1, 0.3, 1]",
        "cubic-bezier(0.16, 1, 0.3, 1)",
    ),
    (
        "EASE_IN_OUT",
        "[0.77, 0, 0.175, 1]",
        "cubic-bezier(0.77, 0, 0.175, 1)",
    ),
    (
        "EASE_DRAWER",
        "[0.32, 0.72, 0, 1]",
        "cubic-bezier(0.32, 0.72, 0, 1)",
    ),
];

#[component]
pub fn Foundation() -> Element {
    let dark = beui_theme::use_dark_mode();
    let theme = beui_theme::use_color_theme();
    let reduce = beui_core::hooks::use_reduced_motion();
    let hoverable = beui_core::hooks::use_hover_capable();
    let touchable = beui_core::hooks::use_touch_capable();

    rsx! {
        div { class: "min-h-dvh bg-background font-sans text-foreground",
            div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
                header { class: "flex flex-col gap-3",
                    p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                        "Foundations"
                    }
                    h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                        "beUI tokens, palettes and type"
                    }
                    p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                        "Everything every component draws on: the token layer copied from globals.css, the eleven palettes from lib/themes.ts, the Geist faces and the utility classes."
                    }
                }

                // --- palettes -------------------------------------------------
                section { class: "flex flex-col gap-4",
                    SectionHeading {
                        title: "Color themes",
                        note: "11 palettes, applied as data-theme on <html>. The default palette lives in globals.css and deletes the attribute."
                    }
                    div { class: "flex flex-wrap gap-2",
                        for (id, name, swatch) in theme_list() {
                            ThemeButton {
                                key: "{id.id()}",
                                id,
                                name,
                                swatch,
                                selected: theme.is(id),
                                on_select: move |id: ThemeId| theme.set(id),
                            }
                        }
                    }
                    div { class: "flex flex-wrap items-center gap-2",
                        button {
                            r#type: "button",
                            "data-theme-toggle": "true",
                            class: "press inline-flex h-9 items-center gap-2 rounded-lg border border-border bg-card px-3 text-sm",
                            onclick: move |_| dark.toggle(),
                            if dark.is_dark() { "Switch to light" } else { "Switch to dark" }
                        }
                        button {
                            r#type: "button",
                            "data-theme-system": "true",
                            class: "press inline-flex h-9 items-center rounded-lg border border-border bg-card px-3 text-sm",
                            onclick: move |_| dark.set(ColorMode::System),
                            "Follow the system"
                        }
                        span { class: "text-xs text-muted-foreground",
                            "mode: {dark.mode().id()}, stored in localStorage under the next-themes key"
                        }
                    }
                }

                // --- tokens ---------------------------------------------------
                section { class: "flex flex-col gap-4",
                    SectionHeading {
                        title: "Tokens",
                        note: "Each card is a Tailwind utility resolved through @theme inline, so a missing token shows up here as a transparent card."
                    }
                    div { class: "grid grid-cols-2 gap-3 sm:grid-cols-3", "data-token-grid": "true",
                        for (name, utility, value) in TOKENS {
                            div {
                                key: "{name}",
                                "data-token": "{name}",
                                class: "flex flex-col gap-2 rounded-xl border border-border bg-card p-3",
                                div { "data-token-swatch": "{name}", class: cn!("h-16 w-full rounded-lg border border-border", utility) }
                                div { class: "flex flex-col",
                                    span { class: "text-sm font-medium", "{name}" }
                                    code { class: "font-mono text-[11px] text-muted-foreground", "{value}" }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-wrap gap-3 text-sm",
                        Badge { class: "border-border bg-card", probe: None, value: None, "border" }
                        Badge { class: "border-border-strong bg-card", probe: None, value: None, "border-strong" }
                        Badge { class: "bg-primary text-primary-foreground", probe: None, value: None, "primary" }
                        Badge { class: "bg-accent text-accent-foreground", probe: None, value: None, "accent" }
                        Badge { class: "bg-secondary text-secondary-foreground", probe: None, value: None, "secondary" }
                        Badge { class: "bg-muted text-muted-foreground", probe: None, value: None, "muted" }
                        Badge { class: "bg-destructive text-primary-foreground", probe: None, value: None, "destructive" }
                        Badge { class: "bg-card text-muted-foreground", probe: None, value: None, "card" }
                    }
                }

                // --- surfaces -------------------------------------------------
                section { class: "flex flex-col gap-4",
                    SectionHeading {
                        title: "Surfaces",
                        note: "glass / glass-strong / glass-thin, over the grid-noise utility."
                    }
                    div { class: "grid-noise grid gap-3 rounded-2xl border border-border bg-brand-bg p-6 sm:grid-cols-3",
                        GlassCard { class: "glass", label: "glass", note: "blur(20px) saturate(160%)" }
                        GlassCard { class: "glass-strong", label: "glass-strong", note: "blur(16px)" }
                        GlassCard { class: "glass-thin", label: "glass-thin", note: "blur(12px)" }
                    }
                }

                // --- type -----------------------------------------------------
                section { class: "flex flex-col gap-4",
                    SectionHeading {
                        title: "Type",
                        note: "Geist and Geist Mono, self-hosted from the same woff2 files next/font/google serves."
                    }
                    div { class: "flex flex-col gap-3 rounded-xl border border-border bg-card p-5",
                        p { class: "text-2xl font-semibold tracking-tight", "The quick brown fox jumps over the lazy dog" }
                        p { class: "text-pretty text-sm leading-6 text-muted-foreground",
                            "Geist Sans, 100–900. text-pretty and text-balance come from the utilities layer."
                        }
                        code { class: "font-mono text-sm text-muted-foreground",
                            "const tokens = {{ easeOut: [0.16, 1, 0.3, 1] }};"
                        }
                    }
                }

                // --- utilities ------------------------------------------------
                section { class: "flex flex-col gap-4",
                    SectionHeading {
                        title: "Utilities",
                        note: "The marquee keyframes, the press transition and the radial mask, all from globals.css."
                    }
                    div { class: "scrollbar-hide relative flex overflow-hidden rounded-xl border border-border bg-card py-3",
                        div { class: "flex shrink-0 animate-marquee items-center gap-4 pr-4",
                            for i in 0..8 {
                                span { key: "a{i}", class: "whitespace-nowrap text-sm text-muted-foreground",
                                    "animate-marquee · beUI · {i}"
                                }
                            }
                        }
                        div { class: "flex shrink-0 animate-marquee items-center gap-4 pr-4", aria_hidden: "true",
                            for i in 0..8 {
                                span { key: "b{i}", class: "whitespace-nowrap text-sm text-muted-foreground",
                                    "animate-marquee · beUI · {i}"
                                }
                            }
                        }
                    }
                    div { class: "mask-radial-fade grid h-32 place-items-center rounded-xl border border-border bg-brand-accent",
                        span { class: "text-sm font-medium text-primary-foreground", "mask-radial-fade" }
                    }
                }

                // --- motion tokens --------------------------------------------
                section { class: "flex flex-col gap-4",
                    SectionHeading {
                        title: "Motion tokens",
                        note: "lib/ease.ts, copied verbatim into beui-core::ease. Springs integrate the same ODE; the curves run through motion's own cubic-bezier solver."
                    }
                    div { class: "grid gap-3 sm:grid-cols-2",
                        div { class: "rounded-xl border border-border bg-card p-4",
                            h3 { class: "mb-3 text-sm font-medium", "Springs" }
                            dl { class: "flex flex-col gap-2 font-mono text-[11px] text-muted-foreground",
                                for (name, stiffness, damping, mass) in SPRINGS {
                                    div { key: "{name}", class: "flex items-baseline justify-between gap-4",
                                        dt { "{name}" }
                                        dd { "stiffness {stiffness} · damping {damping} · mass {mass}" }
                                    }
                                }
                            }
                        }
                        div { class: "rounded-xl border border-border bg-card p-4",
                            h3 { class: "mb-3 text-sm font-medium", "Easing" }
                            dl { class: "flex flex-col gap-2 font-mono text-[11px] text-muted-foreground",
                                for (name, tuple, css) in EASINGS {
                                    div { key: "{name}", class: "flex flex-col",
                                        dt { "{name} = {tuple}" }
                                        dd { "{css}" }
                                    }
                                }
                            }
                        }
                    }
                }

                // --- capability hooks -----------------------------------------
                section { class: "flex flex-col gap-4",
                    SectionHeading {
                        title: "Capability hooks",
                        note: "use_reduced_motion, use_hover_capable and use_touch_capable, read straight from the media queries the source uses."
                    }
                    div { class: "flex flex-wrap gap-2 text-sm",
                        Badge {
                            class: if reduce() { "bg-success text-primary-foreground" } else { "border-border bg-card text-muted-foreground" },
                            value: reduce(),
                            probe: "reduce",
                            "prefers-reduced-motion: {reduce()}"
                        }
                        Badge {
                            class: if hoverable() { "bg-success text-primary-foreground" } else { "border-border bg-card text-muted-foreground" },
                            value: hoverable(),
                            probe: "hover",
                            "hover: hover + pointer: fine: {hoverable()}"
                        }
                        Badge {
                            class: if touchable() { "bg-success text-primary-foreground" } else { "border-border bg-card text-muted-foreground" },
                            value: touchable(),
                            probe: "touch",
                            "any-pointer: coarse or maxTouchPoints: {touchable()}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SectionHeading(title: String, note: String) -> Element {
    rsx! {
        div { class: "flex flex-col gap-1",
            h2 { class: "text-lg font-semibold tracking-tight", "{title}" }
            p { class: "max-w-3xl text-sm text-muted-foreground", "{note}" }
        }
    }
}

#[component]
fn Badge(
    class: Option<String>,
    /// Set on the capability probes so the verification script can read the hook
    /// result out of the DOM.
    probe: Option<&'static str>,
    value: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        span {
            class: cn!("inline-flex items-center rounded-full border border-transparent px-2.5 py-1 text-xs font-medium", class),
            "data-probe": probe,
            "data-probe-value": value.map(|value| value.to_string()),
            {children}
        }
    }
}

#[component]
fn GlassCard(class: &'static str, label: &'static str, note: &'static str) -> Element {
    rsx! {
        div { class: cn!("flex flex-col gap-1 rounded-xl p-4", class),
            span { class: "text-sm font-medium", "{label}" }
            span { class: "font-mono text-[11px] text-muted-foreground", "{note}" }
        }
    }
}

#[component]
fn ThemeButton(
    id: ThemeId,
    name: &'static str,
    swatch: &'static str,
    selected: bool,
    on_select: EventHandler<ThemeId>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            "data-palette": "{id.id()}",
            "aria-pressed": if selected { "true" } else { "false" },
            class: cn!(
                "press flex items-center gap-2 rounded-lg border px-3 py-2 text-xs",
                if selected { "border-primary bg-card" } else { "border-border bg-card text-muted-foreground" },
            ),
            onclick: move |_| on_select.call(id),
            span { class: "size-3 shrink-0 rounded-full border border-border", style: "background:{swatch}" }
            span { "{name}" }
            span { class: "font-mono text-[10px] text-muted-foreground", "{id.id()}" }
        }
    }
}
