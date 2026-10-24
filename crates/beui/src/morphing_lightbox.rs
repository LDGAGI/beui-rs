//! `motion/morphing-lightbox.tsx` — a thumbnail gallery whose images morph into
//! a full-screen, focus-managed viewer.
//!
//! Parts: [`ImageViewer`] (aliased [`MorphingLightbox`]), [`ImageViewerGallery`],
//! [`ImageViewerThumbnail`], [`ImageViewerContent`], [`ImageViewerCounter`],
//! [`ImageViewerCaption`], [`ImageViewerClose`] / [`ImageViewerPrevious`] /
//! [`ImageViewerNext`], and [`use_image_viewer`] for custom controls.
//!
//! Deviations from the source, all forced by the platform:
//!
//! - **State travels in signals, not a re-provided context.** A Dioxus context
//!   is provided once, so every value that changes (the selection, the images,
//!   the label, reduced motion) lives in a `Signal` inside [`ViewerContext`],
//!   mirrored from the prop each render. `render_caption` is the exception: a
//!   closure cannot be mirrored cheaply, so it is read from the first render —
//!   use `ImageViewerCaption`'s children when the caption must be dynamic.
//! - **The shared-layout morph is a one-way [`beui_core::flip`] handoff.** The
//!   thumbnail carries `layout_id = "{group}-{id}"` and the viewer's image box
//!   takes the same id over, so opening morphs out of the thumbnail. Closing
//!   has no owner to hand the id back to once the box unmounts, so the exit is
//!   the backdrop fade (the limitation `AttachmentUpload` records as well).
//! - **The image slide animates a percentage transform** through
//!   `MotionStyle::add_css_property("transform", "translateX(±100%)")`, because
//!   `MotionStyle`'s own `x` channel is pixels only. `IMAGE_SLIDE_TRANSITION`
//!   stays a tween, for the reason the source gives.
//! - **Swiping stays hand-rolled** on pointer down/up (the 50 px and 1.5× ratio
//!   gate, verbatim), while panning goes through [`beui_core::drag`] with the
//!   source's constraints (double-size bounds, elastic 0.08, no momentum); its
//!   `dragListener={zoom}` becomes "only feed the drag while zoomed".
//! - **Click-outside-to-close** is the modal scope plus an explicit handler on
//!   the frame's letterbox area, rather than a `closest()` walk over the event
//!   target (a Dioxus pointer event does not expose its target).
//! - **Invalid props do not throw.** The source throws on duplicate ids or
//!   non-positive dimensions; the viewer logs and renders nothing (the
//!   `AspectRatio` precedent). The same is true for an unknown thumbnail id.
//! - **`PortalHost` + `use_modal_scope` replace `createPortal` + `useModalScope`**,
//!   and the viewer is client-only (no `mounted` gate): the gallery is a
//!   client-rendered app.

use std::collections::BTreeSet;

use beui_core::drag::{use_drag, Axes, DragElastic, DragOptions, DragRange};
use beui_core::ease::{tween_ease_out, SPRING_LAYOUT, SPRING_PRESS};
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::{use_dismiss, use_modal_scope, DismissOptions};
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

use crate::motion::{number, spring, tween, use_number_motion, use_style_motion, Ease, PortalHost};
use crate::prelude::*;
use crate::rect::use_element_size;

// --- public types -----------------------------------------------------------

/// `LightboxImage`.
#[derive(Clone, PartialEq, Debug)]
pub struct LightboxImage {
    pub id: String,
    pub src: String,
    pub alt: String,
    /// Intrinsic width, kept through the thumbnail morph.
    pub width: f64,
    /// Intrinsic height, kept through the thumbnail morph.
    pub height: f64,
    pub caption: Option<String>,
}

impl LightboxImage {
    pub fn new(
        id: impl Into<String>,
        src: impl Into<String>,
        alt: impl Into<String>,
        width: f64,
        height: f64,
    ) -> Self {
        Self {
            id: id.into(),
            src: src.into(),
            alt: alt.into(),
            width,
            height,
            caption: None,
        }
    }

    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// The caption the default renderer shows: `caption ?? alt`.
    pub fn caption_text(&self) -> String {
        self.caption.clone().unwrap_or_else(|| self.alt.clone())
    }
}

