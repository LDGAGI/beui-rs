//! `motion/attachment-upload.tsx` — the drop zone, the attachment list and the
//! full-screen image preview.
//!
//! The row state machine is the source's: `UPLOAD_PROGRESS_MS` then
//! `UPLOAD_COMPLETE_HOLD_MS` after a file is added, a `REMOVE_PENDING_MS`
//! spinner before a row actually leaves, the 28-bar waveform, and the per-row
//! arrival stagger (`min(index, 5) * 0.055s`).
//!
//! Deviations, all forced by the platform:
//!
//! - **No `File` handle on the item.** The source keeps `file?: File` so it can
//!   mint an object URL. `FileData` is neither portable nor fit for props, so
//!   the handles are handed to the consumer once through `on_files_added` (the
//!   contract `FileUpload` already uses). Picker-added images therefore render
//!   the placeholder until the consumer supplies `preview_url` / `href`; items
//!   passed in already get the thumbnail and the dialog.
//! - **Native `title` instead of `Tooltip`.** The source wraps nearly every
//!   control in `Tooltip`, which would be a portaled overlay per row, and a
//!   portaled subtree loses its `onclick` on this platform . Each control keeps the same accessible name; the hint
//!   moves to `title`.
//! - **The preview renders in place.** The source portals it to
//!   `document.body`; here it is a `fixed` layer inside the component, with
//!   `use_modal_scope` for the scroll lock, Escape, focus trap and focus return.
//!   The shared-element morph from thumbnail to full image is a cross-fade.
//! - **Exit animations are dropped.** Rows and the dialog unmount as soon as
//!   their state says so; the source holds them through `AnimatePresence`.
//! - **The list mirrors into a signal.** `on_value_change` still fires and an
//!   incoming `value` still wins, but the rendered list comes from the internal
//!   store, so a controlled parent that ignores the callback will not freeze the
//!   spinner states.

use crate::motion::motion_style;
use crate::prelude::*;
use beui_core::hooks::{use_dismiss, use_modal_scope, DismissOptions, OverlayScope};
use beui_core::platform::{js_string, next_scope_id};
use dioxus::html::{DragData, FileData, HasFileData};

// --- public types -----------------------------------------------------------

/// `AttachmentUploadKind`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AttachmentUploadKind {
    #[default]
    File,
    Link,
    Image,
    Audio,
}

/// `AttachmentRejectReason`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttachmentRejectReason {
    TooLarge,
    MaxFiles,
}

/// `AttachmentUploadStatus`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AttachmentUploadStatus {
    #[default]
    Idle,
    Uploading,
    Complete,
    Failed,
}

/// `AttachmentUploadItem`, minus the source's `file?: File`.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct AttachmentUploadItem {
    pub id: String,
    pub name: String,
    pub kind: AttachmentUploadKind,
    pub size: Option<f64>,
    pub href: Option<String>,
    pub preview_url: Option<String>,
    pub current_time: Option<f64>,
    pub duration: Option<f64>,
    pub status: Option<AttachmentUploadStatus>,
    pub error: Option<String>,
}

