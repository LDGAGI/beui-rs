//! Loader, on screen — `loader.preview.tsx`.
//!
//! All seventeen variants at `size=36`, each with its name under it.

use crate::pages::common::*;
use beui::prelude::*;

const VARIANTS: [(LoaderVariant, &str); 17] = [
    (LoaderVariant::Spinner, "Spinner"),
    (LoaderVariant::Dots, "Dots"),
    (LoaderVariant::Bars, "Bars"),
    (LoaderVariant::DotMatrix, "Dot Matrix"),
    (LoaderVariant::Dither, "Dither"),
    (LoaderVariant::Morph, "Morph"),
    (LoaderVariant::Comet, "Comet"),
    (LoaderVariant::Metaballs, "Metaballs"),
    (LoaderVariant::Newton, "Newton"),
    (LoaderVariant::Helix, "Helix"),
    (LoaderVariant::Scramble, "Scramble"),
    (LoaderVariant::Percent, "Percent"),
    (LoaderVariant::Ascii, "ASCII"),
    (LoaderVariant::AsciiLine, "ASCII Line"),
    (LoaderVariant::AsciiBraille, "ASCII Braille"),
    (LoaderVariant::AsciiBlocks, "ASCII Blocks"),
    (LoaderVariant::AsciiBounce, "ASCII Bounce"),
];

#[component]
pub fn LoaderPage() -> Element {
    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "Loader",
                lede: "Seventeen loops, from a plain spinner to the terminal ASCII sets. Each is a linear phase evaluated against its keyframe tracks."
            }

            Section {
                title: "Variants",
                note: "size 36, speed 1. Reduced motion keeps the calm opacity pulse and drops every transform.",
                div { class: "flex flex-wrap items-center justify-center gap-8 rounded-xl border border-border bg-card p-8",
                    for (variant, label) in VARIANTS {
                        div { key: "{label}", class: "flex flex-col items-center gap-4",
                            Loader { variant, size: 36.0 }
                            span { class: "text-xs text-muted-foreground", "{label}" }
                        }
                    }
                }
            }
        }
    }
}
