//! `morphing-lightbox.preview.tsx` — three photographs that morph into a viewer.
//!
//! The source's images come from Unsplash; the gallery stays offline, so each
//! photograph is an inline SVG gradient with the same 4:5 aspect the source
//! declares (1600×2000), which is what the thumbnail morph and the aspect-ratio
//! frame read.

use beui::prelude::*;
use beui::{
    ImageViewer, ImageViewerCaption, ImageViewerClose, ImageViewerContent, ImageViewerCounter,
    ImageViewerGallery, ImageViewerNext, ImageViewerPrevious, ImageViewerThumbnail, LightboxImage,
};

/// A 1600×2000 gradient photograph, percent-encoded for a `data:` URL.
fn photo(first: &str, second: &str, third: &str, label: &str) -> String {
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1600 2000'>\
         <defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'>\
         <stop offset='0' stop-color='{first}'/><stop offset='0.55' stop-color='{second}'/>\
         <stop offset='1' stop-color='{third}'/></linearGradient></defs>\
         <rect width='1600' height='2000' fill='url(%23g)'/>\
         <circle cx='420' cy='560' r='300' fill='white' opacity='0.16'/>\
         <circle cx='1180' cy='1420' r='420' fill='white' opacity='0.10'/>\
         <rect x='180' y='1560' width='1240' height='10' rx='5' fill='white' opacity='0.35'/>\
         <text x='180' y='1500' font-family='Geist, system-ui, sans-serif' font-size='64' \
         fill='white' opacity='0.75'>{label}</text></svg>"
    );
    let encoded: String = svg
        .chars()
        .map(|character| match character {
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            '#' => "%23".to_string(),
            '"' => "%22".to_string(),
            ' ' => "%20".to_string(),
            '\n' => String::new(),
            other => other.to_string(),
        })
        .collect();
    format!("data:image/svg+xml,{encoded}")
}

fn photographs() -> Vec<LightboxImage> {
    vec![
        LightboxImage::new(
            "architecture",
            photo("#f59e0b", "#ef4444", "#7c3aed", "Form and light"),
            "Sculptural architecture",
            1600.0,
            2000.0,
        )
        .caption("01 / Form and light"),
        LightboxImage::new(
            "landscape",
            photo("#0ea5e9", "#22c55e", "#eab308", "Open landscape"),
            "Open landscape",
            1600.0,
            2000.0,
        )
        .caption("02 / A little room to breathe"),
        LightboxImage::new(
            "workspace",
            photo("#e11d48", "#9333ea", "#0ea5e9", "Places to make things"),
            "Light-filled workspace",
            1600.0,
            2000.0,
        )
        .caption("03 / Places to make things"),
    ]
}

#[component]
pub fn MorphingLightboxPage() -> Element {
    let images = use_hook(photographs);
    let mut value = use_signal(|| None::<String>);
    let count = images.len();

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · morphing-lightbox"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                    "Morphing Lightbox"
                }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A thumbnail gallery whose images morph out of the thumbnail into a focus-managed viewer: swipe or use the arrow keys to page through, and zoom in to pan."
                }
            }
            div { class: "w-full max-w-xl",
                div { class: "mb-5 flex items-end justify-between gap-3",
                    div {
                        p { class: "mb-1 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground",
                            "Collected moments"
                        }
                        h3 { class: "text-lg font-medium tracking-tight", "Look a little closer." }
                    }
                    span { class: "text-xs text-muted-foreground", "{count} photographs" }
                }
                ImageViewer {
                    images: images.clone(),
                    value: value(),
                    on_value_change: move |next| value.set(next),
                    label: "Collected photographs",
                    ImageViewerGallery { class: "grid-cols-3 gap-2 sm:gap-3",
                        for image in images.iter() {
                            ImageViewerThumbnail { key: "{image.id}", image_id: image.id.clone() }
                        }
                    }
                    ImageViewerContent {
                        header: Some(rsx! {
                            ImageViewerCounter {}
                            ImageViewerClose {}
                        }),
                        ImageViewerPrevious {}
                        ImageViewerCaption {}
                        ImageViewerNext {}
                    }
                }
                p { class: "mt-4 text-xs text-muted-foreground",
                    "Open a photograph. Swipe or use the arrows to explore."
                }
            }
        }
    }
}