/// The validation the source does with `throw`.
pub fn validate_images(images: &[LightboxImage]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for image in images {
        if !ids.insert(image.id.clone()) {
            return Err("ImageViewer requires unique image ids.".to_string());
        }
        if !image.width.is_finite()
            || !image.height.is_finite()
            || image.width <= 0.0
            || image.height <= 0.0
        {
            return Err("ImageViewer requires positive image dimensions.".to_string());
        }
    }
    Ok(())
}

// --- context ----------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
struct ViewerContext {
    images: Signal<Vec<LightboxImage>>,
    value: Signal<Option<String>>,
    label: Signal<String>,
    /// The stable `useId()` prefix shared by the thumbnail and viewer layout ids.
    group: Signal<String>,
    reduce: Signal<bool>,
    /// Read from the first render; see the module docs.
    render_caption: Option<Callback<LightboxImage, Element>>,
    select: EventHandler<Option<String>>,
    move_by: EventHandler<i8>,
    /// Mirrors `image.is_some()`; the scopes need it as a `Signal`.
    open: Signal<bool>,
}

fn viewer_context(part: &str) -> Option<ViewerContext> {
    let context = try_consume_context::<ViewerContext>();
    if context.is_none() {
        dioxus::logger::tracing::error!("{part} must be used within ImageViewer.");
    }
    context
}

fn current_index(context: &ViewerContext) -> Option<usize> {
    let value = (context.value)();
    let images = (context.images)();
    value
        .as_ref()
        .and_then(|id| images.iter().position(|image| &image.id == id))
}

fn current_image(context: &ViewerContext) -> Option<LightboxImage> {
    let index = current_index(context)?;
    (context.images)().get(index).cloned()
}

/// `useImageViewer` — selection and navigation for custom controls.
#[derive(Clone, Copy, PartialEq)]
pub struct ImageViewerApi {
    context: ViewerContext,
}

impl ImageViewerApi {
    pub fn images(&self) -> Vec<LightboxImage> {
        (self.context.images)()
    }

    pub fn value(&self) -> Option<String> {
        (self.context.value)()
    }

    /// `None` while the viewer is closed.
    pub fn image(&self) -> Option<LightboxImage> {
        current_image(&self.context)
    }

    /// `-1` while the viewer is closed, as in the source.
    pub fn index(&self) -> i64 {
        current_index(&self.context).map_or(-1, |index| index as i64)
    }

    pub fn has_previous(&self) -> bool {
        self.index() > 0
    }

    pub fn has_next(&self) -> bool {
        let index = self.index();
        index >= 0 && index < self.images().len() as i64 - 1
    }

    pub fn select(&self, id: Option<String>) {
        self.context.select.call(id);
    }

    pub fn close(&self) {
        self.context.select.call(None);
    }

    pub fn previous(&self) {
        self.context.move_by.call(-1);
    }

    pub fn next(&self) {
        self.context.move_by.call(1);
    }
}

/// `useImageViewer()` — call inside an [`ImageViewer`] subtree.
pub fn use_image_viewer() -> Option<ImageViewerApi> {
    viewer_context("useImageViewer").map(|context| ImageViewerApi { context })
}

// --- viewer -----------------------------------------------------------------