impl AttachmentUploadItem {
    /// An item with the three required fields.
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: AttachmentUploadKind) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            ..Self::default()
        }
    }

    pub fn size(mut self, size: f64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    pub fn preview_url(mut self, url: impl Into<String>) -> Self {
        self.preview_url = Some(url.into());
        self
    }

    pub fn current_time(mut self, seconds: f64) -> Self {
        self.current_time = Some(seconds);
        self
    }

    pub fn duration(mut self, seconds: f64) -> Self {
        self.duration = Some(seconds);
        self
    }

    pub fn status(mut self, status: AttachmentUploadStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

// --- constants --------------------------------------------------------------

const DEFAULT_MAX_FILE_SIZE: f64 = 500.0 * 1024.0 * 1024.0;
/// `UPLOAD_PROGRESS_MS`.
pub const UPLOAD_PROGRESS_MS: u64 = 900;
/// `UPLOAD_COMPLETE_HOLD_MS`.
pub const UPLOAD_COMPLETE_HOLD_MS: u64 = 1000;
/// `REMOVE_PENDING_MS`.
pub const REMOVE_PENDING_MS: u64 = 420;

/// `WAVEFORM_BARS` — the 28 bar heights, verbatim.
const WAVEFORM_BARS: [f64; 28] = [
    18.0, 31.0, 24.0, 39.0, 30.0, 43.0, 27.0, 18.0, 9.0, 29.0, 38.0, 24.0, 34.0, 18.0, 26.0, 37.0,
    21.0, 14.0, 7.0, 11.0, 22.0, 35.0, 18.0, 26.0, 41.0, 29.0, 17.0, 33.0,
];

// --- helpers ----------------------------------------------------------------

/// `formatBytes` — `None` for a missing or non-positive size.
pub fn format_bytes(bytes: Option<f64>) -> Option<String> {
    let bytes = bytes?;
    if !bytes.is_finite() || bytes <= 0.0 {
        return None;
    }
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let exponent = ((bytes.ln() / 1024f64.ln()).floor() as usize).min(UNITS.len() - 1);
    let value = bytes / 1024f64.powi(exponent as i32);
    let text = if value >= 10.0 || exponent == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    };
    Some(format!("{text} {}", UNITS[exponent]))
}

/// `formatDuration` — `m:ss`.
pub fn format_duration(seconds: Option<f64>) -> String {
    let safe = seconds.unwrap_or(0.0).max(0.0).round() as i64;
    format!("{}:{:02}", safe / 60, safe % 60)
}

/// `formatMaxSize` — whole megabytes drop the decimal.
pub fn format_max_size(bytes: f64) -> String {
    let megabytes = bytes / (1024.0 * 1024.0);
    if megabytes.fract() == 0.0 {
        format!("{megabytes:.0} MB")
    } else {
        format!("{megabytes:.1} MB")
    }
}

/// `inferKind` — image / audio by MIME, everything else is a file.
pub fn infer_kind(mime: &str) -> AttachmentUploadKind {
    if mime.starts_with("image/") {
        AttachmentUploadKind::Image
    } else if mime.starts_with("audio/") {
        AttachmentUploadKind::Audio
    } else {
        AttachmentUploadKind::File
    }
}

/// `imageSource` — only images have one.
fn image_source(item: &AttachmentUploadItem) -> Option<String> {
    if item.kind != AttachmentUploadKind::Image {
        return None;
    }
    item.preview_url.clone().or_else(|| item.href.clone())
}

/// `await new Promise(setTimeout)` — one-shot, never per frame (a per-frame
/// `eval` blows the wasm table; see the module notes).
async fn delay_ms(ms: u64) {
    let _ = beui_core::platform::json::<bool>(&format!(
        "await new Promise((resolve) => setTimeout(resolve, {ms}));\nreturn true;"
    ))
    .await;
}

/// The waveform and the removing spinner, as CSS keyframes.
const ATTACHMENT_ANIMATIONS: &str = r#"
@keyframes beui-attachment-wave { 0%, 100% { transform: scaleY(0.78); } 50% { transform: scaleY(1); } }
@keyframes beui-attachment-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) {
  [data-wave] { animation: none !important; }
  [data-beui-attachment-spin] { animation: none !important; }
}
"#;

// --- small components -------------------------------------------------------

/// `RowActionState`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowActionState {
    Idle,
    Uploading,
    Complete,
    Failed,
    Removing,
}

#[component]
fn AttachmentIcon(kind: AttachmentUploadKind) -> Element {
    match kind {
        AttachmentUploadKind::Link => rsx! { LinkIcon { class: "size-4" } },
        AttachmentUploadKind::Image => rsx! { FileImage { class: "size-4" } },
        AttachmentUploadKind::Audio => rsx! { Mic { class: "size-4" } },
        AttachmentUploadKind::File => rsx! { Paperclip { class: "size-4" } },
    }
}

