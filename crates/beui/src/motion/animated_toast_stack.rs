//! `components/motion/animated-toast-stack.tsx` — a stack of toasts.
//!
//! Each toast is a presence child of one `AnimatePresence mode="popLayout"`, so
//! dismissing one pops it out of layout while the siblings move immediately. It
//! enters on the `STACK_SPRING` (420/34/0.75) from 22px below at 96% with an
//! blur, exits to the right in 0.18 s, and swings on `drag="x"` — the card only
//! ever shows 18% of the finger, so the threshold reads the raw gesture, not the
//! damped card.
//!
//! Deviations from the source:
//!
//! - the stack is portaled through `PortalHost` (a `display:contents` container
//!   under the app event root) instead of `createPortal(body)`;
//! - `renderToast`, the per-part `icons` override and `onAnimationComplete`
//!   progress are not ported; `title`/`description` are `String`s rather than
//!   arbitrary nodes, and the action is `action_label` plus `on_action`.

use dioxus::prelude::*;
use dioxus_motion::prelude::{AnimationConfig, Duration, Spring, Time, TimeProvider};

use crate::icons::{BellIcon, CheckIcon, CircleAlertIcon, InfoIcon, LoaderCircleIcon, XIcon};
use crate::motion::PortalHost;
use beui_core::drag::{use_drag, DragAxis, DragElastic, DragEnd, DragOptions, DragRange};
use beui_core::ease::tween_ease_out;
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceMode,
};
use beui_core::{cn, use_reduced_motion};

/// `STACK_SPRING`.
const SPRING_STACK: Spring = Spring {
    stiffness: 420.0,
    damping: 34.0,
    mass: 0.75,
};

/// `"loading"` widgets spin; the rest are static.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastStatus {
    #[default]
    Neutral,
    Info,
    Loading,
    Success,
    Error,
}

impl ToastStatus {
    const fn key(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Info => "info",
            Self::Loading => "loading",
            Self::Success => "success",
            Self::Error => "error",
        }
    }

    /// `STATUS_CLASS`, verbatim.
    const fn class(self) -> &'static str {
        match self {
            Self::Neutral => "text-muted-foreground bg-muted/60",
            Self::Info => "text-primary bg-muted",
            Self::Loading => "text-primary bg-muted",
            Self::Success => "text-emerald-600 bg-emerald-500/10 dark:text-emerald-400",
            Self::Error => "text-destructive bg-destructive/10",
        }
    }

    fn glyph(self) -> Element {
        match self {
            Self::Neutral => rsx! { BellIcon { class: "h-3.5 w-3.5" } },
            Self::Info => rsx! { InfoIcon { class: "h-3.5 w-3.5" } },
            Self::Loading => rsx! { LoaderCircleIcon { class: "h-3.5 w-3.5" } },
            Self::Success => rsx! { CheckIcon { class: "h-3.5 w-3.5" } },
            Self::Error => rsx! { CircleAlertIcon { class: "h-3.5 w-3.5" } },
        }
    }
}

/// `ToastPosition` — the six corners the stack can sit in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    #[default]
    BottomRight,
}

impl ToastPosition {
    const fn class(self) -> &'static str {
        match self {
            Self::TopLeft => "left-4 top-4",
            Self::TopCenter => "left-1/2 top-4 -translate-x-1/2",
            Self::TopRight => "right-4 top-4",
            Self::BottomLeft => "bottom-6 left-4",
            Self::BottomCenter => "bottom-6 left-1/2 -translate-x-1/2",
            Self::BottomRight => "bottom-6 right-4",
        }
    }

    const fn is_bottom(self) -> bool {
        matches!(
            self,
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight
        )
    }
}

/// `placement`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastPlacement {
    #[default]
    Static,
    Fixed,
    Absolute,
}

/// One toast.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimatedToast {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: ToastStatus,
    /// Auto-dismiss after this many milliseconds; `<= 0` is sticky.
    pub duration: i64,
    pub dismissible: bool,
    pub created_at: f64,
    pub action_label: Option<String>,
}

impl AnimatedToast {
    /// A toast with the defaults `createToast` applies.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            status: ToastStatus::Neutral,
            duration: -1,
            dismissible: true,
            created_at: platform::now_ms(),
            action_label: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn status(mut self, status: ToastStatus) -> Self {
        self.status = status;
        self
    }

    pub fn duration(mut self, duration: i64) -> Self {
        self.duration = duration;
        self
    }

    pub fn action_label(mut self, label: impl Into<String>) -> Self {
        self.action_label = Some(label.into());
        self
    }
}

/// Per-part class overrides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToastClassNames {
    pub root: Option<String>,
    pub item: Option<String>,
    pub surface: Option<String>,
    pub icon_wrap: Option<String>,
    pub content: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub action: Option<String>,
    pub close: Option<String>,
}

