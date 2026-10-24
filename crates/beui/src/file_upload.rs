//! `motion/file-upload.tsx` — a dropzone with an upload queue.
//!
//! The behaviour is the source's: `clampProgress` (a success is always 100%),
//! `formatBytes`' `value >= 10 || exponent == 0 ? toFixed(0) : toFixed(1)`,
//! `fileKind`, the `getFileIcon` cascade (image, video, audio, archive,
//! spreadsheet, pdf/text, code, then the generic file), the `maxFiles` slot
//! arithmetic with `multiple` deciding how many of them are taken, `addFiles`,
//! `removeItem`, `retryItem` (clearing the error, resetting progress and going
//! back to uploading), the drag-depth counter that keeps the highlight while the
//! pointer crosses child elements, and the `data-[dragging=true]` border.
//!
//! No interop is needed for the files themselves: Dioxus hands both the
//! `drop` event and the file input a `FileData` with `name()` / `size()` /
//! `content_type()` already resolved, so `createFileUploadItem` reads them the
//! same way the source reads the DOM `File`.
//!
//! One difference: the source keeps the DOM `File` on each item for the consumer
//! to upload later; here the handles are handed over once through
//! `on_files_added`, since `FileData` is not `PartialEq` and cannot live in
//! a Dioxus prop.

use crate::icons::{
    AlertCircle, CheckCircle2, FileArchive, FileAudio, FileCode2, FileIcon, FileImage,
    FileSpreadsheet, FileText, FileVideo, LoaderCircle, RotateCcw, UploadCloud, X,
};
use crate::prelude::*;
use beui_core::platform::{js_string, json, next_scope_id};
use dioxus::html::{DragData, FileData, HasFileData};
use dioxus_motion::prelude::{use_motion, AnimationManager, MotionHandle};

const ROW_ANIMATIONS: &str = "\
@keyframes beui-upload-row{\
0%{opacity:0;transform:translateY(8px)}\
100%{opacity:1;transform:translateY(0)}}\
@keyframes beui-upload-icon{\
0%{opacity:0;transform:translateY(4px)}\
100%{opacity:1;transform:translateY(0)}}";

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// Where a file is in the queue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileUploadStatus {
    #[default]
    Queued,
    Uploading,
    Success,
    Error,
}

impl FileUploadStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Uploading => "Uploading",
            Self::Success => "Uploaded",
            Self::Error => "Failed",
        }
    }

    fn tone(self) -> &'static str {
        match self {
            Self::Queued => "text-muted-foreground",
            Self::Uploading => "text-foreground",
            Self::Success => "text-emerald-600 dark:text-emerald-400",
            Self::Error => "text-destructive",
        }
    }
}

/// The dropzone's shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileUploadVariant {
    #[default]
    Default,
    Centered,
}

/// One queued file.
#[derive(Clone, Debug, PartialEq)]
pub struct FileUploadItem {
    pub id: String,
    pub name: String,
    pub size: u64,
    /// MIME type, when the browser reported one.
    pub mime: Option<String>,
    pub progress: Option<f64>,
    pub status: FileUploadStatus,
    pub error: Option<String>,
}