#[component]
fn RowAction(
    label: String,
    onclick: EventHandler<()>,
    state: RowActionState,
    #[props(default)] retryable: bool,
    #[props(default)] reduce: bool,
) -> Element {
    // Every hook is called before any branch, so the order never changes.
    let mut press = use_style_motion(motion_style! { scale: 1.0 });
    let mut appear = use_style_motion(if reduce {
        motion_style! { opacity: 0.0 }
    } else {
        motion_style! { opacity: 0.0, scale: 0.75 }
    });
    let enter = state == RowActionState::Complete;
    use_effect(use_reactive((&enter,), move |(enter,)| {
        if enter {
            animate_style(
                &mut appear,
                motion_style! { opacity: 1.0, scale: 1.0 },
                tween(200, Ease::Out),
            );
        }
    }));

    let mut press_down = move || {
        animate_style(
            &mut press,
            motion_style! { scale: 0.92 },
            spring(SPRING_PRESS),
        )
    };
    let mut press_up = move || {
        animate_style(
            &mut press,
            motion_style! { scale: 1.0 },
            spring(SPRING_PRESS),
        )
    };

    match state {
        RowActionState::Uploading => {
            rsx! { span { aria_hidden: "true", class: "size-9 shrink-0" } }
        }
        RowActionState::Complete => rsx! {
            span {
                role: "status",
                title: "Upload complete",
                aria_label: "Upload complete for {label}",
                "data-beui-attachment-state": "complete",
                class: "grid size-9 shrink-0 place-items-center rounded-xl text-emerald-600 dark:text-emerald-400",
                style: css(&appear),
                Check { class: "size-4" }
            }
        },
        RowActionState::Removing => rsx! {
            span {
                role: "status",
                title: "Removing attachment",
                aria_label: "Removing {label}",
                "data-beui-attachment-state": "removing",
                class: "grid size-9 shrink-0 place-items-center rounded-xl text-muted-foreground",
                span {
                    class: "grid place-items-center",
                    "data-beui-attachment-spin": "",
                    style: if reduce {
                        String::new()
                    } else {
                        "animation: beui-attachment-spin 0.7s linear infinite".to_string()
                    },
                    LoaderCircle { class: "size-4" }
                }
            }
        },
        RowActionState::Failed if !retryable => rsx! {
            span {
                role: "status",
                title: "Upload failed",
                aria_label: "Upload failed for {label}",
                "data-beui-attachment-state": "failed",
                class: "grid size-9 shrink-0 place-items-center rounded-xl text-destructive",
                AlertCircle { class: "size-4" }
            }
        },
        RowActionState::Failed => rsx! {
            button {
                r#type: "button",
                title: "Retry upload",
                aria_label: "Retry {label}",
                "data-beui-attachment-action": "retry",
                class: "grid size-9 shrink-0 place-items-center rounded-xl text-destructive outline-none transition-colors hover:bg-destructive/10 focus-visible:ring-2 focus-visible:ring-ring",
                style: css(&press),
                onclick: move |_| onclick.call(()),
                onpointerdown: move |_| press_down(),
                onpointerup: move |_| press_up(),
                onpointerleave: move |_| press_up(),
                RotateCcw { class: "size-4" }
            }
        },
        RowActionState::Idle => rsx! {
            button {
                r#type: "button",
                title: "Remove attachment",
                aria_label: "Remove {label}",
                "data-beui-attachment-action": "remove",
                class: "grid size-9 shrink-0 place-items-center rounded-xl text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                style: css(&press),
                onclick: move |_| onclick.call(()),
                onpointerdown: move |_| press_down(),
                onpointerup: move |_| press_up(),
                onpointerleave: move |_| press_up(),
                X { class: "size-4" }
            }
        },
    }
}

/// `ImageThumbnail` — the framed preview button, or the placeholder.
#[component]
fn ImageThumbnail(
    item: AttachmentUploadItem,
    on_preview: EventHandler<AttachmentUploadItem>,
) -> Element {
    let mut press = use_style_motion(motion_style! { scale: 1.0 });
    let Some(src) = image_source(&item) else {
        return rsx! {
            span {
                aria_hidden: "true",
                class: "grid size-8 shrink-0 place-items-center rounded-lg bg-muted text-muted-foreground",
                FileImage { class: "size-4" }
            }
        };
    };
    let label = item.name.clone();
    let for_click = item.clone();

    rsx! {
        button {
            r#type: "button",
            title: "Click to preview",
            aria_label: "Preview {label}",
            "data-beui-attachment-thumbnail": "{item.id}",
            class: "group/image relative size-9 shrink-0 overflow-hidden rounded-[10px] bg-muted outline-none ring-1 ring-border/70 focus-visible:ring-2 focus-visible:ring-ring",
            style: css(&press),
            onclick: move |_| on_preview.call(for_click.clone()),
            onpointerdown: move |_| {
                animate_style(&mut press, motion_style! { scale: 0.94 }, spring(SPRING_PRESS))
            },
            onpointerup: move |_| {
                animate_style(&mut press, motion_style! { scale: 1.0 }, spring(SPRING_PRESS))
            },
            onpointerleave: move |_| {
                animate_style(&mut press, motion_style! { scale: 1.0 }, spring(SPRING_PRESS))
            },
            img { src: "{src}", alt: "", class: "size-full object-cover" }
        }
    }
}