/// Options for [`use_animated_toast_stack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimatedToastStackOptions {
    pub default_duration: i64,
    pub limit: Option<usize>,
}

impl Default for AnimatedToastStackOptions {
    fn default() -> Self {
        Self {
            default_duration: 4200,
            limit: None,
        }
    }
}

/// One toast's patch for [`AnimatedToastStackHandle::update`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnimatedToastPatch {
    pub status: Option<ToastStatus>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub duration: Option<i64>,
}

/// `useAnimatedToastStack` — the list, its actions, and the default duration.
#[derive(Clone, Copy)]
pub struct AnimatedToastStackHandle {
    pub toasts: Signal<Vec<AnimatedToast>>,
    default_duration: i64,
    limit: Option<usize>,
}

impl AnimatedToastStackHandle {
    /// `showToast(input)` — pushes a toast and returns its id. An empty id is
    /// generated, and `duration < 0` means "use the default duration".
    pub fn show(&self, mut toast: AnimatedToast) -> String {
        if toast.id.is_empty() {
            toast.id = format!("beui-toast-{}", platform::next_scope_id());
        }
        if toast.duration < 0 {
            toast.duration = self.default_duration;
        }
        toast.created_at = platform::now_ms();
        let id = toast.id.clone();
        let mut signal = self.toasts;
        let mut list = signal.write();
        list.push(toast);
        if let Some(limit) = self.limit {
            if list.len() > limit {
                let excess = list.len() - limit;
                list.drain(0..excess);
            }
        }
        id
    }

    /// `dismissToast(id)`.
    pub fn dismiss(&self, id: &str) {
        let mut signal = self.toasts;
        signal.write().retain(|toast| toast.id != id);
    }

    /// `clearToasts()`.
    pub fn clear(&self) {
        let mut signal = self.toasts;
        signal.write().clear();
    }

    /// `updateToast(id, patch)` — a duration change restarts the clock, the
    /// way the source resets `createdAt`.
    pub fn update(&self, id: &str, patch: AnimatedToastPatch) {
        let mut signal = self.toasts;
        let mut list = signal.write();
        for toast in list.iter_mut().filter(|toast| toast.id == id) {
            if let Some(status) = patch.status {
                toast.status = status;
            }
            if let Some(title) = &patch.title {
                toast.title = title.clone();
            }
            if let Some(description) = &patch.description {
                toast.description = Some(description.clone());
            }
            if let Some(duration) = patch.duration {
                toast.duration = duration;
                toast.created_at = platform::now_ms();
            }
        }
    }
}

/// `useAnimatedToastStack`.
pub fn use_animated_toast_stack(options: AnimatedToastStackOptions) -> AnimatedToastStackHandle {
    let toasts = use_signal(Vec::<AnimatedToast>::new);
    AnimatedToastStackHandle {
        toasts,
        default_duration: options.default_duration,
        limit: options.limit,
    }
}

fn item_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(180),
            tween_ease_out(180),
        );
    }
    let mut from = motion_style! { opacity: 0.0, y: 22.0, scale: 0.96 };
    from.add_css_property("filter", "blur(10px)");
    let mut to = motion_style! { opacity: 1.0, y: 0.0, scale: 1.0 };
    to.add_css_property("filter", "blur(0px)");
    let mut exit = motion_style! { opacity: 0.0, x: 32.0, scale: 0.96 };
    exit.add_css_property("filter", "blur(8px)");
    PresenceConfig::with_transitions(
        from,
        to,
        exit,
        AnimationConfig::spring(SPRING_STACK),
        tween_ease_out(180),
    )
}

/// The status glyph and the text block use the same `CONTENT_TRANSITION`
/// (0.28 s ease-out) with a small blur.
fn content_config(tall: bool, reduce: bool) -> PresenceConfig {
    let scale = if tall { 0.8 } else { 1.0 };
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(280),
            tween_ease_out(280),
        );
    }
    let mut from = motion_style! { opacity: 0.0, y: 8.0, scale: scale };
    from.add_css_property("filter", "blur(6px)");
    let mut to = motion_style! { opacity: 1.0, y: 0.0, scale: 1.0 };
    to.add_css_property("filter", "blur(0px)");
    let mut exit = motion_style! { opacity: 0.0, y: -8.0, scale: 0.9 };
    exit.add_css_property("filter", "blur(6px)");
    PresenceConfig::with_transitions(from, to, exit, tween_ease_out(280), tween_ease_out(280))
}

