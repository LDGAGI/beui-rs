//! `blocks/attachment-upload.preview.tsx` — the launch-brief demo.
//!
//! A failed PDF, an image with a working preview, and an audio row whose
//! playhead advances while it is "playing". The retry path re-runs the source's
//! `uploading → complete → idle` timers so every row state is on screen.

use beui::prelude::*;
use beui::{AttachmentUpload, AttachmentUploadItem, AttachmentUploadKind, AttachmentUploadStatus};

/// The source pulls this from Unsplash; the gallery stays offline, so it is a
/// gradient inlined as a data URL (same box, same `object-cover`).
const PREVIEW_IMAGE: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAxMjAwIDgwMCI+PGRlZnM+PGxpbmVhckdyYWRpZW50IGlkPSJnIiB4MT0iMCIgeTE9IjAiIHgyPSIxIiB5Mj0iMSI+PHN0b3Agb2Zmc2V0PSIwIiBzdG9wLWNvbG9yPSIjZjU5ZTBiIi8+PHN0b3Agb2Zmc2V0PSIwLjU1IiBzdG9wLWNvbG9yPSIjZWY0NDQ0Ii8+PHN0b3Agb2Zmc2V0PSIxIiBzdG9wLWNvbG9yPSIjN2MzYWVkIi8+PC9saW5lYXJHcmFkaWVudD48L2RlZnM+PHJlY3Qgd2lkdGg9IjEyMDAiIGhlaWdodD0iODAwIiBmaWxsPSJ1cmwoI2cpIi8+PGNpcmNsZSBjeD0iMzAwIiBjeT0iMjUwIiByPSIxMjAiIGZpbGw9IiNmZmZmZmYiIG9wYWNpdHk9IjAuMTgiLz48Y2lyY2xlIGN4PSI5MDAiIGN5PSI1NjAiIHI9IjE4MCIgZmlsbD0iI2ZmZmZmZiIgb3BhY2l0eT0iMC4xMiIvPjwvc3ZnPgo=";

fn initial_items() -> Vec<AttachmentUploadItem> {
    vec![
        AttachmentUploadItem::new("brief", "launch-brief.pdf", AttachmentUploadKind::File)
            .size(32_400_000.0)
            .href("data:application/pdf,beUI%20launch%20brief")
            .status(AttachmentUploadStatus::Failed)
            .error("Upload failed"),
        AttachmentUploadItem::new("flowers", "orange-flowers.jpg", AttachmentUploadKind::Image)
            .size(9_800_000.0)
            .preview_url(PREVIEW_IMAGE),
        AttachmentUploadItem::new("voice-note", "launch-note.m4a", AttachmentUploadKind::Audio)
            .current_time(12.0)
            .duration(48.0),
    ]
}

async fn delay(ms: u64) {
    let _ = beui_core::platform::json::<bool>(&format!(
        "await new Promise((resolve) => setTimeout(resolve, {ms}));\nreturn true;"
    ))
    .await;
}

#[component]
pub fn AttachmentUploadPage() -> Element {
    let mut items = use_signal(initial_items);
    let mut playing_id = use_signal(|| Option::<String>::None);

    // The source runs a one-second interval while a row is playing.
    use_future(move || async move {
        loop {
            delay(1000).await;
            let Some(id) = playing_id() else {
                continue;
            };
            let mut next = items();
            let mut finished = false;
            for entry in next.iter_mut() {
                if entry.id != id {
                    continue;
                }
                if let Some(duration) = entry.duration {
                    let time = (entry.current_time.unwrap_or(0.0) + 1.0).min(duration);
                    entry.current_time = Some(time);
                    finished = time >= duration;
                }
            }
            items.set(next);
            if finished {
                playing_id.set(None);
            }
        }
    });

    let retry = EventHandler::new(move |item: AttachmentUploadItem| {
        let id = item.id.clone();
        let set_status = move |status: Option<AttachmentUploadStatus>| {
            let mut next = items();
            for entry in next.iter_mut() {
                if entry.id == id {
                    entry.status = status;
                    if status != Some(AttachmentUploadStatus::Failed) {
                        entry.error = None;
                    }
                }
            }
            items.set(next);
        };
        let mut first = set_status.clone();
        first(Some(AttachmentUploadStatus::Uploading));
        spawn(async move {
            delay(900).await;
            first(Some(AttachmentUploadStatus::Complete));
            delay(1000).await;
            first(None);
        });
    });

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · blocks"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "AttachmentUpload" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Drop a file or browse for one. Each row owns its own upload lifecycle, and the image thumbnail opens a full-screen preview."
                }
            }
            div { class: "flex w-full items-start justify-center",
                div { class: "w-full max-w-2xl px-3 py-6 sm:px-6",
                    AttachmentUpload {
                        value: Some(items()),
                        on_value_change: EventHandler::new(move |next| items.set(next)),
                        on_retry: retry,
                        playing_id: playing_id(),
                        on_audio_toggle: EventHandler::new(move |item: AttachmentUploadItem| {
                            let current = playing_id();
                            if current.as_deref() == Some(item.id.as_str()) {
                                playing_id.set(None);
                            } else {
                                playing_id.set(Some(item.id.clone()));
                            }
                        }),
                        attachments_label: "Attachments:",
                    }
                }
            }
        }
    }
}
