//! `motion/loader.tsx` — the loader set.
//!
//! Every variant is a loop. beUI expresses those as motion keyframes with
//! `repeat: Infinity`; the port drives one linear `0 → 1` phase per variant
//! ([`use_loop_phase`](crate::motion::use_loop_phase)) and evaluates each
//! keyframe track against it every render, so the curve is the same one motion
//! would draw (see [`track`](crate::motion::track)) and no completion callback
//! is needed. Per-dot `delay` becomes a phase offset.
//!
//! Reduced motion keeps the calm opacity pulse the source defines, with every
//! transform dropped.

use std::f64::consts::PI;

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::ease::ease_in_out;
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;

use crate::motion::{number, track, track_at, track_offset, use_loop_phase};

/// `LoaderVariant`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum LoaderVariant {
    #[default]
    Spinner,
    Dots,
    Bars,
    DotMatrix,
    Dither,
    Ascii,
    AsciiLine,
    AsciiBraille,
    AsciiBlocks,
    AsciiBounce,
    Morph,
    Comet,
    Scramble,
    Metaballs,
    Newton,
    Helix,
    Percent,
}

const ASCII_SETS: [(LoaderVariant, &[&str]); 5] = [
    (
        LoaderVariant::Ascii,
        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
    ),
    (LoaderVariant::AsciiLine, &["|", "/", "-", "\\"]),
    (
        LoaderVariant::AsciiBraille,
        &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
    ),
    (
        LoaderVariant::AsciiBlocks,
        &[
            "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂",
        ],
    ),
    (
        LoaderVariant::AsciiBounce,
        &["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"],
    ),
];

#[component]
pub fn Loader(
    #[props(default)] variant: LoaderVariant,
    #[props(default = 32.0)] size: f32,
    #[props(default = 1.0)] speed: f32,
    #[props(default = "Loading".to_string())] label: String,
    #[props(default)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();

    rsx! {
        span {
            role: "status",
            "aria-label": label,
            class: cn!("inline-flex items-center justify-center text-foreground", class),
            match variant {
                LoaderVariant::Spinner => rsx! { Spinner { size, speed, reduce } },
                LoaderVariant::Dots => rsx! { Dots { size, speed, reduce } },
                LoaderVariant::Bars => rsx! { Bars { size, speed, reduce } },
                LoaderVariant::DotMatrix => rsx! { DotMatrix { size, speed, reduce } },
                LoaderVariant::Dither => rsx! { Dither { size, speed, reduce } },
                LoaderVariant::Morph => rsx! { Morph { size, speed, reduce } },
                LoaderVariant::Comet => rsx! { Comet { size, speed, reduce } },
                LoaderVariant::Scramble => rsx! { Scramble { size, speed, reduce } },
                LoaderVariant::Metaballs => rsx! { Metaballs { size, speed, reduce } },
                LoaderVariant::Newton => rsx! { Newton { size, speed, reduce } },
                LoaderVariant::Helix => rsx! { Helix { size, speed, reduce } },
                LoaderVariant::Percent => rsx! { Percent { size, speed, reduce } },
                _ => rsx! { Ascii { variant, size, speed, reduce } },
            }
            span { class: "sr-only", "{label}" }
        }
    }
}

fn ms(speed: f32) -> u64 {
    (f64::from(speed) * 1000.0).max(1.0) as u64
}

// --- spinner ----------------------------------------------------------------

#[component]
fn Spinner(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase(if reduce { 1400 } else { ms(speed) });
    let stroke = 2.0f32.max(size * 0.09);
    let r = (size - stroke) / 2.0;
    let (rotate, opacity) = if reduce {
        (0.0, track(&[1.0, 0.4, 1.0], number(&phase), ease_in_out))
    } else {
        (360.0 * number(&phase), 1.0)
    };

    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box: "0 0 {size} {size}",
            style: "opacity:{opacity};transform:rotate({rotate}deg)",
            circle {
                cx: "{size / 2.0}",
                cy: "{size / 2.0}",
                r: "{r}",
                fill: "none",
                stroke: "currentColor",
                stroke_opacity: "0.2",
                stroke_width: "{stroke}",
            }
            path {
                d: "M {size / 2.0} {size / 2.0 - r} A {r} {r} 0 0 1 {size / 2.0 + r} {size / 2.0}",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "{stroke}",
                stroke_linecap: "round",
            }
        }
    }
}