/// `ImageViewer` / `MorphingLightbox`.
#[component]
pub fn ImageViewer(
    images: Vec<LightboxImage>,
    #[props(default = None)] value: Option<String>,
    #[props(default = None)] default_value: Option<String>,
    #[props(default = None)] on_value_change: Option<EventHandler<Option<String>>>,
    #[props(default = None)] label: Option<String>,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] thumbnail_class: Option<String>,
    #[props(default = None)] render_caption: Option<Callback<LightboxImage, Element>>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    if let Err(message) = validate_images(&images) {
        dioxus::logger::tracing::error!("{message}");
        return VNode::empty();
    }

    let reduce = use_reduced_motion();
    let controlled = value.is_some();

    // Every changing value lives in a signal: the context is provided once.
    let mut images_signal = use_signal(|| images.clone());
    if *images_signal.peek() != images {
        images_signal.set(images.clone());
    }
    let mut value_signal = use_signal(|| value.clone().or_else(|| default_value.clone()));
    if let Some(controlled_value) = value.as_ref() {
        if value_signal.peek().as_ref() != Some(controlled_value) {
            value_signal.set(Some(controlled_value.clone()));
        }
    }
    let label_text = label.unwrap_or_else(|| "Image gallery".to_string());
    let mut label_signal = use_signal(|| label_text.clone());
    if *label_signal.peek() != label_text {
        label_signal.set(label_text);
    }
    let group_text = use_hook(|| format!("beui-lightbox-{}", platform::next_scope_id()));
    let group_signal = use_signal(|| group_text);
    let mut open_signal = use_signal(|| value_signal.peek().is_some());
    let open_now = value_signal.peek().is_some();
    if *open_signal.peek() != open_now {
        open_signal.set(open_now);
    }
    let reduce_signal = reduce;

    let select = EventHandler::new(move |next: Option<String>| {
        // The signal is the store in both modes, so a parent that ignores the
        // callback cannot freeze the selection.
        value_signal.set(next.clone());
        if let Some(callback) = on_value_change {
            callback.call(next);
        }
    });
    let move_by = EventHandler::new(move |direction: i8| {
        let images = images_signal.peek().clone();
        let index = value_signal
            .peek()
            .as_ref()
            .and_then(|id| images.iter().position(|image| &image.id == id));
        let Some(index) = index else {
            return;
        };
        let target = if direction < 0 {
            index.checked_sub(1)
        } else {
            Some(index + 1)
        };
        if let Some(image) = target.and_then(|target| images.get(target)) {
            select.call(Some(image.id.clone()));
        }
    });

    let _ = controlled;
    let context = ViewerContext {
        images: images_signal,
        value: value_signal,
        label: label_signal,
        group: group_signal,
        reduce: reduce_signal,
        render_caption,
        select,
        move_by,
        open: open_signal,
    };
    let _ = use_context_provider(|| context);

    match children {
        Some(children) => rsx! { {children} },
        None => {
            let gallery_images = (context.images)();
            let thumbnail_class = thumbnail_class.clone();
            rsx! {
                ImageViewerGallery { class: class.clone(),
                    for image in gallery_images.iter() {
                        ImageViewerThumbnail {
                            key: "{image.id}",
                            image_id: image.id.clone(),
                            class: thumbnail_class.clone(),
                        }
                    }
                }
                ImageViewerContent {}
            }
        }
    }
}

/// `ImageViewerGallery`.
#[component]
pub fn ImageViewerGallery(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    let Some(context) = viewer_context("ImageViewerGallery") else {
        return VNode::empty();
    };
    let label = (context.label)();
    rsx! {
        section {
            aria_label: "{label}",
            class: cn!("grid w-full grid-cols-2 gap-3 sm:grid-cols-3", class),
            if let Some(children) = children {
                {children}
            }
        }
    }
}

/// `ImageViewerThumbnail`.
#[component]
pub fn ImageViewerThumbnail(
    image_id: String,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] image_class: Option<String>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    let Some(context) = viewer_context("ImageViewerThumbnail") else {
        return VNode::empty();
    };
    let Some(image) = (context.images)()
        .into_iter()
        .find(|image| image.id == image_id)
    else {
        dioxus::logger::tracing::error!("ImageViewerThumbnail: unknown image id \"{image_id}\".");
        return VNode::empty();
    };

    let value = (context.value)();
    let group = (context.group)();
    let reduce = (context.reduce)();
    let selected = value.as_deref() == Some(image.id.as_str());
    let flip = use_flip(
        value,
        FlipOptions::default()
            .layout_id(format!("{group}-{}", image.id))
            .transition(spring(SPRING_LAYOUT)),
    );
    let mut scale = use_number_motion(1.0);
    let release = move |_: Event<PointerData>| {
        if !reduce {
            animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
        }
    };

    let alt = image.alt.clone();
    let select_id = image.id.clone();
    let src = image.src.clone();
    let width = image.width;
    let height = image.height;
    rsx! {
        button {
            r#type: "button",
            aria_label: "Open {alt}",
            "aria-haspopup": "dialog",
            "aria-expanded": if selected { "true" } else { "false" },
            "data-beui-lightbox-thumbnail": "{image.id}",
            style: flip.style_with(&scale_style(number(&scale))),
            class: cn!(
                "relative overflow-hidden rounded-2xl bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                class,
            ),
            onpointerdown: move |_| {
                if !reduce {
                    animate_number(&mut scale, 0.98, spring(SPRING_PRESS));
                }
            },
            onpointerup: release,
            onpointercancel: release,
            onclick: move |_| context.select.call(Some(select_id.clone())),
            img {
                src: "{src}",
                alt: "{alt}",
                width: "{width}",
                height: "{height}",
                loading: "lazy",
                draggable: "false",
                class: cn!("block h-auto w-full", image_class),
            }
            if let Some(children) = children {
                {children}
            }
        }
    }
}

