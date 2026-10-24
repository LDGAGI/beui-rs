//! `FileUpload` — the beUI preview: a queue with a finished, a running and a
//! failed upload, plus a variant toggle.

use beui::prelude::*;
use beui::{FileUpload, FileUploadItem, FileUploadStatus, FileUploadVariant};
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

fn initial_items() -> Vec<FileUploadItem> {
    vec![
        FileUploadItem::new("brand-assets", "brand-assets.zip", 18_400_000)
            .mime("application/zip")
            .progress(100.0)
            .status(FileUploadStatus::Success),
        FileUploadItem::new("release-video", "release-cut.mov", 84_200_000)
            .mime("video/quicktime")
            .progress(58.0)
            .status(FileUploadStatus::Uploading),
        FileUploadItem::new("contracts", "vendor-contract.pdf", 2_800_000)
            .mime("application/pdf")
            .progress(32.0)
            .status(FileUploadStatus::Error)
            .error("Connection lost"),
    ]
}

#[component]
pub fn FileUploadPage() -> Element {
    let mut items = use_signal(initial_items);
    let mut variant = use_signal(|| FileUploadVariant::Centered);

    // The preview's simulated upload: every uploading row advances until done.
    use_future(move || async move {
        loop {
            if Time::delay(Duration::from_millis(400)).await.is_err() {
                return;
            }
            let next: Vec<FileUploadItem> = items()
                .into_iter()
                .map(|item| {
                    if item.status != FileUploadStatus::Uploading {
                        return item;
                    }
                    let progress = (item.progress.unwrap_or(0.0) + 6.0).min(100.0);
                    if progress >= 100.0 {
                        FileUploadItem {
                            progress: Some(100.0),
                            status: FileUploadStatus::Success,
                            ..item
                        }
                    } else {
                        FileUploadItem {
                            progress: Some(progress),
                            ..item
                        }
                    }
                })
                .collect();
            items.set(next);
        }
    });

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "FileUpload" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A dropzone with an upload queue. Files arrive from the picker or a drop, each row reports its kind and size, the status swaps out of a small lift, and progress is drawn with scaleX rather than width."
                }
            }
            div { class: "flex w-full flex-col items-start gap-4",
                div { class: "flex items-center gap-2 rounded-full border border-border bg-muted/40 p-1",
                    for (candidate, label) in [(FileUploadVariant::Centered, "Centered"), (FileUploadVariant::Default, "Row")] {
                        button {
                            key: "{label}",
                            r#type: "button",
                            class: cn!(
                                "rounded-full px-3 py-1 text-xs font-medium transition-colors",
                                if variant() == candidate { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" },
                            ),
                            onclick: move |_| variant.set(candidate),
                            "{label}"
                        }
                    }
                }
                FileUpload {
                    value: Some(items()),
                    variant: variant(),
                    on_value_change: Some(EventHandler::new(move |next: Vec<FileUploadItem>| items.set(next))),
                    on_retry: Some(EventHandler::new(move |retried: FileUploadItem| {
                        items.set(
                            items()
                                .into_iter()
                                .map(|item| if item.id == retried.id { retried.clone() } else { item })
                                .collect(),
                        );
                    })),
                }
            }
        }
    }
}