/// `ImagePreviewDialog` — an in-place `fixed` overlay (no portal here).
#[component]
fn ImagePreviewDialog(
    item: Option<AttachmentUploadItem>,
    on_close: EventHandler<()>,
    #[props(default)] reduce: bool,
) -> Element {
    let mut open = use_signal(|| item.is_some());
    let present = item.is_some();
    use_effect(use_reactive((&present,), move |(present,)| {
        open.set(present)
    }));

    let close = EventHandler::new(move |_| on_close.call(()));
    let scope: OverlayScope = use_dismiss(open, close, DismissOptions::default().without_escape());
    use_modal_scope(open, close, scope);

    let mut fade = use_style_motion(motion_style! { opacity: 0.0 });
    use_effect(use_reactive((&present,), move |(present,)| {
        if present {
            animate_style(
                &mut fade,
                motion_style! { opacity: 1.0 },
                tween(if reduce { 100 } else { 200 }, Ease::Out),
            );
        }
    }));

    let Some(item) = item else {
        return rsx! {};
    };
    let Some(src) = image_source(&item) else {
        return rsx! {};
    };
    let name = item.name.clone();

    rsx! {
        div {
            "data-beui-attachment-preview": "{item.id}",
            class: "pointer-events-none fixed left-0 top-0 z-[10000] size-0",
            button {
                r#type: "button",
                tabindex: "-1",
                title: "Close image preview",
                aria_label: "Close image preview",
                class: "pointer-events-auto fixed inset-0 size-full cursor-default bg-black/45 backdrop-blur-xl",
                style: css(&fade),
                onclick: move |_| on_close.call(()),
            }
            div { class: "fixed inset-4 flex items-center justify-center sm:inset-8",
                div {
                    "data-beui-scope": scope.attr(),
                    role: "dialog",
                    aria_modal: "true",
                    aria_label: "Preview of {name}",
                    class: "pointer-events-auto relative",
                    style: css(&fade),
                    img {
                        src: "{src}",
                        alt: "{name}",
                        class: "max-h-[90vh] max-w-[90vw] rounded-2xl object-contain shadow-2xl",
                    }
                    button {
                        r#type: "button",
                        title: "Close image preview",
                        aria_label: "Close image preview",
                        "data-beui-attachment-preview-close": "",
                        autofocus: true,
                        class: "absolute -right-3 -top-3 grid size-9 place-items-center rounded-full bg-background text-foreground shadow-xl outline-none ring-1 ring-border/70 transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring",
                        onclick: move |_| on_close.call(()),
                        X { class: "size-4" }
                    }
                }
            }
        }
    }
}