/// `AnimatedToastStack`.
#[component]
pub fn AnimatedToastStack(
    toasts: Vec<AnimatedToast>,
    #[props(default = None)] on_dismiss: Option<EventHandler<String>>,
    #[props(default = None)] on_action: Option<EventHandler<String>>,
    #[props(default)] position: ToastPosition,
    #[props(default = None)] placement: Option<ToastPlacement>,
    #[props(default = false)] fixed: bool,
    #[props(default = None)] portal: Option<bool>,
    #[props(default = 4)] max_visible: usize,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] class_names: Option<ToastClassNames>,
) -> Element {
    let names = class_names.unwrap_or_default();
    let resolved = placement.unwrap_or(if fixed {
        ToastPlacement::Fixed
    } else {
        ToastPlacement::Static
    });
    let should_portal = portal.unwrap_or(resolved == ToastPlacement::Fixed);
    let start = toasts.len().saturating_sub(max_visible);
    let visible: Vec<AnimatedToast> = toasts[start..].to_vec();

    let list = rsx! {
        ol {
            "aria-live": "polite",
            "aria-atomic": "false",
            class: cn!(
                "pointer-events-none flex w-[calc(100vw-2rem)] max-w-sm gap-2",
                if position.is_bottom() { "flex-col-reverse" } else { "flex-col" },
                if resolved == ToastPlacement::Fixed { "fixed z-[90]" } else { "" },
                if resolved == ToastPlacement::Absolute { "absolute z-20" } else { "" },
                if resolved != ToastPlacement::Static { position.class() } else { "" },
                names.root.as_deref().unwrap_or(""),
                class.as_deref().unwrap_or(""),
            ),
            AnimatePresence {
                initial: false,
                mode: PresenceMode::PopLayout,
                for (index, toast) in visible.into_iter().enumerate() {
                    ToastItem {
                        key: "{toast.id}",
                        toast,
                        index,
                        on_dismiss,
                        on_action,
                        names: names.clone(),
                    }
                }
            }
        }
    };

    if should_portal {
        rsx! { PortalHost { {list} } }
    } else {
        list
    }
}