impl FileUploadItem {
    /// A queued file, measuring nothing yet.
    pub fn new(id: impl Into<String>, name: impl Into<String>, size: u64) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            size,
            mime: None,
            progress: None,
            status: FileUploadStatus::Queued,
            error: None,
        }
    }

    /// Set the MIME type.
    pub fn mime(mut self, mime: impl Into<String>) -> Self {
        self.mime = Some(mime.into());
        self
    }

    /// Set the upload status.
    pub fn status(mut self, status: FileUploadStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the progress percentage.
    pub fn progress(mut self, progress: f64) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Set the failure message.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

/// An added file, with the handle the consumer needs to upload it.
#[derive(Clone)]
pub struct FileUploadAdded {
    pub item: FileUploadItem,
    pub file: FileData,
}

/// `createFileUploadItem`.
fn item_from_file(file: &FileData, index: usize) -> FileUploadItem {
    let now = beui_core::platform::now_ms();
    let mime = file.content_type();
    let mut item = FileUploadItem::new(
        format!("{}-{}-{}", now as u64, index, file.name()),
        file.name(),
        file.size(),
    )
    .status(FileUploadStatus::Uploading)
    .progress(0.0);
    if let Some(mime) = mime {
        item = item.mime(mime);
    }
    item
}

/// `clampProgress`.
fn clamp_progress(value: Option<f64>, status: FileUploadStatus) -> f64 {
    if status == FileUploadStatus::Success {
        return 100.0;
    }
    match value {
        None => 0.0,
        Some(value) if value.is_nan() => 0.0,
        Some(value) => value.clamp(0.0, 100.0),
    }
}

/// `formatBytes`.
fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let bytes = bytes as f64;
    let exponent = ((bytes.ln() / 1024f64.ln()).floor() as usize).min(units.len() - 1);
    let value = bytes / 1024f64.powi(exponent as i32);
    let text = if value >= 10.0 || exponent == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    };
    format!("{text} {}", units[exponent])
}

/// `fileKind`.
fn file_kind(item: &FileUploadItem) -> String {
    if let Some(extension) = item.name.rsplit_once('.').map(|(_, ext)| ext) {
        return extension.to_uppercase();
    }
    if let Some(mime) = &item.mime {
        if let Some(subtype) = mime.split('/').next_back() {
            return subtype.to_uppercase();
        }
    }
    "FILE".to_string()
}

/// `getFileIcon`.
fn file_icon(item: &FileUploadItem) -> Element {
    let extension = item
        .name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_lowercase())
        .unwrap_or_default();
    let mime = item.mime.clone().unwrap_or_default();
    let class = Some("h-5 w-5".to_string());

    if mime.starts_with("image/") {
        return rsx! { FileImage { class } };
    }
    if mime.starts_with("video/") {
        return rsx! { FileVideo { class } };
    }
    if mime.starts_with("audio/") {
        return rsx! { FileAudio { class } };
    }
    if mime.contains("zip")
        || mime.contains("compressed")
        || ["zip", "rar", "7z", "tar", "gz"].contains(&extension.as_str())
    {
        return rsx! { FileArchive { class } };
    }
    if mime.contains("spreadsheet")
        || mime.contains("excel")
        || ["csv", "xls", "xlsx"].contains(&extension.as_str())
    {
        return rsx! { FileSpreadsheet { class } };
    }
    if mime.contains("pdf")
        || mime.starts_with("text/")
        || ["pdf", "doc", "docx", "md", "txt"].contains(&extension.as_str())
    {
        return rsx! { FileText { class } };
    }
    if [
        "css", "html", "js", "jsx", "json", "mdx", "ts", "tsx", "xml", "yaml", "yml",
    ]
    .contains(&extension.as_str())
    {
        return rsx! { FileCode2 { class } };
    }

    rsx! { FileIcon { class } }
}

/// Props for [`FileUpload`].
#[derive(Props, Clone, PartialEq)]
pub struct FileUploadProps {
    #[props(default)]
    pub value: Option<Vec<FileUploadItem>>,
    #[props(default)]
    pub default_value: Vec<FileUploadItem>,
    #[props(default)]
    pub on_value_change: Option<EventHandler<Vec<FileUploadItem>>>,
    /// The added files, with the handles to upload.
    #[props(default)]
    pub on_files_added: Option<EventHandler<Vec<FileUploadAdded>>>,
    #[props(default)]
    pub on_remove: Option<EventHandler<FileUploadItem>>,
    #[props(default)]
    pub on_retry: Option<EventHandler<FileUploadItem>>,
    #[props(default)]
    pub accept: Option<String>,
    #[props(default = true)]
    pub multiple: bool,
    #[props(default)]
    pub max_files: Option<usize>,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub variant: FileUploadVariant,
    #[props(default = "Drop files here".to_string())]
    pub title: String,
    #[props(default = "Add files to the upload queue".to_string())]
    pub description: String,
    #[props(default = "Browse".to_string())]
    pub browse_label: String,
    #[props(default)]
    pub class: Option<String>,
}