/// `AttachmentRow` — one entry plus its action button.
#[component]
fn AttachmentRow(
    item: AttachmentUploadItem,
    #[props(default)] playing: bool,
    #[props(default)] uploading: bool,
    #[props(default)] upload_complete: bool,
    #[props(default)] failed: bool,
    #[props(default)] removing: bool,
    #[props(default = -1)] arrival_index: i32,
    #[props(default)] on_audio_toggle: Option<EventHandler<AttachmentUploadItem>>,
    on_image_preview: EventHandler<AttachmentUploadItem>,
    on_remove: EventHandler<AttachmentUploadItem>,
    #[props(default)] on_retry: Option<EventHandler<AttachmentUploadItem>>,
    #[props(default)] reduce: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let size = format_bytes(item.size);
    let progress = match item.duration {
        Some(duration) if duration > 0.0 => {
            (item.current_time.unwrap_or(0.0) / duration).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };
    let action_state = if removing {
        RowActionState::Removing
    } else if uploading {
        RowActionState::Uploading
    } else if upload_complete {
        RowActionState::Complete
    } else if failed {
        RowActionState::Failed
    } else {
        RowActionState::Idle
    };
    let arrival_delay = (arrival_index.clamp(0, 5) as f64) * 0.055;

    let initial = if reduce {
        motion_style! { opacity: 0.0 }
    } else if arrival_index >= 0 {
        motion_style! { opacity: 0.0, y: -16.0, scale: 0.985 }
    } else {
        motion_style! { opacity: 0.0, y: 6.0 }
    };
    let mut row = use_style_motion(initial);
    use_effect(move || {
        let config = if !reduce && arrival_index >= 0 {
            spring(SPRING_LAYOUT).with_delay(dioxus_motion::prelude::Duration::from_secs_f64(
                arrival_delay,
            ))
        } else {
            tween(200, Ease::Out)
        };
        animate_style(
            &mut row,
            motion_style! { opacity: 1.0, y: 0.0, scale: 1.0 },
            config,
        );
    });

    // The upload bar sweeps left→right over `UPLOAD_PROGRESS_MS`, then dims.
    let mut sweep = use_style_motion(motion_style! { opacity: 1.0, scale_x: 0.0 });
    let show_upload_progress = uploading || upload_complete;
    use_effect(use_reactive((&show_upload_progress,), move |(shown,)| {
        if shown {
            animate_style(
                &mut sweep,
                motion_style! { opacity: 1.0, scale_x: 1.0 },
                tween(if reduce { 100 } else { UPLOAD_PROGRESS_MS }, Ease::Out),
            );
        } else {
            animate_style(
                &mut sweep,
                motion_style! { opacity: 0.0 },
                tween(100, Ease::Out),
            );
        }
    }));

    let wave_playing = playing && !reduce;
    let play_label = if playing { "Pause" } else { "Play" }.to_string();
    let is_image = item.kind == AttachmentUploadKind::Image;
    let is_audio = item.kind == AttachmentUploadKind::Audio;
    let error_text = item
        .error
        .clone()
        .unwrap_or_else(|| "Upload failed".to_string());
    let play_for_click = item.clone();

    rsx! {
        li {
            "data-beui-attachment-row": "{item.id}",
            class: cn!(
                "flex min-h-14 items-center gap-1 rounded-2xl bg-muted/70 p-1",
                class,
            ),
            style: css(&row),
            div { class: "relative isolate flex min-w-0 flex-1 items-center gap-3 self-stretch overflow-hidden rounded-xl bg-background px-2 py-1",
                if failed {
                    span {
                        aria_hidden: "true",
                        class: "pointer-events-none absolute inset-0 -z-10 bg-destructive/10",
                    }
                }

                if is_image {
                    ImageThumbnail { item: item.clone(), on_preview: on_image_preview }
                } else {
                    span { aria_hidden: "true", class: "grid size-7 shrink-0 place-items-center text-muted-foreground",
                        AttachmentIcon { kind: item.kind }
                    }
                }

                if is_audio {
                    span { class: "w-9 shrink-0 text-xs tabular-nums text-muted-foreground",
                        "{format_duration(item.current_time)}"
                    }
                    span { aria_hidden: "true", class: "flex h-11 min-w-0 flex-1 items-center gap-[3px] overflow-hidden",
                        for (index, height) in WAVEFORM_BARS.iter().enumerate() {
                            span {
                                key: "{index}",
                                "data-wave": "{index}",
                                class: cn!(
                                    "w-[3px] shrink-0 rounded-full",
                                    if (index as f64) / (WAVEFORM_BARS.len() as f64) <= progress {
                                        "bg-foreground"
                                    } else {
                                        "bg-muted-foreground/35"
                                    },
                                ),
                                style: format!(
                                    "height:{height}px;{}",
                                    if wave_playing {
                                        format!(
                                            "animation: beui-attachment-wave 0.55s {}s ease-out infinite",
                                            index as f64 * 0.018
                                        )
                                    } else {
                                        String::new()
                                    }
                                ),
                            }
                        }
                    }
                    span { class: "w-9 shrink-0 text-right text-xs tabular-nums text-muted-foreground",
                        "{format_duration(item.duration)}"
                    }
                    button {
                        r#type: "button",
                        aria_label: "{play_label} {item.name}",
                        "data-beui-attachment-play": "{item.id}",
                        class: "grid size-9 shrink-0 place-items-center rounded-full bg-foreground text-background outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                        onclick: move |_| {
                            if let Some(handler) = &on_audio_toggle {
                                handler.call(play_for_click.clone());
                            }
                        },
                        if playing {
                            Pause { class: "size-4 fill-current" }
                        } else {
                            Play { class: "size-4 translate-x-px fill-current" }
                        }
                    }
                } else {
                    span { class: "min-w-0 flex-1",
                        span { class: "block truncate text-sm font-medium text-foreground", "{item.name}" }
                        if failed {
                            span { class: "block truncate text-[11px] text-destructive", "{error_text}" }
                        }
                    }
                    span { class: "shrink-0 text-xs text-muted-foreground",
                        if item.kind == AttachmentUploadKind::Link {
                            "Web"
                        } else if let Some(size) = &size {
                            "{size}"
                        }
                    }
                    if item.kind == AttachmentUploadKind::Link {
                        if let Some(href) = &item.href {
                            a {
                                href: "{href}",
                                target: "_blank",
                                rel: "noreferrer noopener",
                                aria_label: "Open {item.name}",
                                class: "grid size-8 shrink-0 place-items-center rounded-lg text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                                ExternalLink { class: "size-4" }
                            }
                        }
                    }
                }

                if show_upload_progress {
                    span {
                        role: "progressbar",
                        aria_label: "Uploading {item.name}",
                        "data-beui-attachment-progress": "{item.id}",
                        class: "pointer-events-none absolute inset-0 -z-10 origin-left bg-emerald-400/25 dark:bg-emerald-500/20",
                        style: css(&sweep),
                    }
                }
            }

            RowAction {
                label: item.name.clone(),
                state: action_state,
                retryable: on_retry.is_some(),
                reduce,
                onclick: {
                    let item = item.clone();
                    let is_failed = action_state == RowActionState::Failed;
                    move |_| {
                        if is_failed {
                            if let Some(handler) = &on_retry {
                                handler.call(item.clone());
                            }
                        } else {
                            on_remove.call(item.clone());
                        }
                    }
                },
            }
        }
    }
}

// --- the component ----------------------------------------------------------

