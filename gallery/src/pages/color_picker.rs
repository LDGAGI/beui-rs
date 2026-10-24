//! ColorPicker, on screen — `color-picker.preview.tsx`.
//!
//! The full picker: trigger, popover, saturation/brightness area, hue and alpha
//! rails, a format toggle (hex / rgb / hsl), the hex field or channels, a swatch
//! and a preset row.

use crate::pages::common::*;
use beui::prelude::*;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

const FORMATS: [&str; 3] = ["hex", "rgb", "hsl"];

const PRESETS: [(&str, &str); 8] = [
    ("#3478f6", "Blue"),
    ("#9270e8", "Purple"),
    ("#e66aa4", "Pink"),
    ("#e55656", "Red"),
    ("#ed9141", "Orange"),
    ("#e5b63c", "Amber"),
    ("#65a65a", "Green"),
    ("#169d83", "Teal"),
];

#[component]
pub fn ColorPickerPage() -> Element {
    let mut color = use_signal(|| "#3478f6".to_string());
    let mut format_index = use_signal(|| 0usize);
    let format = FORMATS[format_index() % FORMATS.len()];
    let next_index = (format_index() + 1) % FORMATS.len();
    let next = FORMATS[next_index];

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "ColorPicker",
                lede: "A popover picker over one HSV state. The area and the rails all drag on the same rail model; hex, rgb and hsl are three views of the same value."
            }

            Section {
                title: "Brand color",
                note: "Click the trigger, then the format button to cycle hex → rgb → hsl.",
                Stage { class: "min-h-[460px] items-start pt-2",
                    ColorPicker {
                        aria_label: "Brand color",
                        value: color(),
                        on_value_change: move |next| color.set(next),
                        ColorPickerTrigger {}
                        ColorPickerContent {
                            aria_label: "Brand color",
                            ColorPickerArea {}
                            div { class: "flex items-center gap-3",
                                ColorPickerEyeDropper {}
                                div { class: "flex flex-1 flex-col gap-3",
                                    ColorPickerHue {}
                                    ColorPickerAlpha {}
                                }
                            }
                            div { class: "flex items-start gap-1.5",
                                button {
                                    r#type: "button",
                                    "aria-label": "Format {format.to_uppercase()}, switch to {next.to_uppercase()}",
                                    class: "relative inline-flex h-9 w-12 shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border bg-background text-xs font-medium text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                                    onclick: move |_| format_index.set(next_index),
                                    {
                                        let formats: Vec<String> = vec![format.to_string()];
                                        rsx! {
                                            AnimatePresence {
                                                for f in formats {
                                                    FormatLabel { key: "{f}", format: f }
                                                }
                                            }
                                        }
                                    }
                                }
                                if format == "hex" {
                                    ColorPickerHexInput {}
                                    ColorPickerSwatch {}
                                } else {
                                    ColorPickerChannels { format: format.to_string() }
                                }
                            }
                            ColorPickerPresets { class: "justify-between px-0.5 pt-1",
                                for (value, label) in PRESETS {
                                    ColorPickerPreset { key: "{value}", value, label }
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
fn FormatLabel(format: String) -> Element {
    let gate = use_gate();
    let style = use_presence_style(PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, y: 10.0 },
        motion_style! { opacity: 1.0, y: 0.0 },
        motion_style! { opacity: 0.0, y: -10.0 },
        spring(SPRING_SWAP),
        spring(SPRING_SWAP),
    ))
    .expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        span {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            "{format.to_uppercase()}"
        }
    }
}
