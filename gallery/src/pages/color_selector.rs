//! ColorSelector, on screen — `color-selector.preview.tsx`.

use crate::pages::common::*;
use beui::prelude::*;

const COLORS: [(&str, &str, &str); 8] = [
    ("blue", "#3478f6", "Blue"),
    ("purple", "#9270e8", "Purple"),
    ("pink", "#e66aa4", "Pink"),
    ("red", "#e55656", "Red"),
    ("orange", "#ed9141", "Orange"),
    ("amber", "#e5b63c", "Amber"),
    ("green", "#65a65a", "Green"),
    ("teal", "#169d83", "Teal"),
];

#[component]
pub fn ColorSelectorPage() -> Element {
    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "ColorSelector",
                lede: "A radio group of swatches. The selection ring is a layoutId: it hands its identity to the next swatch's ring, which glides into place from the old one's box."
            }

            Section {
                title: "Accent",
                note: "Selection never relies on color alone — each swatch carries an accessible name.",
                Stage { class: "justify-start",
                    ColorSelector { default_value: "blue", name: "accent",
                        ColorSelectorLabel { "Accent" }
                        ColorSelectorList {
                            for (value, color, label) in COLORS {
                                ColorSelectorItem { key: "{value}", value, color, label }
                            }
                        }
                    }
                }
            }
        }
    }
}