/// Props for [`AttachmentUpload`].
#[derive(Props, Clone, PartialEq)]
pub struct AttachmentUploadProps {
    /// Controlled list; an incoming value wins, and the internal mirror keeps
    /// the transient states moving either way.
    #[props(default)]
    pub value: Option<Vec<AttachmentUploadItem>>,
    #[props(default)]
    pub default_value: Vec<AttachmentUploadItem>,
    #[props(default)]
    pub on_value_change: Option<EventHandler<Vec<AttachmentUploadItem>>>,
    /// `(items, files)` — the file handles are handed over exactly once.
    #[props(default)]
    pub on_files_added: Option<EventHandler<(Vec<AttachmentUploadItem>, Vec<FileData>)>>,
    /// `(files, reason)` for an oversize batch or a full list.
    #[props(default)]
    pub on_files_rejected: Option<EventHandler<(Vec<FileData>, AttachmentRejectReason)>>,
    #[props(default)]
    pub on_remove: Option<EventHandler<AttachmentUploadItem>>,
    #[props(default)]
    pub on_retry: Option<EventHandler<AttachmentUploadItem>>,
    #[props(default)]
    pub playing_id: Option<String>,
    #[props(default)]
    pub on_audio_toggle: Option<EventHandler<AttachmentUploadItem>>,
    #[props(default)]
    pub accept: Option<String>,
    #[props(default = true)]
    pub multiple: bool,
    #[props(default = 12)]
    pub max_files: usize,
    #[props(default = DEFAULT_MAX_FILE_SIZE)]
    pub max_file_size: f64,
    #[props(default)]
    pub disabled: bool,
    #[props(default = "Drag and drop or browse files".to_string())]
    pub title: String,
    #[props(default)]
    pub description: Option<String>,
    #[props(default = "Attachments".to_string())]
    pub attachments_label: String,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub dropzone_class: Option<String>,
    #[props(default)]
    pub list_class: Option<String>,
    #[props(default)]
    pub row_class: Option<String>,
}