#[component]
fn ToastItem(
    toast: AnimatedToast,
    index: usize,
    on_dismiss: Option<EventHandler<String>>,
    on_action: Option<EventHandler<String>>,
    names: ToastClassNames,
) -> Element {
    let gate = use_gate();
    let reduce = use_reduced_motion()();
    let handle = use_presence_style(item_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);

    let can_dismiss = toast.dismissible && on_dismiss.is_some();
    let has_details = toast.description.is_some() || toast.action_label.is_some();

    // Auto-dismiss: the timer lives with the item, and re-arms when the toast's
    // duration or clock changes (the loading→success update that gives a sticky
    // toast a duration).
    let mut timer_key = use_signal(|| (toast.duration, toast.created_at));
    if *timer_key.peek() != (toast.duration, toast.created_at) {
        timer_key.set((toast.duration, toast.created_at));
    }
    let id_for_timer = toast.id.clone();
    let timer_dismiss = on_dismiss;
    use_effect(move || {
        let (duration, _) = timer_key();
        if duration <= 0 {
            return;
        }
        let Some(dismiss) = timer_dismiss else {
            return;
        };
        let id = id_for_timer.clone();
        spawn(async move {
            if Time::delay(Duration::from_millis(duration as u64))
                .await
                .is_ok()
            {
                dismiss.call(id);
            }
        });
    });

    let id_for_drag = toast.id.clone();
    let on_drag_end = EventHandler::new(move |end: DragEnd| {
        if let Some(dismiss) = on_dismiss {
            if end.offset.x.abs() > 72.0 || end.velocity.x.abs() > 520.0 {
                dismiss.call(id_for_drag.clone());
            }
        }
    });
    let drag = use_drag(
        DragOptions::default()
            .axis(DragAxis::X)
            .range(DragRange::pinned(0.0), DragRange::pinned(0.0))
            .elastic(DragElastic::uniform(0.18))
            .without_momentum(),
        can_dismiss.then_some(on_drag_end),
    );

    // See `bottom_sheet.rs`: reading `velocity` keeps this render subscribed to
    // the gesture, because `offset()`'s base is read untracked.
    let _ = drag.velocity();
    let offset_x = drag.offset().x;
    let dragging = drag.is_dragging() || drag.is_settling();

    let mut style = gate.style_with(&css);
    if dragging {
        style.push_str(&format!(";transform:translateX({offset_x}px)"));
    }

    let drag_handlers = can_dismiss;
    let z = 20i32 - index as i32;

    rsx! {
        li {
            inert: gate.inert_attr(),
            style: "{style};z-index:{z}",
            class: cn!("pointer-events-auto relative will-change-transform", names.item.as_deref().unwrap_or("")),
            "data-beui-drag": drag.scope(),
            onpointerdown: move |event| { if drag_handlers { drag.on_pointer_down(&event); } },
            onpointermove: move |event| { if drag_handlers { drag.on_pointer_move(&event); } },
            onpointerup: move |event| { if drag_handlers { drag.on_pointer_up(&event); } },
            onpointercancel: move |event| { if drag_handlers { drag.on_pointer_cancel(&event); } },
            onlostpointercapture: move |event| { if drag_handlers { drag.on_pointer_up(&event); } },
            div {
                class: cn!(
                    "relative overflow-hidden rounded-2xl border border-border bg-card/95 p-3 shadow-2xl backdrop-blur-xl",
                    names.surface.as_deref().unwrap_or(""),
                ),
                div {
                    class: cn!(
                        "flex gap-3",
                        if has_details { "items-start" } else { "items-center" },
                    ),
                    ToastGlyph { status: toast.status, has_details, reduce, names: names.clone() }
                    ToastContent { toast: toast.clone(), names: names.clone(), on_action }
                    if can_dismiss {
                        if let Some(dismiss) = on_dismiss {
                            button {
                                r#type: "button",
                                "aria-label": "Dismiss toast",
                                class: cn!(
                                    "inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-muted/80 hover:text-foreground",
                                    names.close.as_deref().unwrap_or(""),
                                ),
                                onclick: {
                                    let id = toast.id.clone();
                                    move |_| dismiss.call(id.clone())
                                },
                                XIcon { class: "h-3.5 w-3.5" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ToastGlyph(
    status: ToastStatus,
    has_details: bool,
    reduce: bool,
    names: ToastClassNames,
) -> Element {
    let variants = [status];
    // The wrapper is a presence child itself, so it stays mounted while its own
    // exit plays; the glyph inside is a second boundary keyed by status.
    rsx! {
        span {
            class: cn!(
                "inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full",
                if has_details { "mt-0.5" } else { "" },
                status.class(),
                names.icon_wrap.as_deref().unwrap_or(""),
            ),
            AnimatePresence {
                mode: PresenceMode::PopLayout,
                initial: false,
                for value in variants {
                    GlyphSwap { key: "{value.key()}", status: value, reduce }
                }
            }
        }
    }
}

#[component]
fn GlyphSwap(status: ToastStatus, reduce: bool) -> Element {
    let handle = use_presence_style(content_config(true, reduce)).expect("static presence config");
    let css = presence_css(&handle);
    rsx! {
        span {
            style: css,
            class: "inline-flex",
            if status == ToastStatus::Loading {
                span { class: "inline-flex animate-spin", {status.glyph()} }
            } else {
                {status.glyph()}
            }
        }
    }
}

#[component]
fn ToastContent(
    toast: AnimatedToast,
    names: ToastClassNames,
    on_action: Option<EventHandler<String>>,
) -> Element {
    let reduce = use_reduced_motion()();
    let key = format!("{}-{}-{}", toast.id, toast.status.key(), toast.title);
    let keys = [key];
    let action_label = toast.action_label.clone();
    let on_action_id = toast.id.clone();

    rsx! {
        div { class: cn!("min-w-0 flex-1", names.content.as_deref().unwrap_or("")),
            AnimatePresence {
                mode: PresenceMode::PopLayout,
                initial: false,
                for value in keys {
                    ContentSwap {
                        key: "{value}",
                        reduce,
                        description: toast.description.clone(),
                        title: toast.title.clone(),
                        names: names.clone(),
                    }
                }
            }
            if let Some(label) = action_label {
                if let Some(action) = on_action {
                    button {
                        r#type: "button",
                        class: cn!(
                            "mt-2 inline-flex h-7 items-center rounded-full bg-muted/80 px-3 text-xs font-medium text-foreground transition-colors hover:bg-muted",
                            names.action.as_deref().unwrap_or(""),
                        ),
                        onclick: move |_| action.call(on_action_id.clone()),
                        "{label}"
                    }
                }
            }
        }
    }
}

#[component]
fn ContentSwap(
    reduce: bool,
    title: String,
    description: Option<String>,
    names: ToastClassNames,
) -> Element {
    let handle = use_presence_style(content_config(false, reduce)).expect("static presence config");
    let css = presence_css(&handle);
    rsx! {
        div {
            style: css,
            p {
                class: cn!("truncate text-sm font-medium leading-5 text-foreground", names.title.as_deref().unwrap_or("")),
                "{title}"
            }
            if let Some(description) = description {
                p {
                    class: cn!("mt-0.5 line-clamp-2 text-xs leading-4 text-muted-foreground", names.description.as_deref().unwrap_or("")),
                    "{description}"
                }
            }
        }
    }
}