// --- dots -------------------------------------------------------------------

#[component]
fn Dots(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase(ms(speed));
    let dot = size * 0.24;
    let offset = 0.16;

    rsx! {
        span { class: "flex items-center", style: "gap:{size * 0.14}px",
            for i in 0..3 {
                span {
                    key: "{i}",
                    class: "rounded-full bg-current",
                    style: {
                        let phase = number(&phase);
                        let offset = i as f32 * offset;
                        if reduce {
                            let opacity = track_offset(&[0.4, 1.0, 0.4], phase, offset, ease_in_out);
                            format!("width:{dot}px;height:{dot}px;opacity:{opacity}")
                        } else {
                            let y = track_offset(&[0.0, -size * 0.3, 0.0], phase, offset, ease_in_out);
                            let opacity = track_offset(&[0.5, 1.0, 0.5], phase, offset, ease_in_out);
                            format!("width:{dot}px;height:{dot}px;opacity:{opacity};transform:translateY({y}px)")
                        }
                    },
                }
            }
        }
    }
}

// --- bars -------------------------------------------------------------------

#[component]
fn Bars(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase(ms(speed));
    let bar = size * 0.16;
    rsx! {
        span { class: "flex items-center", style: "gap:{size * 0.1}px;height:{size}px",
            for i in 0..4 {
                span {
                    key: "{i}",
                    class: "rounded-full bg-current",
                    style: {
                        let phase = number(&phase);
                        let offset = i as f32 * 0.12;
                        if reduce {
                            let opacity = track_offset(&[0.4, 1.0, 0.4], phase, offset, ease_in_out);
                            format!("width:{bar}px;height:{size}px;opacity:{opacity}")
                        } else {
                            let scale_y = track_offset(&[0.3, 1.0, 0.3], phase, offset, ease_in_out);
                            format!("width:{bar}px;height:{size}px;transform-origin:center bottom;transform:scaleY({scale_y})")
                        }
                    },
                }
            }
        }
    }
}

// --- dot matrix -------------------------------------------------------------

#[component]
fn DotMatrix(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase(ms(speed));
    let n = 3usize;
    let gap = size * 0.14;
    let dot = (size - gap * (n as f32 - 1.0)) / n as f32;

    rsx! {
        span {
            class: "grid",
            style: "gap:{gap}px;grid-template-columns:repeat({n}, {dot}px)",
            for idx in 0..(n * n) {
                span {
                    key: "{idx}",
                    class: "rounded-full bg-current",
                    style: {
                        let phase = number(&phase);
                        let x = (idx % n) as f32;
                        let y = (idx / n) as f32;
                        let offset = (x + y) / (2.0 * (n as f32 - 1.0));
                        let opacity = track_offset(&[0.2, 1.0, 0.2], phase, offset, ease_in_out);
                        if reduce {
                            let opacity = track_offset(&[0.3, 1.0, 0.3], phase, offset, ease_in_out);
                            format!("width:{dot}px;height:{dot}px;opacity:{opacity}")
                        } else {
                            let scale = track_offset(&[0.7, 1.0, 0.7], phase, offset, ease_in_out);
                            format!("width:{dot}px;height:{dot}px;opacity:{opacity};transform:scale({scale})")
                        }
                    },
                }
            }
        }
    }
}

// --- dither -----------------------------------------------------------------