/// The drop zone plus the attachment list and image preview.
#[component]
pub fn AttachmentUpload(props: AttachmentUploadProps) -> Element {
    let AttachmentUploadProps {
        value,
        default_value,
        on_value_change,
        on_files_added,
        on_files_rejected,
        on_remove,
        on_retry,
        playing_id,
        on_audio_toggle,
        accept,
        multiple,
        max_files,
        max_file_size,
        disabled,
        title,
        description,
        attachments_label,
        class,
        dropzone_class,
        list_class,
        row_class,
    } = props;

    let scope = use_hook(next_scope_id);
    let input_selector = format!("[data-beui-attachment-input=\"{scope}\"]");
    let section_id = format!("{scope}-attachments");

    // --- state ---------------------------------------------------------------
    let store = use_signal(|| value.clone().unwrap_or_else(|| default_value.clone()));
    use_effect(use_reactive((&value,), move |(value,)| {
        if let Some(value) = value {
            let mut store = store;
            store.set(value);
        }
    }));
    let items = store();

    let mut dragging = use_signal(|| false);
    let mut drag_depth = use_signal(|| 0i32);
    let preview_item = use_signal(|| Option::<AttachmentUploadItem>::None);
    let mut uploading_ids = use_signal(Vec::<String>::new);
    let upload_complete_ids = use_signal(Vec::<String>::new);
    let mut removing_ids = use_signal(Vec::<String>::new);

    let reduce = use_reduced_motion()();
    let commit = move |next: Vec<AttachmentUploadItem>| {
        let mut store = store;
        store.set(next.clone());
        if let Some(handler) = &on_value_change {
            handler.call(next);
        }
    };

    let max_reached = items.len() >= max_files;
    let blocked = disabled || max_reached;

    // --- add -----------------------------------------------------------------
    let add_files = {
        let items = items.clone();
        move |incoming: Vec<FileData>| {
            if blocked || incoming.is_empty() {
                return;
            }
            let current_len = items.len();
            let available = max_files.saturating_sub(current_len);
            if available == 0 {
                if let Some(handler) = &on_files_rejected {
                    handler.call((incoming, AttachmentRejectReason::MaxFiles));
                }
                return;
            }
            let take = if multiple {
                available
            } else {
                available.min(1)
            };
            let selected: Vec<FileData> = incoming.iter().take(take).cloned().collect();
            let oversized: Vec<FileData> = selected
                .iter()
                .filter(|file| file.size() as f64 > max_file_size)
                .cloned()
                .collect();
            let accepted: Vec<FileData> = selected
                .iter()
                .filter(|file| file.size() as f64 <= max_file_size)
                .cloned()
                .collect();
            if !oversized.is_empty() {
                if let Some(handler) = &on_files_rejected {
                    handler.call((oversized, AttachmentRejectReason::TooLarge));
                }
            }
            if incoming.len() > selected.len() {
                if let Some(handler) = &on_files_rejected {
                    handler.call((
                        incoming[selected.len()..].to_vec(),
                        AttachmentRejectReason::MaxFiles,
                    ));
                }
            }

            let now = beui_core::platform::now_ms();
            let added: Vec<AttachmentUploadItem> = accepted
                .iter()
                .enumerate()
                .map(|(index, file)| {
                    let mime = file.content_type().unwrap_or_default();
                    AttachmentUploadItem {
                        id: format!("{}-{}-{}", now as u64, index, file.name()),
                        name: file.name(),
                        kind: infer_kind(&mime),
                        size: Some(file.size() as f64),
                        ..AttachmentUploadItem::default()
                    }
                })
                .collect();
            if added.is_empty() {
                return;
            }

            let mut next = items.clone();
            next.extend(added.iter().cloned());
            commit(next);

            let added_ids: Vec<String> = added.iter().map(|entry| entry.id.clone()).collect();
            {
                let mut current = uploading_ids();
                current.extend(added_ids.iter().cloned());
                uploading_ids.set(current);
            }

            let progress_ms = if reduce { 140 } else { UPLOAD_PROGRESS_MS };
            let ids = added_ids.clone();
            spawn(async move {
                delay_ms(progress_ms).await;
                let mut uploading = uploading_ids;
                uploading.set(
                    uploading()
                        .into_iter()
                        .filter(|id| !ids.contains(id))
                        .collect(),
                );
                let mut complete = upload_complete_ids;
                let mut next_complete = complete();
                next_complete.extend(ids.iter().cloned());
                complete.set(next_complete);

                delay_ms(UPLOAD_COMPLETE_HOLD_MS).await;
                let mut complete = upload_complete_ids;
                complete.set(
                    complete()
                        .into_iter()
                        .filter(|id| !ids.contains(id))
                        .collect(),
                );
            });

            if let Some(handler) = &on_files_added {
                handler.call((added, accepted));
            }
        }
    };

    // --- remove --------------------------------------------------------------
    let finalize_remove = {
        move |item: AttachmentUploadItem| {
            let mut preview = preview_item;
            if preview().as_ref().is_some_and(|entry| entry.id == item.id) {
                preview.set(None);
            }
            let mut uploading = uploading_ids;
            uploading.set(
                uploading()
                    .into_iter()
                    .filter(|id| id != &item.id)
                    .collect(),
            );
            let mut complete = upload_complete_ids;
            complete.set(complete().into_iter().filter(|id| id != &item.id).collect());
            let store = store;
            let next = store()
                .into_iter()
                .filter(|entry| entry.id != item.id)
                .collect::<Vec<_>>();
            commit(next);
            if let Some(handler) = &on_remove {
                handler.call(item);
            }
        }
    };

    let mut request_remove = {
        move |item: AttachmentUploadItem| {
            if removing_ids().contains(&item.id) {
                return;
            }
            {
                let mut current = removing_ids();
                current.push(item.id.clone());
                removing_ids.set(current);
            }
            let id = item.id.clone();
            let delay = if reduce { 140 } else { REMOVE_PENDING_MS };
            let item_for_finalize = item.clone();
            spawn(async move {
                delay_ms(delay).await;
                finalize_remove(item_for_finalize);
                let mut removing = removing_ids;
                removing.set(
                    removing()
                        .into_iter()
                        .filter(|entry| entry != &id)
                        .collect(),
                );
            });
        }
    };

    // Drop a preview that points at an item that is gone.
    use_effect(move || {
        if let Some(entry) = preview_item() {
            if !store().iter().any(|item| item.id == entry.id) {
                let mut preview = preview_item;
                preview.set(None);
            }
        }
    });

    // --- drop zone motion ----------------------------------------------------
    let dragging_now = dragging();
    let mut zone_press = use_style_motion(motion_style! { scale: 1.0 });
    let mut zone_lift = use_style_motion(motion_style! { y: 0.0, scale: 1.0 });
    use_effect(use_reactive((&dragging_now,), move |(dragging_now,)| {
        if reduce {
            return;
        }
        animate_style(
            &mut zone_lift,
            motion_style! {
                y: if dragging_now { -4.0 } else { 0.0 },
                scale: if dragging_now { 1.08 } else { 1.0 },
            },
            tween(200, Ease::Out),
        );
    }));

    let dropzone_classes = cn!(
        "group relative isolate flex min-h-52 w-full flex-col items-center justify-center overflow-hidden rounded-[2rem] bg-muted/65 p-2 text-center outline-none",
        "transition-colors duration-200 hover:bg-muted/85",
        "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "data-[dragging=true]:bg-muted",
        "disabled:pointer-events-none disabled:opacity-55",
        dropzone_class,
    );
    let zone_scale = if reduce || !dragging_now {
        String::new()
    } else {
        "transform: scale(1.006);".to_string()
    };
    let today = if max_reached {
        "Attachment limit reached".to_string()
    } else {
        title
    };
    let hint = if max_reached {
        format!("{} of {} attachments added", items.len(), max_files)
    } else if let Some(description) = description.clone() {
        description
    } else {
        format!("Maximum {} file size", format_max_size(max_file_size))
    };
    let has_items = !items.is_empty();

    rsx! {
        style { dangerous_inner_html: ATTACHMENT_ANIMATIONS }
        div { class: cn!("w-full", class),
            input {
                "data-beui-attachment-input": "{scope}",
                id: "{scope}",
                r#type: "file",
                aria_label: "Upload attachments",
                accept,
                multiple,
                disabled: blocked,
                tabindex: "-1",
                class: "sr-only",
                onchange: {
                    let mut add_files = add_files.clone();
                    move |event: Event<FormData>| {
                        add_files(event.files());
                    }
                },
            }

            button {
                r#type: "button",
                disabled: blocked,
                "data-beui-attachment-dropzone": "",
                "data-dragging": if dragging_now { "true" } else { "false" },
                class: dropzone_classes,
                style: zone_scale,
                onclick: {
                    let selector = input_selector.clone();
                    move |_| {
                        let selector = selector.clone();
                        spawn(async move {
                            let script = format!(
                                "const el = document.querySelector({});\nif (el) el.click();\nreturn true;",
                                js_string(&selector)
                            );
                            let _ = beui_core::platform::json::<bool>(&script).await;
                        });
                    }
                },
                onpointerdown: move |_| {
                    if !reduce {
                        animate_style(&mut zone_press, motion_style! { scale: 0.995 }, spring(SPRING_PRESS));
                    }
                },
                onpointerup: move |_| {
                    if !reduce {
                        animate_style(&mut zone_press, motion_style! { scale: 1.0 }, spring(SPRING_PRESS));
                    }
                },
                ondragenter: move |event: Event<DragData>| {
                    if blocked {
                        return;
                    }
                    event.prevent_default();
                    drag_depth += 1;
                    dragging.set(true);
                },
                ondragover: move |event: Event<DragData>| {
                    if blocked {
                        return;
                    }
                    event.prevent_default();
                    dragging.set(true);
                },
                ondragleave: move |event: Event<DragData>| {
                    if blocked {
                        return;
                    }
                    event.prevent_default();
                    drag_depth.set((drag_depth() - 1).max(0));
                    if drag_depth() == 0 {
                        dragging.set(false);
                    }
                },
                ondrop: {
                    let mut add_files = add_files.clone();
                    move |event: Event<DragData>| {
                        if blocked {
                            return;
                        }
                        event.prevent_default();
                        drag_depth.set(0);
                        dragging.set(false);
                        add_files(event.files());
                    }
                },
                span {
                    aria_hidden: "true",
                    class: "absolute inset-2 -z-10 rounded-[1.5rem] border border-dashed border-muted-foreground/25 bg-background transition-[border-color,background-color] duration-200 group-hover:border-muted-foreground/45 group-data-[dragging=true]:border-foreground/65 group-data-[dragging=true]:bg-muted/20",
                }
                span {
                    aria_hidden: "true",
                    class: "mb-3 grid size-11 place-items-center rounded-2xl bg-muted text-foreground transition-colors duration-200 group-hover:bg-muted/80 group-data-[dragging=true]:bg-foreground group-data-[dragging=true]:text-background",
                    style: css(&zone_lift),
                    Upload { class: "size-[18px]" }
                }
                span { class: "text-sm font-semibold tracking-[-0.01em] text-foreground", "{today}" }
                span { class: "mt-1 text-xs leading-5 text-muted-foreground", "{hint}" }
            }

            if has_items {
                section { class: "mt-8", aria_labelledby: "{section_id}",
                    h3 { id: "{section_id}", class: "text-sm font-semibold text-foreground",
                        "{attachments_label}"
                    }
                    ul { class: cn!("mt-3 space-y-2", list_class),
                        for item in items.iter() {
                            AttachmentRow {
                                key: "{item.id}",
                                item: item.clone(),
                                playing: playing_id.as_deref() == Some(item.id.as_str()),
                                uploading: uploading_ids().contains(&item.id)
                                    || item.status == Some(AttachmentUploadStatus::Uploading),
                                upload_complete: upload_complete_ids().contains(&item.id)
                                    || item.status == Some(AttachmentUploadStatus::Complete),
                                failed: item.status == Some(AttachmentUploadStatus::Failed),
                                removing: removing_ids().contains(&item.id),
                                arrival_index: uploading_ids()
                                    .iter()
                                    .position(|id| id == &item.id)
                                    .map(|index| index as i32)
                                    .unwrap_or(-1),
                                on_audio_toggle,
                                on_image_preview: EventHandler::new({
                                    move |entry: AttachmentUploadItem| {
                                        let mut preview = preview_item;
                                        preview.set(Some(entry));
                                    }
                                }),
                                on_remove: EventHandler::new({
                                    move |entry: AttachmentUploadItem| request_remove(entry)
                                }),
                                on_retry,
                                reduce,
                                class: row_class.clone(),
                            }
                        }
                    }
                }
            }

            ImagePreviewDialog {
                item: preview_item(),
                on_close: EventHandler::new({
                    move |_| {
                        let mut preview = preview_item;
                        preview.set(None);
                    }
                }),
                reduce,
            }
        }
    }
}
