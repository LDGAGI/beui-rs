//! The five `NotFound` variants, one page each, as the beUI previews are.

use beui::prelude::*;
use beui::{NotFound, NotFoundVariant};

fn stage(variant: NotFoundVariant) -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks · not-found"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "NotFound" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A 404 page in five variants, all sharing the same stage and dual CTA."
                }
            }
            NotFound { variant }
        }
    }
}

#[component]
pub fn NotFoundStackedPage() -> Element {
    stage(NotFoundVariant::Stacked)
}

#[component]
pub fn NotFoundTerminalPage() -> Element {
    stage(NotFoundVariant::Terminal)
}

#[component]
pub fn NotFoundGlitchPage() -> Element {
    stage(NotFoundVariant::Glitch)
}

#[component]
pub fn NotFoundSpotlightPage() -> Element {
    stage(NotFoundVariant::Spotlight)
}

#[component]
pub fn NotFoundMagneticPage() -> Element {
    stage(NotFoundVariant::Magnetic)
}
