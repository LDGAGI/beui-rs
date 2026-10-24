//! `center-morph-modal.preview.tsx` — a Pro card that unfolds from its centre.

use beui::prelude::*;
use beui::{
    ArrowUpRightIcon, CenterMorphModal, CenterMorphModalContent, CenterMorphModalTrigger, CheckIcon,
};

const FEATURES: [&str; 3] = [
    "Premium animated sections",
    "Complete Next.js templates",
    "Editable source and private registry",
];

#[component]
pub fn CenterMorphModalPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Center morph modal" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The panel is laid out full size and clipped to a 48% inset; the clip tweens to zero while the radius stays constant, so the whole duration reads as surface unfolding from its exact centre."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    rsx! {
        section { class: "flex min-h-[420px] w-full items-center justify-center",
            CenterMorphModal {
                CenterMorphModalTrigger { class: "press inline-flex h-10 items-center justify-center rounded-full bg-foreground px-5 text-sm font-medium text-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    "Open modal"
                }
                CenterMorphModalContent {
                    aria_label: "beUI Pro",
                    aria_described_by: Some("center-morph-pro-description".to_string()),
                    div { class: "p-7 sm:p-8",
                        p { class: "text-sm font-medium text-muted-foreground", "beUI Pro" }
                        h2 { class: "mt-5 max-w-xs pr-8 text-2xl font-medium tracking-tight text-foreground",
                            "Ship the whole experience."
                        }
                        p { id: "center-morph-pro-description", class: "mt-3 text-sm leading-relaxed text-muted-foreground",
                            "Go beyond individual components with premium animated sections and complete Next.js templates."
                        }
                        div { class: "mt-7 space-y-3 border-y border-border py-5",
                            for feature in FEATURES {
                                div { key: "{feature}", class: "flex items-center gap-3 text-sm text-foreground",
                                    CheckIcon { class: "h-4 w-4 text-muted-foreground" }
                                    span { "{feature}" }
                                }
                            }
                        }
                        a {
                            href: "https://pro.beui.dev/?utm_source=beui&utm_medium=component_preview&utm_campaign=center_morph_modal",
                            target: "_blank",
                            rel: "noreferrer",
                            class: "press mt-7 inline-flex h-11 w-full items-center justify-center gap-2 rounded-full bg-foreground px-5 text-sm font-medium text-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                            "Explore beUI Pro"
                            ArrowUpRightIcon { class: "h-4 w-4" }
                        }
                    }
                }
            }
        }
    }
}