const BAYER_4: [usize; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

#[component]
fn Dither(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase(ms(speed));
    let n = 4usize;
    let gap = 1.0f32.max(size * 0.05);
    let cell = (size - gap * (n as f32 - 1.0)) / n as f32;

    rsx! {
        span {
            class: "grid",
            style: "gap:{gap}px;grid-template-columns:repeat({n}, {cell}px)",
            for (idx, order) in BAYER_4.iter().enumerate() {
                span {
                    key: "{idx}",
                    class: "bg-current",
                    style: {
                        let phase = number(&phase);
                        let offset = *order as f32 / BAYER_4.len() as f32;
                        let opacity = if reduce {
                            track_offset(&[0.3, 1.0, 0.3], phase, offset, ease_in_out)
                        } else {
                            track_offset(&[0.1, 1.0, 0.1], phase, offset, ease_in_out)
                        };
                        format!("width:{cell}px;height:{cell}px;opacity:{opacity}")
                    },
                }
            }
        }
    }
}

// --- ascii ------------------------------------------------------------------

#[component]
fn Ascii(variant: LoaderVariant, size: f32, speed: f32, reduce: bool) -> Element {
    let frames = ASCII_SETS
        .iter()
        .find(|(candidate, _)| *candidate == variant)
        .map(|(_, frames)| *frames)
        .unwrap_or(&["⠋"]);
    // Reduced motion slows the cycle rather than stopping it — it is a glyph
    // swap, not on-screen movement.
    let step = (if reduce { speed * 2.5 } else { speed }) / frames.len() as f32;
    let phase = use_loop_phase(((step * frames.len() as f32) * 1000.0).max(1.0) as u64);
    let frame = ((number(&phase) * frames.len() as f32) as usize) % frames.len();

    rsx! {
        span {
            class: "font-mono leading-none tabular-nums",
            style: "font-size:{size}px;line-height:1",
            "{frames[frame]}"
        }
    }
}

// --- morph ------------------------------------------------------------------

fn ngon_radius(ang: f64, n: i32, phase: f64) -> f64 {
    let seg = 2.0 * PI / f64::from(n);
    let a = ang - phase;
    let local = ((a % seg) + seg) % seg - seg / 2.0;
    (PI / f64::from(n)).cos() / local.cos()
}

fn morph_points(radius_at: impl Fn(f64) -> f64) -> Vec<(f32, f32)> {
    (0..24)
        .map(|i| {
            let ang = (f64::from(i) / 24.0) * 2.0 * PI - PI / 2.0;
            let r = radius_at(ang).min(1.05);
            (
                (50.0 + ang.cos() * 46.0 * r) as f32,
                (50.0 + ang.sin() * 46.0 * r) as f32,
            )
        })
        .collect()
}

fn morph_paths() -> Vec<Vec<(f32, f32)>> {
    vec![
        morph_points(|_| 1.0),
        morph_points(|a| ngon_radius(a, 4, PI / 4.0)),
        morph_points(|a| ngon_radius(a, 3, 0.0)),
        morph_points(|a| ngon_radius(a, 6, 0.0)),
        morph_points(|a| ngon_radius(a, 4, 0.0)),
    ]
}

const MORPH_ROT: [f32; 11] = [
    0.0, 0.0, 72.0, 72.0, 144.0, 144.0, 216.0, 216.0, 288.0, 288.0, 360.0,
];
const MORPH_SCALE: [f32; 11] = [1.0, 1.0, 0.88, 0.88, 1.0, 1.0, 0.88, 0.88, 1.0, 1.0, 1.0];

#[component]
fn Morph(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase(if reduce {
        1400
    } else {
        (speed * 5.0 * 1000.0) as u64
    });
    let paths = use_hook(morph_paths);
    // Each shape appears twice so it forms and holds before the next morph.
    let sequence: Vec<usize> = vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0];

    let phase_value = number(&phase);
    let rotate = track(&MORPH_ROT, phase_value, ease_in_out);
    let scale = track(&MORPH_SCALE, phase_value, ease_in_out);
    let opacity = if reduce {
        track(&[1.0, 0.4, 1.0], phase_value, ease_in_out)
    } else {
        1.0
    };

    let d = {
        let scaled =
            (phase_value * (sequence.len() - 1) as f32).clamp(0.0, (sequence.len() - 1) as f32);
        let index = (scaled.floor() as usize).min(sequence.len() - 2);
        let local = ease_in_out(f64::from(scaled - index as f32)) as f32;
        let a = &paths[sequence[index]];
        let b = &paths[sequence[index + 1]];
        let mut out = String::new();
        for (i, (ax, ay)) in a.iter().enumerate() {
            let (bx, by) = b[i];
            let x = ax + (bx - ax) * local;
            let y = ay + (by - ay) * local;
            if i == 0 {
                out.push_str(&format!("M{x:.2} {y:.2}"));
            } else {
                out.push_str(&format!(" L{x:.2} {y:.2}"));
            }
        }
        out.push_str(" Z");
        out
    };

    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box: "0 0 100 100",
            role: "img",
            style: "opacity:{opacity};transform:rotate({rotate}deg) scale({scale})",
            title { "Loading" }
            path { fill: "currentColor", d }
        }
    }
}