/// `ImageViewerCounter`.
#[component]
pub fn ImageViewerCounter(#[props(default = None)] class: Option<String>) -> Element {
    let Some(context) = viewer_context("ImageViewerCounter") else {
        return VNode::empty();
    };
    let count = (context.images)().len();
    let index = current_index(&context).map_or(0, |index| index + 1);
    rsx! {
        span {
            "aria-live": "polite",
            "data-beui-lightbox-counter": "",
            class: cn!("text-xs tabular-nums", class),
            "{index} / {count}"
        }
    }
}

/// `ImageViewerCaption`.
#[component]
pub fn ImageViewerCaption(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    let Some(context) = viewer_context("ImageViewerCaption") else {
        return VNode::empty();
    };
    let text = current_image(&context).map(|image| match children.clone() {
        Some(children) => children,
        None => match context.render_caption {
            Some(render) => render.call(image.clone()),
            None => rsx! { "{image.caption_text()}" },
        },
    });
    rsx! {
        div {
            "aria-live": "polite",
            "data-lightbox-content": "",
            "data-beui-lightbox-caption": "",
            class: cn!("min-w-0 text-center text-sm", class),
            if let Some(text) = text {
                {text}
            }
        }
    }
}

/// The shared appearance of the viewer's round controls.
pub const CONTROL_CLASS: &str = "inline-flex size-10 shrink-0 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white disabled:opacity-30";

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewerAction {
    Close,
    Previous,
    Next,
}

#[component]
fn ViewerButton(
    action: ViewerAction,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    let Some(context) = viewer_context("ImageViewer controls") else {
        return VNode::empty();
    };
    let index = current_index(&context);
    let count = (context.images)().len();
    let unavailable = match action {
        ViewerAction::Previous => index.is_none_or(|index| index == 0),
        ViewerAction::Next => index.is_none_or(|index| index + 1 >= count),
        ViewerAction::Close => false,
    };
    let aria_label = match action {
        ViewerAction::Close => "Close viewer",
        ViewerAction::Previous => "Previous image",
        ViewerAction::Next => "Next image",
    };
    rsx! {
        button {
            r#type: "button",
            aria_label: "{aria_label}",
            disabled: unavailable,
            class: cn!(CONTROL_CLASS, class),
            onclick: move |_| match action {
                ViewerAction::Close => context.select.call(None),
                ViewerAction::Previous => context.move_by.call(-1),
                ViewerAction::Next => context.move_by.call(1),
            },
            if let Some(children) = children {
                {children}
            } else if action == ViewerAction::Close {
                X { class: "size-[18px]" }
            } else if action == ViewerAction::Previous {
                IconChevronLeft { class: "size-[18px]" }
            } else {
                IconChevronRight { class: "size-[18px]" }
            }
        }
    }
}

/// `ImageViewerClose`.
#[component]
pub fn ImageViewerClose(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    rsx! {
        ViewerButton { action: ViewerAction::Close, class, children }
    }
}

/// `ImageViewerPrevious`.
#[component]
pub fn ImageViewerPrevious(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    rsx! {
        ViewerButton { action: ViewerAction::Previous, class, children }
    }
}

/// `ImageViewerNext`.
#[component]
pub fn ImageViewerNext(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    rsx! {
        ViewerButton { action: ViewerAction::Next, class, children }
    }
}

// --- presence configs -------------------------------------------------------

fn backdrop_config(reduce: bool) -> PresenceConfig {
    let duration = if reduce { 100 } else { 180 };
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0 },
        motion_style! { opacity: 1.0 },
        motion_style! { opacity: 0.0 },
        tween_ease_out(duration),
        tween_ease_out(duration),
    )
}