/// A file dropzone with a queue.
#[component]
pub fn FileUpload(props: FileUploadProps) -> Element {
    let FileUploadProps {
        value,
        default_value,
        on_value_change,
        on_files_added,
        on_remove,
        on_retry,
        accept,
        multiple,
        max_files,
        disabled,
        variant,
        title,
        description,
        browse_label,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let scope = use_hook(next_scope_id);
    let input_selector = format!("[data-beui-upload-input=\"{scope}\"]");
    let controlled = value.is_some();
    let mut internal = use_signal(|| default_value.clone());
    let items = value.clone().unwrap_or(internal());
    let dragging = use_signal(|| false);
    let drag_depth = use_signal(|| 0i32);

    let commit = {
        move |next: Vec<FileUploadItem>| {
            if !controlled {
                internal.set(next.clone());
            }
            if let Some(handler) = on_value_change {
                handler.call(next);
            }
        }
    };
    let max_reached = max_files.is_some_and(|max| items.len() >= max);
    let blocked = disabled || max_reached;

    let add_files = {
        let items = items.clone();
        let mut commit = commit;
        move |incoming: Vec<FileData>| {
            if blocked || incoming.is_empty() {
                return;
            }
            let remaining = match max_files {
                None => incoming.len(),
                Some(max) => max.saturating_sub(items.len()),
            };
            if remaining == 0 {
                return;
            }
            let take = if multiple {
                remaining
            } else {
                remaining.min(1)
            };
            let files: Vec<FileData> = incoming.into_iter().take(take).collect();
            let added: Vec<FileUploadItem> = files
                .iter()
                .enumerate()
                .map(|(index, file)| item_from_file(file, index))
                .collect();
            if added.is_empty() {
                return;
            }
            let mut next = items.clone();
            next.extend(added.iter().cloned());
            commit(next);
            if let Some(handler) = on_files_added {
                handler.call(
                    added
                        .into_iter()
                        .zip(files)
                        .map(|(item, file)| FileUploadAdded { item, file })
                        .collect(),
                );
            }
        }
    };
    let remove_item = {
        let items = items.clone();
        let mut commit = commit;
        move |item: FileUploadItem| {
            commit(
                items
                    .iter()
                    .filter(|entry| entry.id != item.id)
                    .cloned()
                    .collect(),
            );
            if let Some(handler) = on_remove {
                handler.call(item);
            }
        }
    };
    let retry_item = {
        let items = items.clone();
        let mut commit = commit;
        move |item: FileUploadItem| {
            let retrying = FileUploadItem {
                error: None,
                progress: Some(0.0),
                status: FileUploadStatus::Uploading,
                ..item.clone()
            };
            commit(
                items
                    .iter()
                    .map(|entry| {
                        if entry.id == item.id {
                            retrying.clone()
                        } else {
                            entry.clone()
                        }
                    })
                    .collect(),
            );
            if let Some(handler) = on_retry {
                handler.call(retrying);
            }
        }
    };

    let centered = variant == FileUploadVariant::Centered;
    let dropzone_class = cn!(
        "group relative flex w-full overflow-hidden rounded-3xl border border-dashed border-border bg-background outline-none",
        "transition-[border-color,transform] duration-200 active:scale-[0.99]",
        "hover:border-foreground/40 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        if dragging() { "border-foreground" } else { "" },
        "disabled:pointer-events-none disabled:opacity-55",
        if centered {
            "min-h-56 flex-col items-center justify-center gap-3 p-7 text-center"
        } else {
            "items-center gap-4 p-5 text-left"
        },
        class,
    );
    let icon_box_class = cn!(
        "grid shrink-0 place-items-center bg-muted text-foreground",
        if centered {
            "h-16 w-16 rounded-[1.35rem] border border-border"
        } else {
            "h-14 w-14 rounded-[1.25rem]"
        },
    );
    let icon_style = if reduce_value {
        String::new()
    } else {
        format!(
            "transform:translateY({}px);transition:transform 160ms ease-out",
            if dragging() { -2.0 } else { 0.0 }
        )
    };

    rsx! {
        style { dangerous_inner_html: ROW_ANIMATIONS }
        div { class: "w-full space-y-3",
            input {
                "data-beui-upload-input": "{scope}",
                r#type: "file",
                aria_label: "Upload files",
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
                "data-dragging": if dragging() { "true" } else { "false" },
                class: dropzone_class,
                // The zone opens the native picker; the input is in the DOM
                // either way, so a script click is enough.
                onclick: {
                    let selector = input_selector.clone();
                    move |_| {
                        let selector = selector.clone();
                        spawn(async move {
                            let script = format!(
                                "const el = document.querySelector({});\nif (el) el.click();\nreturn true;",
                                js_string(&selector)
                            );
                            let _ = json::<bool>(&script).await;
                        });
                    }
                },
                ondragenter: {
                    let mut dragging = dragging;
                    let mut drag_depth = drag_depth;
                    move |event: Event<DragData>| {
                        if blocked {
                            return;
                        }
                        event.prevent_default();
                        drag_depth += 1;
                        dragging.set(true);
                    }
                },
                ondragover: {
                    let mut dragging = dragging;
                    move |event: Event<DragData>| {
                        if blocked {
                            return;
                        }
                        event.prevent_default();
                        dragging.set(true);
                    }
                },
                ondragleave: {
                    let mut dragging = dragging;
                    let mut drag_depth = drag_depth;
                    move |event: Event<DragData>| {
                        if blocked {
                            return;
                        }
                        event.prevent_default();
                        drag_depth.set((drag_depth() - 1).max(0));
                        if drag_depth() == 0 {
                            dragging.set(false);
                        }
                    }
                },
                ondrop: {
                    let mut dragging = dragging;
                    let mut drag_depth = drag_depth;
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
                span { aria_hidden: "true", class: icon_box_class, style: icon_style,
                    UploadCloud { class: Some(if centered { "h-7 w-7".to_string() } else { "h-6 w-6".to_string() }) }
                }
                span { class: cn!("min-w-0", if centered { "max-w-xs" } else { "flex-1" }),
                    span {
                        class: cn!("block font-semibold text-foreground", if centered { "text-base" } else { "text-sm" }),
                        if max_reached { "Upload limit reached" } else { "{title}" }
                    }
                    span {
                        class: cn!("block text-xs text-muted-foreground", if centered { "mt-1 leading-5" } else { "mt-0.5" }),
                        if max_reached {
                            "{items.len()} of {max_files.unwrap_or_default()} files added"
                        } else {
                            "{description}"
                        }
                    }
                }
                span {
                    class: cn!(
                        "shrink-0 rounded-full border border-border text-xs font-medium text-foreground transition-colors duration-150 group-hover:bg-muted",
                        if centered { "mt-1 px-4 py-2" } else { "px-3.5 py-2" },
                    ),
                    "{browse_label}"
                }
            }
            ul { class: "space-y-2",
                for item in items.iter() {
                    FileUploadRow {
                        key: "{item.id}",
                        item: item.clone(),
                        reduce: reduce_value,
                        on_remove: EventHandler::new({
                            let mut remove_item = remove_item.clone();
                            move |item: FileUploadItem| remove_item(item)
                        }),
                        on_retry: EventHandler::new({
                            let mut retry_item = retry_item.clone();
                            move |item: FileUploadItem| retry_item(item)
                        }),
                    }
                }
            }
        }
    }
}

/// One row of the queue.
#[component]
fn FileUploadRow(
    item: FileUploadItem,
    reduce: bool,
    on_remove: EventHandler<FileUploadItem>,
    on_retry: EventHandler<FileUploadItem>,
) -> Element {
    let status = item.status;
    let progress = clamp_progress(item.progress, status);
    let progress_ratio = progress / 100.0;
    let show_progress = matches!(
        status,
        FileUploadStatus::Uploading | FileUploadStatus::Success
    );
    let row_class = "relative overflow-hidden rounded-2xl border border-border bg-background p-3";

    rsx! {
        li {
            class: row_class,
            style: if reduce {
                String::new()
            } else {
                "animation:beui-upload-row 220ms ease-out both".to_string()
            },
            div { class: "flex items-center gap-3",
                div { class: "grid h-11 w-11 shrink-0 place-items-center rounded-xl bg-muted text-muted-foreground",
                    {file_icon(&item)}
                }
                div { class: "min-w-0 flex-1",
                    div { class: "flex items-start justify-between gap-3",
                        div { class: "min-w-0",
                            p { class: "truncate text-sm font-medium text-foreground", "{item.name}" }
                            p { class: "mt-0.5 text-xs text-muted-foreground",
                                "{file_kind(&item)} · {format_bytes(item.size)}"
                                if status == FileUploadStatus::Error {
                                    if let Some(error) = item.error.clone() {
                                        " · {error}"
                                    }
                                }
                            }
                        }
                        div { class: "flex shrink-0 items-center gap-1",
                            span {
                                key: "{status.label()}",
                                class: cn!("grid h-6 w-6 place-items-center", status.tone()),
                                style: if reduce {
                                    String::new()
                                } else {
                                    "animation:beui-upload-icon 160ms ease-out both".to_string()
                                },
                                match status {
                                    FileUploadStatus::Success => rsx! { CheckCircle2 { class: Some("h-4 w-4".to_string()) } },
                                    FileUploadStatus::Error => rsx! { AlertCircle { class: Some("h-4 w-4".to_string()) } },
                                    FileUploadStatus::Uploading => rsx! { LoaderCircle { class: Some(cn!("h-4 w-4", if reduce { "" } else { "animate-spin" })) } },
                                    FileUploadStatus::Queued => rsx! { FileIcon { class: Some("h-4 w-4".to_string()) } },
                                }
                                span { class: "sr-only", "{status.label()}" }
                            }
                            if status == FileUploadStatus::Error {
                                button {
                                    r#type: "button",
                                    aria_label: "Retry {item.name}",
                                    class: "grid h-7 w-7 place-items-center rounded-full text-muted-foreground transition-colors duration-150 hover:bg-muted hover:text-foreground active:scale-95",
                                    onclick: {
                                        let item = item.clone();
                                        move |_| on_retry.call(item.clone())
                                    },
                                    RotateCcw { class: Some("h-3.5 w-3.5".to_string()) }
                                }
                            }
                            button {
                                r#type: "button",
                                aria_label: "Remove {item.name}",
                                class: "grid h-7 w-7 place-items-center rounded-full text-muted-foreground transition-colors duration-150 hover:bg-muted hover:text-foreground active:scale-95",
                                onclick: {
                                    let item = item.clone();
                                    move |_| on_remove.call(item.clone())
                                },
                                X { class: Some("h-3.5 w-3.5".to_string()) }
                            }
                        }
                    }
                    if show_progress {
                        div {
                            role: "progressbar",
                            aria_valuemin: "0",
                            aria_valuemax: "100",
                            aria_valuenow: "{progress.round()}",
                            aria_label: "{item.name} upload progress",
                            class: "mt-3 h-1.5 overflow-hidden rounded-full bg-muted",
                            ProgressFill {
                                ratio: progress_ratio,
                                success: status == FileUploadStatus::Success,
                                reduce,
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The progress bar's fill.
#[component]
fn ProgressFill(ratio: f64, success: bool, reduce: bool) -> Element {
    // Seeded at the current ratio, like the source's `initial={false}`: a row
    // that mounts mid-upload shows its progress immediately and only *changes*
    // tween. `ratio` is a plain prop, so it has to be declared through
    // `use_reactive` or the fill would never follow an update.
    let mut scale = use_motion(ratio as f32).expect("ratio is finite");
    use_effect(use_reactive((&ratio,), move |(ratio,)| {
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            beui_core::ease::tween_ease_out(280)
        };
        let _ = scale.animate_to(ratio as f32, config);
    }));

    let scale_value = f64::from(current(scale));

    rsx! {
        div {
            class: cn!(
                "h-full origin-left rounded-full",
                if success { "bg-emerald-500" } else { "bg-foreground" },
            ),
            style: "transform:scaleX({scale_value})",
        }
    }
}