// --- comet ------------------------------------------------------------------

const COMET_TRAIL: [usize; 6] = [0, 1, 2, 3, 4, 5];

#[component]
fn Comet(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase(if reduce { 1400 } else { ms(speed) });
    let head = size * 0.2;
    let r = size / 2.0 - head / 2.0;
    let phase_value = number(&phase);
    let rotate = if reduce { 0.0 } else { 360.0 * phase_value };
    let opacity = if reduce {
        track(&[1.0, 0.4, 1.0], phase_value, ease_in_out)
    } else {
        1.0
    };

    rsx! {
        span { class: "relative", style: "width:{size}px;height:{size}px;opacity:{opacity}",
            span { class: "absolute inset-0", style: "transform:rotate({rotate}deg)",
                for i in COMET_TRAIL {
                    {
                        let scale = 1.0 - i as f32 * 0.13;
                        let sz = head * scale;
                        let dot_opacity = 1.0 - i as f32 * 0.16;
                        rsx! {
                            span {
                                key: "{i}",
                                class: "absolute top-1/2 left-1/2 rounded-full bg-current",
                                style: "width:{sz}px;height:{sz}px;margin-left:{-(sz / 2.0)}px;margin-top:{-(sz / 2.0)}px;opacity:{dot_opacity};transform:rotate({-(i as f32) * 15.0}deg) translateY({-r}px)",
                            }
                        }
                    }
                }
            }
        }
    }
}

// --- scramble ---------------------------------------------------------------