/// `imageSwapVariants` — a percentage `translateX` through an animated CSS
/// property (`MotionStyle`'s `x` channel is pixels only), on
/// `IMAGE_SLIDE_TRANSITION`.
fn slide_config(direction: i8, reduce: bool) -> PresenceConfig {
    let enter_x = if reduce {
        0.0
    } else {
        f64::from(direction) * 100.0
    };
    let exit_x = -enter_x;

    let mut enter = motion_style! { opacity: if reduce { 0.0 } else { 1.0 } };
    enter.add_css_property("transform", format!("translateX({enter_x}%)"));
    let mut animate = motion_style! { opacity: 1.0 };
    animate.add_css_property("transform", "translateX(0%)");
    let mut exit = motion_style! { opacity: if reduce { 0.0 } else { 1.0 } };
    exit.add_css_property("transform", format!("translateX({exit_x}%)"));

    let transition = if reduce {
        tween(100, Ease::Out)
    } else {
        tween(280, Ease::Drawer)
    };
    PresenceConfig::with_transitions(enter, animate, exit, transition.clone(), transition)
}

// --- viewer content ---------------------------------------------------------

/// `ImageViewerContent`.
#[component]
pub fn ImageViewerContent(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] header: Option<Element>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    let Some(context) = viewer_context("ImageViewerContent") else {
        return VNode::empty();
    };
    let image = current_image(&context);

    let close = EventHandler::new(move |_: ()| context.select.call(None));
    let scope = use_dismiss(
        context.open,
        close,
        DismissOptions::default().without_escape(),
    );
    use_modal_scope(context.open, close, scope);

    let layers: Vec<()> = if image.is_some() {
        vec![()]
    } else {
        Vec::new()
    };
    let layer_key = "lightbox";

    rsx! {
        PortalHost {
            AnimatePresence {
                for _ in layers {
                    ViewerLayer {
                        key: "{layer_key}",
                        image: image.clone().unwrap_or_else(|| LightboxImage::new("", "", "", 1.0, 1.0)),
                        index: current_index(&context).unwrap_or(0),
                        group: (context.group)(),
                        label: (context.label)(),
                        reduce: (context.reduce)(),
                        class: class.clone(),
                        header: header.clone(),
                        children: children.clone(),
                        scope: scope.attr(),
                    }
                }
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn ViewerLayer(
    image: LightboxImage,
    index: usize,
    group: String,
    label: String,
    reduce: bool,
    class: Option<String>,
    header: Option<Element>,
    children: Option<Element>,
    scope: String,
) -> Element {
    let Some(context) = viewer_context("ImageViewerContent") else {
        return VNode::empty();
    };
    let gate = use_gate();
    let backdrop = use_presence_style(backdrop_config(reduce)).expect("static presence config");
    let backdrop_css = presence_css(&backdrop);

    let select_none = context.select;
    let move_by = context.move_by;

    // Re-target focus when the shown image changes and focus sits nowhere useful
    // (the source's layout effect): the first enabled control keeps the arrows
    // and Enter working after a navigation that disabled the focused control.
    let session = format!("{}-{}", image.id, image.src);
    let mut focus_key = use_signal(|| session.clone());
    if *focus_key.peek() != session {
        focus_key.set(session.clone());
    }
    let focus_session = session.clone();
    let focus_selector = format!("[data-beui-scope=\"{scope}\"]");
    use_effect(use_reactive((&focus_session,), move |_| {
        let selector = focus_selector.clone();
        spawn(async move {
            let script = format!(
                "const panel = document.querySelector({panel});\n\
                 if (!panel) return false;\n\
                 const active = document.activeElement;\n\
                 const inside = active && panel.contains(active);\n\
                 const disabled = active && active.tagName === 'BUTTON' && active.disabled;\n\
                 if (inside && !disabled) return false;\n\
                 const target = panel.querySelector('button:not(:disabled)') ?? panel;\n\
                 target.focus({{ preventScroll: true }});\n\
                 return true;",
                panel = platform::js_string(&selector)
            );
            let _ = platform::json::<bool>(&script).await;
        });
    }));

    let backdrop_style = gate.style_with(&format!(
        "{backdrop_css};position:fixed;inset:0;z-index:100;height:100%;width:100%;cursor:default;background-color:rgb(0 0 0 / 0.9)"
    ));

    rsx! {
        button {
            r#type: "button",
            tabindex: "-1",
            aria_hidden: "true",
            "data-beui-lightbox-backdrop": "",
            style: backdrop_style,
            onclick: move |_| select_none.call(None),
        }
        div {
            inert: gate.inert_attr(),
            class: "pointer-events-none fixed inset-3 z-[101] sm:inset-6",
            div {
                "data-beui-scope": "{scope}",
                role: "dialog",
                "aria-modal": "true",
                aria_label: "{label}",
                tabindex: "-1",
                style: gate.style(),
                class: cn!("flex h-full flex-col outline-none", class),
                onkeydown: move |event: KeyboardEvent| {
                    if event.modifiers().alt()
                        || event.modifiers().ctrl()
                        || event.modifiers().meta()
                        || event.modifiers().shift()
                    {
                        return;
                    }
                    if event.key() == Key::ArrowLeft {
                        event.prevent_default();
                        move_by.call(-1);
                    } else if event.key() == Key::ArrowRight {
                        event.prevent_default();
                        move_by.call(1);
                    }
                },
                div {
                    "data-lightbox-content": "",
                    class: "flex items-center justify-between gap-3 pb-3 text-white",
                    if let Some(header) = header.clone() {
                        {header}
                    } else {
                        ImageViewerCounter {}
                        ImageViewerClose {}
                    }
                }
                LightboxFrame {
                    image: image.clone(),
                    index,
                    group,
                    reduce,
                    present: gate.is_present(),
                }
                div {
                    "data-lightbox-content": "",
                    class: "flex items-center justify-between gap-3 pt-3 text-white",
                    if let Some(children) = children.clone() {
                        {children}
                    } else {
                        ImageViewerPrevious {}
                        ImageViewerCaption {}
                        ImageViewerNext {}
                    }
                }
            }
        }
    }
}

// --- the frame --------------------------------------------------------------

#[component]
fn LightboxFrame(
    image: LightboxImage,
    index: usize,
    group: String,
    reduce: bool,
    present: bool,
) -> Element {
    let Some(context) = viewer_context("Image viewer frame") else {
        return VNode::empty();
    };
    let on_close = context.select;
    let move_by = context.move_by;
    let session = format!("{}-{}", image.id, image.src);

    // The swap direction, and the per-session resets (zoom off, pan home).
    let mut selection = use_signal(|| (session.clone(), index, 1i8));
    let (selection_session, selection_index, direction) = selection();
    let mut zoom = use_signal(|| false);
    let mut failed = use_signal(|| false);
    if selection_session != session {
        let direction = if index < selection_index { -1 } else { 1 };
        selection.set((session.clone(), index, direction));
        failed.set(false);
        zoom.set(false);
    }

    // The pan range: the source measures both boxes and allows half of the
    // overflow at 2×, through a ResizeObserver.
    let bounds = use_element_size();
    let image_box = use_element_size();
    let (bounds_width, bounds_height) = bounds.size();
    let (box_width, box_height) = image_box.size();
    let pan_x = ((box_width * 2.0 - bounds_width) / 2.0).max(0.0);
    let pan_y = ((box_height * 2.0 - bounds_height) / 2.0).max(0.0);
    let drag = use_drag(
        DragOptions::default()
            .range(
                DragRange::new(Some(-pan_x), Some(pan_x)),
                DragRange::new(Some(-pan_y), Some(pan_y)),
            )
            .elastic(DragElastic::uniform(0.08))
            .without_momentum(),
        None,
    );
    use_effect(use_reactive((&session,), move |_| drag.reset()));

    let flip = use_flip(
        session.clone(),
        FlipOptions::default()
            .layout_id(format!("{group}-{}", image.id))
            .transition(spring(SPRING_LAYOUT)),
    );
    let mut zoom_scale = use_style_motion(motion_style! { scale: 1.0 });
    let zoom_now = zoom();
    use_effect(use_reactive!(|zoom_now, present, reduce| {
        let target = motion_style! {
            scale: if zoom_now && present { 2.0 } else { 1.0 },
        };
        let config = if reduce {
            tween(0, Ease::Out)
        } else {
            spring(SPRING_LAYOUT)
        };
        animate_style(&mut zoom_scale, target, config);
    }));

    let offset = drag.offset();
    // A *tracked* read: this is what re-renders the frame once per spring frame.
    let zoom_value = zoom_scale.current().read().scale;
    let image_style = format!(
        "transform:translateX({}px) translateY({}px) scale({zoom_value});touch-action:{};{}",
        offset.x,
        offset.y,
        if zoom_now { "none" } else { "pan-y" },
        if zoom_now { "cursor:grab;" } else { "" },
    );
    let aspect = format!("{} / {}", image.width, image.height);
    let box_style = flip.style_with(&format!(
        "aspect-ratio:{aspect};width:min(100%, calc((100dvh - 160px) * {}))",
        image.width / image.height
    ));

    let mut swipe = use_signal(|| None::<(f64, f64)>);
    let press = move |event: Event<PointerData>| {
        if zoom_now {
            drag.on_pointer_down(&event);
            return;
        }
        // Swiping the image keeps keyboard focus on the controls.
        event.prevent_default();
        if event.is_primary() {
            swipe.set(Some((
                event.client_coordinates().x,
                event.client_coordinates().y,
            )));
        }
    };
    let motion = move |event: Event<PointerData>| {
        if zoom_now {
            drag.on_pointer_move(&event);
        }
    };
    let release = move |event: Event<PointerData>| {
        if zoom_now {
            drag.on_pointer_up(&event);
            return;
        }
        let start = *swipe.peek();
        swipe.set(None);
        let Some((start_x, start_y)) = start else {
            return;
        };
        let dx = event.client_coordinates().x - start_x;
        let dy = event.client_coordinates().y - start_y;
        if dx.abs() > 50.0 && dx.abs() > dy.abs() * 1.5 {
            move_by.call(if dx < 0.0 { 1 } else { -1 });
        }
    };
    let cancel = move |event: Event<PointerData>| {
        if zoom_now {
            drag.on_pointer_cancel(&event);
        } else {
            swipe.set(None);
        }
    };

    let layers: Vec<()> = vec![()];
    let slide_key = session.clone();
    let src = image.src.clone();
    let alt = image.alt.clone();
    let width = image.width;
    let height = image.height;

    rsx! {
        div { class: "relative flex min-h-0 flex-1 items-center justify-center",
            div {
                "data-beui-size": bounds.attr(),
                class: "flex h-full w-full items-center justify-center overflow-hidden rounded-lg",
                // The letterbox area closes the viewer, the way the source's
                // `closest()` walk over the event target does.
                onpointerdown: move |_| on_close.call(None),
                div {
                    "data-beui-flip": flip.scope(),
                    "data-beui-size": image_box.attr(),
                    "data-lightbox-content": "",
                    style: box_style,
                    class: "relative flex max-h-full max-w-full items-center justify-center",
                    onpointerdown: move |event: Event<PointerData>| event.stop_propagation(),
                    div {
                        class: cn!("absolute inset-0", if !zoom_now { "overflow-hidden" } else { "" }),
                        AnimatePresence {
                            initial: false,
                            for _ in layers {
                                LightboxSlide {
                                    key: "{slide_key}",
                                    src: src.clone(),
                                    alt: alt.clone(),
                                    width,
                                    height,
                                    direction,
                                    reduce,
                                    present,
                                    image_style: image_style.clone(),
                                    drag_scope: drag.scope(),
                                    failed,
                                    on_pointer_down: EventHandler::new(press),
                                    on_pointer_move: EventHandler::new(motion),
                                    on_pointer_up: EventHandler::new(release),
                                    on_pointer_cancel: EventHandler::new(cancel),
                                }
                            }
                        }
                    }
                    if failed() {
                        p {
                            role: "status",
                            class: "absolute inset-0 flex items-center justify-center bg-neutral-900 p-4 text-center text-sm text-white",
                            "Unable to load this image."
                        }
                    }
                }
            }
            button {
                r#type: "button",
                "data-beui-lightbox-zoom": "",
                aria_label: if zoom_now { "Zoom out" } else { "Zoom in" },
                aria_pressed: zoom_now,
                class: cn!(CONTROL_CLASS, "absolute bottom-3 right-3 bg-black/60"),
                onkeydown: move |event: KeyboardEvent| {
                    if !zoom_now {
                        return;
                    }
                    let dx = match event.key() {
                        Key::ArrowLeft => -40.0,
                        Key::ArrowRight => 40.0,
                        _ => 0.0,
                    };
                    let dy = match event.key() {
                        Key::ArrowUp => -40.0,
                        Key::ArrowDown => 40.0,
                        _ => 0.0,
                    };
                    if dx == 0.0 && dy == 0.0 {
                        return;
                    }
                    event.prevent_default();
                    event.stop_propagation();
                    let offset = drag.offset();
                    drag.settle_to(
                        Axes {
                            x: (offset.x + dx).clamp(-pan_x, pan_x),
                            y: (offset.y + dy).clamp(-pan_y, pan_y),
                        },
                        tween_ease_out(0),
                    );
                },
                onclick: move |_| {
                    drag.reset();
                    zoom.set(!zoom_now);
                },
                if zoom_now {
                    IconMinus { class: "size-[18px]" }
                } else {
                    Plus { class: "size-[18px]" }
                }
            }
            span { class: "sr-only",
                "When zoomed in, drag the image to pan, or keep this control focused and use the arrow keys."
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn LightboxSlide(
    src: String,
    alt: String,
    width: f64,
    height: f64,
    direction: i8,
    reduce: bool,
    present: bool,
    image_style: String,
    drag_scope: String,
    failed: Signal<bool>,
    on_pointer_down: EventHandler<Event<PointerData>>,
    on_pointer_move: EventHandler<Event<PointerData>>,
    on_pointer_up: EventHandler<Event<PointerData>>,
    on_pointer_cancel: EventHandler<Event<PointerData>>,
) -> Element {
    let gate = use_gate();
    let handle =
        use_presence_style(slide_config(direction, reduce)).expect("static presence config");
    let css = presence_css(&handle);
    let mut failed = failed;

    rsx! {
        div {
            inert: gate.inert_attr(),
            "data-beui-lightbox-slide": "",
            style: gate.style_with(&css),
            class: "absolute inset-0",
            img {
                src: "{src}",
                alt: "{alt}",
                width: "{width}",
                height: "{height}",
                draggable: "false",
                "data-beui-drag": drag_scope,
                onerror: move |_| {
                    if present {
                        failed.set(true);
                    }
                },
                onpointerdown: move |event: Event<PointerData>| on_pointer_down.call(event),
                onpointermove: move |event: Event<PointerData>| on_pointer_move.call(event),
                onpointerup: move |event: Event<PointerData>| on_pointer_up.call(event),
                onpointercancel: move |event: Event<PointerData>| on_pointer_cancel.call(event),
                style: image_style,
                class: "block h-full w-full select-none object-contain",
            }
        }
    }
}

/// `MorphingLightbox` — the original name for [`ImageViewer`].
#[component]
pub fn MorphingLightbox(
    images: Vec<LightboxImage>,
    #[props(default = None)] value: Option<String>,
    #[props(default = None)] default_value: Option<String>,
    #[props(default = None)] on_value_change: Option<EventHandler<Option<String>>>,
    #[props(default = None)] label: Option<String>,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] thumbnail_class: Option<String>,
    #[props(default = None)] render_caption: Option<Callback<LightboxImage, Element>>,
    #[props(default = None)] children: Option<Element>,
) -> Element {
    rsx! {
        ImageViewer {
            images,
            value,
            default_value,
            on_value_change,
            label,
            class,
            thumbnail_class,
            render_caption,
            children,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(id: &str) -> LightboxImage {
        LightboxImage::new(id, "about:blank", id, 1600.0, 2000.0)
    }

    #[test]
    fn validation_matches_the_sources_throws() {
        assert!(validate_images(&[image("a"), image("b")]).is_ok());
        assert!(validate_images(&[image("a"), image("a")]).is_err());
        let mut zero = image("ok");
        zero.width = 0.0;
        assert!(validate_images(&[zero]).is_err());
        let mut nan = image("ok");
        nan.height = f64::NAN;
        assert!(validate_images(&[nan]).is_err());
    }

    #[test]
    fn caption_falls_back_to_alt() {
        assert_eq!(image("a").caption_text(), "a");
        assert_eq!(image("a").caption("01 / Form").caption_text(), "01 / Form");
    }

    #[test]
    fn the_slide_runs_the_way_the_source_did() {
        // Direction +1: in from the right, out to the left.
        let forward = slide_config(1, false);
        assert!(forward
            .initial
            .properties
            .get("transform")
            .is_some_and(|value| value.to_css().contains("100%")));
        // Reduce drops the slide and keeps only the fade.
        let quiet = slide_config(1, true);
        assert!(quiet
            .initial
            .properties
            .get("transform")
            .is_some_and(|value| value.to_css().contains("0%")));
    }
}