const SCRAMBLE_TARGET: &str = "LOADING";
const SCRAMBLE_GLYPHS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789<>/*#@";

/// Deterministic stand-in for `Math.random()`: the same `(tick, index)` always
/// picks the same glyph, so the text only changes when the tick does.
fn scramble_glyph(tick: usize, index: usize) -> char {
    let mixed = (tick.wrapping_mul(2654435761) ^ index.wrapping_mul(40503)) >> 8;
    SCRAMBLE_GLYPHS[mixed % SCRAMBLE_GLYPHS.len()] as char
}

#[component]
fn Scramble(size: f32, speed: f32, reduce: bool) -> Element {
    let len = SCRAMBLE_TARGET.len();
    let total = len + 4;
    let step = (f64::from(speed) / len as f64) * 1000.0 * 0.55;
    let phase = use_loop_phase((step * total as f64).max(1.0) as u64);
    let tick = (number(&phase) * total as f32) as usize;

    let text = if reduce {
        SCRAMBLE_TARGET.to_string()
    } else {
        let reveal = tick % total;
        let mut out = String::new();
        for (i, ch) in SCRAMBLE_TARGET.chars().enumerate() {
            if i < reveal {
                out.push(ch);
            } else {
                out.push(scramble_glyph(tick, i));
            }
        }
        out
    };

    rsx! {
        span {
            class: "font-mono font-medium tracking-[0.2em] tabular-nums",
            style: "font-size:{size * 0.42}px",
            "{text}"
        }
    }
}

// --- metaballs --------------------------------------------------------------

#[component]
fn Metaballs(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase((speed * 1.6 * 1000.0) as u64);
    let id = use_hook(|| format!("beui-metaballs-{}", platform::next_scope_id()));
    let phase_value = number(&phase);
    let (cx_a, cx_b, opacity) = if reduce {
        (
            40.0,
            60.0,
            track(&[0.4, 1.0, 0.4], phase_value, ease_in_out),
        )
    } else {
        (
            track(&[30.0, 70.0, 30.0], phase_value, ease_in_out),
            track(&[70.0, 30.0, 70.0], phase_value, ease_in_out),
            1.0,
        )
    };

    rsx! {
        svg { width: "{size}", height: "{size}", view_box: "0 0 100 100", role: "img",
            title { "Loading" }
            defs {
                filter { id: "{id}",
                    feGaussianBlur { _in: "SourceGraphic", std_deviation: "5", result: "b" }
                    feColorMatrix {
                        _in: "b",
                        values: "1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 20 -8",
                    }
                }
            }
            g { filter: "url(#{id})", fill: "currentColor", style: "opacity:{opacity}",
                circle { cy: "50", r: "15", cx: "{cx_a}" }
                circle { cy: "50", r: "15", cx: "{cx_b}" }
            }
        }
    }
}

// --- newton -----------------------------------------------------------------

const NEWTON_BALLS: [usize; 5] = [0, 1, 2, 3, 4];

#[component]
fn Newton(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase((speed * 1.5 * 1000.0) as u64);
    let d = size * 0.2;
    let out = d * 1.1;
    let phase_value = number(&phase);

    rsx! {
        span { class: "flex items-center justify-center", style: "height:{d}px",
            for i in NEWTON_BALLS {
                span {
                    key: "{i}",
                    class: "rounded-full bg-current",
                    style: {
                        let x = if reduce {
                            0.0
                        } else if i == 0 {
                            track_at(&[0.0, -out, 0.0, 0.0], &[0.0, 0.28, 0.5, 1.0], phase_value, ease_in_out)
                        } else if i == 4 {
                            track_at(&[0.0, 0.0, out, 0.0], &[0.0, 0.5, 0.78, 1.0], phase_value, ease_in_out)
                        } else {
                            0.0
                        };
                        format!("width:{d}px;height:{d}px;transform:translateX({x}px)")
                    },
                }
            }
        }
    }
}

// --- helix ------------------------------------------------------------------

#[component]
fn Helix(size: f32, speed: f32, reduce: bool) -> Element {
    let phase = use_loop_phase(ms(speed));
    let rows = 7usize;
    let dot = size * 0.14;
    let amp = size * 0.32;
    let phase_value = number(&phase);

    rsx! {
        span { class: "relative", style: "width:{size}px;height:{size}px",
            for r in 0..rows {
                {
                    let top = (r as f32 / (rows as f32 - 1.0)) * (size - dot);
                    let offset = r as f32 / rows as f32;
                    let (x_a, scale_a, opacity_a) = if reduce {
                        (0.0, 1.0, track_offset(&[0.4, 1.0, 0.4], phase_value, offset, ease_in_out))
                    } else {
                        (
                            track_offset(&[amp, -amp, amp], phase_value, offset, ease_in_out),
                            track_offset(&[1.0, 0.5, 1.0], phase_value, offset, ease_in_out),
                            track_offset(&[1.0, 0.45, 1.0], phase_value, offset, ease_in_out),
                        )
                    };
                    let (x_b, scale_b, opacity_b) = if reduce {
                        (0.0, 1.0, track_offset(&[0.4, 1.0, 0.4], phase_value, offset, ease_in_out))
                    } else {
                        (
                            track_offset(&[-amp, amp, -amp], phase_value, offset, ease_in_out),
                            track_offset(&[0.5, 1.0, 0.5], phase_value, offset, ease_in_out),
                            track_offset(&[0.45, 1.0, 0.45], phase_value, offset, ease_in_out),
                        )
                    };
                    rsx! {
                        span { key: "row-{r}",
                            span {
                                class: "absolute rounded-full bg-current",
                                style: "width:{dot}px;height:{dot}px;left:{size / 2.0 - dot / 2.0}px;top:{top}px;opacity:{opacity_a};transform:translateX({x_a}px) scale({scale_a})",
                            }
                            span {
                                class: "absolute rounded-full bg-current",
                                style: "width:{dot}px;height:{dot}px;left:{size / 2.0 - dot / 2.0}px;top:{top}px;opacity:{opacity_b};transform:translateX({x_b}px) scale({scale_b})",
                            }
                        }
                    }
                }
            }
        }
    }
}

// --- percent ----------------------------------------------------------------

#[component]
fn Percent(size: f32, speed: f32, reduce: bool) -> Element {
    let duration = if reduce { speed * 2.0 } else { speed };
    let phase = use_loop_phase((duration * 1000.0).max(1.0) as u64);
    let p = ((number(&phase) * 100.0).round() as i32).clamp(0, 100);

    rsx! {
        span {
            class: "flex flex-col items-center",
            style: "gap:{size * 0.14}px;width:{size * 1.4}px",
            span {
                class: "font-mono font-medium tabular-nums",
                style: "font-size:{size * 0.42}px;line-height:1",
                "{p}%"
            }
            span {
                class: "w-full overflow-hidden rounded-full bg-current/15",
                style: "height:{3.0f32.max(size * 0.1)}px",
                span { class: "block h-full rounded-full bg-current", style: "width:{p}%" }
            }
        }
    }
}
