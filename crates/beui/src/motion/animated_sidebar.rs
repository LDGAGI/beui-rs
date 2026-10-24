//! `components/motion/animated-sidebar.tsx` — a sidebar that folds down to an
//! icon rail, with a mobile sheet, shared-layout active pill and collapsing
//! submenus.
//!
//! Structure follows the source one-for-one: `AnimatedSidebarProvider` owns the
//! state (desktop `open` and mobile `openMobile`, plus the ⌘/Ctrl+B shortcut),
//! `AnimatedSidebar` branches on a `(max-width: 767px)` media query into either
//! the desktop `<aside>` or the portaled mobile sheet, and the ~20 leaf
//! components are layout classes. `useAnimatedSidebar()` is [`use_animated_sidebar`].
//!
//! How the three animated pieces map:
//!
//! - rail width: the source animates `width` between the provider's
//!   `--sidebar-width` / `--sidebar-width-icon` variables on the
//!   `SIDEBAR_MORPH_TRANSITION` spring (380 / 35 / 0.75, copied verbatim).
//!   `width` cannot take a CSS variable through `dioxus-motion`, so a single
//!   `0..=1` progress is sprung and the width is `calc()`ed from the variables
//!   with it — the variables still decide the width, so overriding them works.
//! - the active pill is a `layoutId` shared element: rendered inside the active
//!   button and moved by `beui_core::flip` on `SPRING_LAYOUT`, exactly as
//!   `tabs.rs` does its indicator.
//! - offcanvas opacity/`x` and the collapsing labels are tweens on `EASE_DRAWER`
//!   and `EASE_OUT`; those are CSS transitions, since a tween on a fixed easing
//!   is the same curve. The chevron rotation is a `SPRING_LAYOUT` spring, so it
//!   stays in `dioxus-motion`.
//!
//! Deviations from the source:
//!
//! - the collapsed `aria-label`/`title` cannot be read off `children` (an
//!   `Element`, not a string), so menu buttons take an explicit `label`;
//! - `SharedLayoutBg` is not ported in this lane, so the menu's pill is the
//!   per-button flip shape described above (same `pillClassName`);
//! - the submenu's item stagger is CSS, driven by one frame-delayed flip, so
//!   the close is a fade rather than the source's staggered retract.

use dioxus::prelude::*;
use dioxus_motion::prelude::{
    use_motion, AnimationConfig, AnimationManager, Duration, Spring, Time, TimeProvider,
};

use beui_core::cn;
use beui_core::ease::{tween_ease_out, SPRING_LAYOUT};
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::{use_dismiss, use_reduced_motion, DismissOptions};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceMode,
};

use crate::icons::{ChevronRightIcon, XIcon};
use crate::motion::PortalHost;

/// `SIDEBAR_MORPH_TRANSITION`, copied verbatim.
const SIDEBAR_MORPH: Spring = Spring {
    stiffness: 380.0,
    damping: 35.0,
    mass: 0.75,
};

/// `MOBILE_QUERY`.
const MOBILE_QUERY: &str = "(max-width: 767px)";

/// Which edge the sidebar hugs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimatedSidebarSide {
    #[default]
    Left,
    Right,
}

impl AnimatedSidebarSide {
    fn slug(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// The panel's surface treatment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimatedSidebarVariant {
    #[default]
    Sidebar,
    Floating,
    Inset,
}

impl AnimatedSidebarVariant {
    fn slug(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::Floating => "floating",
            Self::Inset => "inset",
        }
    }
}

/// How the sidebar folds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimatedSidebarCollapsible {
    Offcanvas,
    #[default]
    Icon,
    None,
}

impl AnimatedSidebarCollapsible {
    fn slug(self) -> &'static str {
        match self {
            Self::Offcanvas => "offcanvas",
            Self::Icon => "icon",
            Self::None => "none",
        }
    }

    fn animates(self) -> bool {
        self != Self::None
    }
}

/// `useAnimatedSidebar()`'s context value. Signals rather than plain values, so
/// that a consumer re-renders when the panel opens or the viewport flips.
#[derive(Clone, Copy)]
pub struct AnimatedSidebarApi {
    pub is_mobile: Signal<bool>,
    /// Desktop state.
    pub open: Signal<bool>,
    /// Mobile state.
    pub open_mobile: Signal<bool>,
    pub reduce: bool,
    /// The `layoutId` the active pill and any other shared shape uses.
    pub layout_id: Signal<String>,
    pub set_open: EventHandler<bool>,
    pub set_open_mobile: EventHandler<bool>,
    pub toggle: EventHandler<()>,
}

impl AnimatedSidebarApi {
    pub fn is_expanded(&self) -> bool {
        (self.open)()
    }
}

/// `useAnimatedSidebar()`.
///
/// # Panics
///
/// When there is no [`AnimatedSidebarProvider`] above — the source throws.
pub fn use_animated_sidebar() -> AnimatedSidebarApi {
    use_context::<AnimatedSidebarApi>()
}

/// What the panel tells its parts about itself. The open state is a signal, not
/// a bool: a context value is created once, so a bool captured here would freeze
/// at whatever the panel was on the render that provided it.
#[derive(Clone, Copy)]
struct SidebarPanel {
    open: Signal<bool>,
    collapsible: AnimatedSidebarCollapsible,
    side: AnimatedSidebarSide,
}

impl SidebarPanel {
    fn collapsed(self) -> bool {
        self.collapsible.animates() && !(self.open)()
    }
}

fn use_sidebar_panel() -> SidebarPanel {
    use_context::<SidebarPanel>()
}

/// `useIsMobile()` — a `matchMedia` subscription, not a resize listener.
fn use_is_mobile() -> Signal<bool> {
    let is_mobile = use_signal(|| false);
    use_effect(move || {
        let script = r#"
            (() => {
                const query = window.matchMedia("__BEUI_MOBILE_QUERY__");
                let last = null;
                const report = () => {
                    if (query.matches === last) return;
                    last = query.matches;
                    dioxus.send(query.matches);
                };
                report();
                query.addEventListener("change", report);
            })()
        "#
        .replace("__BEUI_MOBILE_QUERY__", MOBILE_QUERY);
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        spawn(async move {
            let mut slot = is_mobile;
            while let Ok(value) = eval.recv::<bool>().await {
                if (slot)() != value {
                    slot.set(value);
                }
            }
        });
    });
    is_mobile
}

/// Provides [`SidebarPanel`] around a panel's children, in both branches.
#[component]
fn SidebarPanelScope(
    open: Signal<bool>,
    collapsible: AnimatedSidebarCollapsible,
    side: AnimatedSidebarSide,
    children: Element,
) -> Element {
    use_context_provider(|| SidebarPanel {
        open,
        collapsible,
        side,
    });
    rsx! {
        {children}
    }
}

/// `AnimatedSidebarProvider`.
#[component]
pub fn AnimatedSidebarProvider(
    children: Element,
    #[props(default = None)] open: Option<bool>,
    #[props(default = true)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = None)] open_mobile: Option<bool>,
    #[props(default = false)] default_open_mobile: bool,
    #[props(default = None)] on_open_mobile_change: Option<EventHandler<bool>>,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] style: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let is_mobile = use_is_mobile();
    let controlled_open = open.is_some();
    let controlled_mobile = open_mobile.is_some();

    let open_signal = use_signal(|| open.unwrap_or(default_open));
    let mobile_signal = use_signal(|| open_mobile.unwrap_or(default_open_mobile));
    if let Some(value) = open {
        if *open_signal.peek() != value {
            let mut slot = open_signal;
            slot.set(value);
        }
    }
    if let Some(value) = open_mobile {
        if *mobile_signal.peek() != value {
            let mut slot = mobile_signal;
            slot.set(value);
        }
    }

    let set_open = EventHandler::new(move |next: bool| {
        if !controlled_open {
            let mut slot = open_signal;
            slot.set(next);
        }
        if let Some(callback) = on_open_change {
            callback.call(next);
        }
    });
    let set_open_mobile = EventHandler::new(move |next: bool| {
        if !controlled_mobile {
            let mut slot = mobile_signal;
            slot.set(next);
        }
        if let Some(callback) = on_open_mobile_change {
            callback.call(next);
        }
    });
    let toggle = EventHandler::new(move |_: ()| {
        if (is_mobile)() {
            let next = !(mobile_signal)();
            set_open_mobile.call(next);
        } else {
            let next = !(open_signal)();
            set_open.call(next);
        }
    });

    // ⌘/Ctrl+B toggles whichever state the current viewport uses.
    let toggle_for_shortcut = toggle;
    use_effect(move || {
        let script = r#"
            (() => {
                const onKeyDown = (event) => {
                    if (event.key.toLowerCase() === "b" && (event.metaKey || event.ctrlKey)) {
                        event.preventDefault();
                        dioxus.send("toggle");
                    }
                };
                window.addEventListener("keydown", onKeyDown);
            })()
        "#;
        let Some(mut eval) = platform::eval(script) else {
            return;
        };
        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                if message == "toggle" {
                    toggle_for_shortcut.call(());
                }
            }
        });
    });

    let layout_id = use_signal(|| format!("beui-sidebar-{}", platform::next_scope_id()));
    let api = AnimatedSidebarApi {
        is_mobile,
        open: open_signal,
        open_mobile: mobile_signal,
        reduce,
        layout_id,
        set_open,
        set_open_mobile,
        toggle,
    };
    use_context_provider(|| api);

    let state = if api.is_expanded() {
        "expanded"
    } else {
        "collapsed"
    };
    let style = format!(
        "--sidebar-width:16rem;--sidebar-width-icon:4.25rem;--sidebar-width-mobile:18rem;{}",
        style.as_deref().unwrap_or(""),
    );

    rsx! {
        div {
            "data-slot": "sidebar-wrapper",
            "data-state": state,
            style,
            class: cn!(
                "group/sidebar-wrapper flex min-h-svh w-full min-w-0",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AnimatedSidebar`.
#[component]
pub fn AnimatedSidebar(
    children: Element,
    #[props(default = AnimatedSidebarSide::Left)] side: AnimatedSidebarSide,
    #[props(default = AnimatedSidebarVariant::Sidebar)] variant: AnimatedSidebarVariant,
    #[props(default = AnimatedSidebarCollapsible::Icon)] collapsible: AnimatedSidebarCollapsible,
    #[props(default = "Sidebar".to_string())] aria_label: String,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] panel_class: Option<String>,
) -> Element {
    let api = use_animated_sidebar();
    let open = api.open;
    let collapsed = collapsible.animates() && !(open)();
    let offcanvas = collapsed && collapsible == AnimatedSidebarCollapsible::Offcanvas;

    // The rail's width progress: 1 = expanded, 0 = collapsed.
    let progress = use_motion(if (open)() { 1.0f32 } else { 0.0f32 })
        .expect("1.0 and 0.0 are finite animation values");
    let progress_signal = progress;
    use_effect(move || {
        let is_open = (open)();
        let config = if api.reduce {
            tween_ease_out(1)
        } else {
            AnimationConfig::spring(SIDEBAR_MORPH)
        };
        let mut handle = progress_signal;
        let _ = handle.animate_to(if is_open { 1.0 } else { 0.0 }, config);
    });

    if (api.is_mobile)() {
        return rsx! {
            MobileSidebar { aria_label, class, side, children }
        };
    }

    let t = progress.get_value();
    let panel_duration = if api.reduce { "160ms" } else { "360ms" };
    let width = match collapsible {
        AnimatedSidebarCollapsible::Offcanvas => {
            "width:calc(var(--sidebar-width) * var(--beui-sidebar-t))".to_string()
        }
        AnimatedSidebarCollapsible::Icon => {
            ("width:calc(var(--sidebar-width-icon) + (var(--sidebar-width) - \
             var(--sidebar-width-icon)) * var(--beui-sidebar-t))")
                .to_string()
        }
        AnimatedSidebarCollapsible::None => "width:var(--sidebar-width)".to_string(),
    };

    rsx! {
        aside {
            "data-slot": "sidebar",
            "data-state": if collapsed { "collapsed" } else { "expanded" },
            "data-collapsible": collapsible.slug(),
            "data-variant": variant.slug(),
            "data-side": side.slug(),
            "aria-label": aria_label,
            style: format!("--beui-sidebar-t:{t};{width}"),
            class: cn!(
                "group/sidebar relative hidden h-auto shrink-0 md:block will-change-[width] peer",
                if side == AnimatedSidebarSide::Right { "order-last" } else { "" },
                class.as_deref().unwrap_or(""),
            ),
            div {
                "data-slot": "sidebar-panel",
                style: "transition-property:opacity,transform;transition-duration:{panel_duration};transition-timing-function:cubic-bezier(0.32,0.72,0,1)",
                class: cn!(
                    "sticky top-0 flex h-svh w-full flex-col overflow-hidden bg-background",
                    if collapsible == AnimatedSidebarCollapsible::Offcanvas {
                        "w-[var(--sidebar-width)]"
                    } else {
                        ""
                    },
                    match variant {
                        AnimatedSidebarVariant::Sidebar => match side {
                            AnimatedSidebarSide::Left => "border-border border-r",
                            AnimatedSidebarSide::Right => "border-border border-l",
                        },
                        AnimatedSidebarVariant::Floating => {
                            "m-2 h-[calc(100svh-1rem)] rounded-2xl border border-border shadow-sm"
                        }
                        AnimatedSidebarVariant::Inset => "m-2 h-[calc(100svh-1rem)] rounded-2xl",
                    },
                    if offcanvas {
                        match side {
                            AnimatedSidebarSide::Left => "opacity-0 -translate-x-full",
                            AnimatedSidebarSide::Right => "opacity-0 translate-x-full",
                        }
                    } else {
                        "opacity-100 translate-x-0"
                    },
                    panel_class.as_deref().unwrap_or(""),
                ),
                SidebarPanelScope { open, collapsible, side, {children} }
            }
        }
    }
}

/// The mobile sheet. Mounted for as long as the viewport is mobile and hidden
/// while closed, so the slide starts in the commit that shows it.
#[component]
fn MobileSidebar(
    aria_label: String,
    class: Option<String>,
    side: AnimatedSidebarSide,
    children: Element,
) -> Element {
    let api = use_animated_sidebar();
    let open_mobile = api.open_mobile;
    let reduce = api.reduce;
    let dismiss = EventHandler::new(move |_: ()| api.set_open_mobile.call(false));
    let options = DismissOptions {
        escape: true,
        ..DismissOptions::default()
    };
    let scope = use_dismiss(open_mobile, dismiss, options);
    // Focus the panel the first time it opens, once it is no longer hidden.
    let scope_selector = scope.selector();
    let mut focused = use_signal(|| false);
    use_effect(move || {
        if !(open_mobile)() {
            focused.set(false);
            return;
        }
        if (focused)() {
            return;
        }
        focused.set(true);
        let selector = scope_selector.clone();
        platform::fire(&format!(
            "requestAnimationFrame(() => {{ const root = document.querySelector({}); if (root) {{ const first = root.querySelector(\"a[href],button:not([disabled]),input:not([disabled]),[tabindex]:not([tabindex='-1'])\"); (first || root).focus({{ preventScroll: true }}); }} }})",
            platform::js_string(&selector),
        ));
    });

    let open = (open_mobile)();
    let duration = if reduce { "160ms" } else { "360ms" };
    let scrim_style = format!(
        "opacity:{};transition-property:opacity;transition-duration:{duration};transition-timing-function:cubic-bezier(0.32,0.72,0,1)",
        if open { 1 } else { 0 },
    );
    let panel_style = format!(
        "opacity:{};transform:translateX({});transition-property:opacity,transform;transition-duration:{duration};transition-timing-function:cubic-bezier(0.32,0.72,0,1)",
        if reduce { if open { 1 } else { 0 } } else { 1 },
        if reduce || open {
            "0%"
        } else if side == AnimatedSidebarSide::Left {
            "-100%"
        } else {
            "100%"
        },
    );

    rsx! {
        PortalHost {
            div {
                class: cn!(
                    "pointer-events-none fixed left-0 top-0 z-50 size-0 md:hidden",
                    if open { "visible" } else { "invisible" },
                ),
                button {
                    r#type: "button",
                    "aria-label": "Close sidebar",
                    tabindex: if open { 0 } else { -1 },
                    style: scrim_style,
                    onclick: move |_| api.set_open_mobile.call(false),
                    class: cn!(
                        "fixed inset-0 bg-black/40",
                        if open { "pointer-events-auto" } else { "pointer-events-none" },
                    ),
                }
                div {
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-label": aria_label,
                    "aria-hidden": (!open).to_string(),
                    "inert": if open { None } else { Some("true") },
                    "data-beui-scope": scope.attr(),
                    tabindex: -1,
                    "data-mobile": "true",
                    "data-state": if open { "expanded" } else { "collapsed" },
                    "data-side": side.slug(),
                    style: panel_style,
                    class: cn!(
                        "pointer-events-auto fixed inset-y-0 flex h-dvh w-(--sidebar-width-mobile) max-w-[88vw] flex-col overflow-hidden",
                        "border-border bg-background shadow-2xl will-change-transform",
                        match side {
                            AnimatedSidebarSide::Left => "left-0 border-r",
                            AnimatedSidebarSide::Right => "right-0 border-l",
                        },
                        if open { "" } else { "pointer-events-none" },
                        class.as_deref().unwrap_or(""),
                    ),
                    SidebarPanelScope {
                        open: api.open,
                        collapsible: AnimatedSidebarCollapsible::None,
                        side,
                        {children}
                    }
                }
            }
        }
    }
}

/// `AnimatedSidebarTrigger`.
#[component]
pub fn AnimatedSidebarTrigger(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let api = use_animated_sidebar();
    let expanded = if (api.is_mobile)() {
        (api.open_mobile)()
    } else {
        (api.open)()
    };

    rsx! {
        button {
            r#type: "button",
            "aria-label": "Toggle sidebar",
            "aria-expanded": expanded,
            "data-slot": "sidebar-trigger",
            "data-state": if expanded { "expanded" } else { "collapsed" },
            onclick: move |_| api.toggle.call(()),
            class: cn!(
                "inline-flex size-10 shrink-0 items-center justify-center rounded-xl outline-none",
                "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AnimatedSidebarClose`.
#[component]
pub fn AnimatedSidebarClose(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let api = use_animated_sidebar();

    rsx! {
        button {
            r#type: "button",
            "aria-label": "Close sidebar",
            onclick: move |_| {
                if (api.is_mobile)() {
                    api.set_open_mobile.call(false);
                } else {
                    api.set_open.call(false);
                }
            },
            class: cn!(
                "inline-flex size-10 shrink-0 items-center justify-center rounded-xl outline-none",
                "focus-visible:ring-2 focus-visible:ring-ring",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AnimatedSidebarRail` — the 4px drag strip on the panel's inner edge.
#[component]
pub fn AnimatedSidebarRail(#[props(default = None)] class: Option<String>) -> Element {
    let api = use_animated_sidebar();
    let panel = use_sidebar_panel();

    rsx! {
        button {
            r#type: "button",
            "data-side": panel.side.slug(),
            "aria-label": "Toggle sidebar",
            title: "Toggle sidebar",
            tabindex: -1,
            onclick: move |_| api.toggle.call(()),
            class: cn!(
                "absolute inset-y-0 z-20 hidden w-4 -translate-x-1/2 outline-none md:block",
                "after:absolute after:inset-y-0 after:left-1/2 after:w-px after:bg-transparent after:transition-colors hover:after:bg-border",
                "data-[side=right]:right-0 data-[side=right]:translate-x-1/2 data-[side=left]:left-full",
                class.as_deref().unwrap_or(""),
            ),
        }
    }
}

/// `AnimatedSidebarInset`.
#[component]
pub fn AnimatedSidebarInset(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    rsx! {
        main {
            "data-slot": "sidebar-inset",
            class: cn!(
                "relative flex min-h-svh min-w-0 flex-1 flex-col bg-background",
                "md:peer-data-[variant=inset]:m-2 md:peer-data-[variant=inset]:ml-0 md:peer-data-[variant=inset]:rounded-2xl md:peer-data-[variant=inset]:shadow-sm",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AnimatedSidebarHeader`.
#[component]
pub fn AnimatedSidebarHeader(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    rsx! {
        div {
            "data-slot": "sidebar-header",
            class: cn!("flex shrink-0 flex-col gap-2 p-3", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `AnimatedSidebarContent`.
#[component]
pub fn AnimatedSidebarContent(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    rsx! {
        div {
            "data-slot": "sidebar-content",
            class: cn!(
                "flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto overflow-x-hidden overscroll-contain px-2 py-2",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AnimatedSidebarFooter`.
#[component]
pub fn AnimatedSidebarFooter(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    rsx! {
        div {
            "data-slot": "sidebar-footer",
            class: cn!(
                "flex shrink-0 flex-col gap-2 border-border border-t p-3 pb-[max(0.75rem,env(safe-area-inset-bottom))]",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AnimatedSidebarGroup`.
#[component]
pub fn AnimatedSidebarGroup(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    rsx! {
        div {
            "data-slot": "sidebar-group",
            class: cn!("flex w-full min-w-0 flex-col px-1 py-1.5", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `AnimatedSidebarGroupLabel`.
#[component]
pub fn AnimatedSidebarGroupLabel(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let panel = use_sidebar_panel();
    let collapsed = panel.collapsed();

    rsx! {
        div {
            "aria-hidden": collapsed,
            "data-slot": "sidebar-group-label",
            class: cn!(
                "mb-1 h-7 overflow-hidden px-2 text-[10px] font-medium uppercase tracking-[0.14em] text-muted-foreground transition-opacity",
                if collapsed { "opacity-0" } else { "opacity-100" },
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AnimatedSidebarGroupContent`.
#[component]
pub fn AnimatedSidebarGroupContent(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    rsx! {
        div {
            "data-slot": "sidebar-group-content",
            class: cn!("w-full min-w-0", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `AnimatedSidebarMenu` — the list the active pill glides within.
#[component]
pub fn AnimatedSidebarMenu(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    rsx! {
        ul {
            "data-slot": "sidebar-menu",
            class: cn!(
                "relative flex w-full min-w-0 list-none flex-col gap-0.5",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AnimatedSidebarMenuItem`.
#[component]
pub fn AnimatedSidebarMenuItem(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    rsx! {
        li {
            "data-slot": "sidebar-menu-item",
            class: cn!("relative", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// The shared-layout active pill. Mounted with the active button only, so
/// `layout_id` is what makes it read the previous box out of the registry —
/// the same shape `tabs.rs` uses for its indicator.
#[component]
fn SidebarMenuPill(layout_id: String, state: String, reduce: bool) -> Element {
    let flip = use_flip(
        state,
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(if reduce {
                tween_ease_out(1)
            } else {
                AnimationConfig::spring(SPRING_LAYOUT)
            })
            .border_radius(12.0),
    );

    rsx! {
        span {
            "aria-hidden": "true",
            "data-beui-flip": flip.scope(),
            style: flip.style_with("border-radius:12px"),
            class: "absolute inset-0 bg-muted",
        }
    }
}

/// `AnimatedSidebarMenuButton`.
#[component]
#[allow(clippy::too_many_arguments)]
pub fn AnimatedSidebarMenuButton(
    children: Element,
    #[props(default = None)] icon: Option<Element>,
    #[props(default = None)] badge: Option<Element>,
    #[props(default = None)] href: Option<String>,
    #[props(default = false)] is_active: bool,
    #[props(default = None)] aria_expanded: Option<bool>,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] close_on_select: Option<bool>,
    /// The button's text, for the collapsed `aria-label` / `title`. The source
    /// reads it off a string `children`; an `Element` cannot be read.
    #[props(default = None)]
    label: Option<String>,
    #[props(default = None)] on_select: Option<EventHandler<()>>,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let api = use_animated_sidebar();
    let panel = use_sidebar_panel();
    let collapsed = panel.collapsed();
    let reduce = api.reduce;

    // The chevron's rotation is a SPRING_LAYOUT spring, so it stays in Rust.
    let rotation = use_motion(if aria_expanded == Some(true) {
        90.0f32
    } else {
        0.0f32
    })
    .expect("0.0 and 90.0 are finite animation values");
    let rotation_signal = rotation;
    use_effect(move || {
        let target = if aria_expanded == Some(true) {
            90.0
        } else {
            0.0
        };
        let config = if reduce {
            tween_ease_out(1)
        } else {
            AnimationConfig::spring(SPRING_LAYOUT)
        };
        let mut handle = rotation_signal;
        let _ = handle.animate_to(target, config);
    });

    let select = move |_| {
        if disabled {
            return;
        }
        if let Some(callback) = on_select {
            callback.call(());
        }
        let should_close = close_on_select.unwrap_or(aria_expanded.is_none());
        if (api.is_mobile)() && should_close {
            api.set_open_mobile.call(false);
        }
        // A submenu cannot render in the icon rail, so opening one from there
        // would leave its children unreachable.
        if aria_expanded.is_some() && collapsed && !(api.is_mobile)() {
            api.set_open.call(true);
        }
    };

    let collapse_style = if reduce {
        "transition-property:opacity,transform;transition-duration:160ms;transition-timing-function:cubic-bezier(0.16,1,0.3,1)"
    } else if collapsed {
        "transition-property:opacity,transform;transition-duration:120ms;transition-timing-function:cubic-bezier(0.16,1,0.3,1)"
    } else {
        "transition-property:opacity,transform;transition-duration:200ms;transition-delay:80ms;transition-timing-function:cubic-bezier(0.16,1,0.3,1)"
    };

    let interactive_class = cn!(
        "relative flex min-h-9 w-full min-w-0 items-center gap-2.5 overflow-hidden rounded-xl px-3 text-left text-sm font-medium outline-none",
        "text-muted-foreground transition-colors hover:text-foreground",
        "focus-visible:bg-muted/70 focus-visible:ring-2 focus-visible:ring-ring",
        if is_active { "text-foreground" } else { "" },
        if disabled { "cursor-not-allowed opacity-40" } else { "" },
        if reduce || disabled { "" } else { "active:scale-[0.98]" },
        class.as_deref().unwrap_or(""),
    );
    let collapsed_label = if collapsed { label.clone() } else { None };
    let pill_state = label.clone().unwrap_or_default();
    let rotate = rotation.get_value();

    rsx! {
        if let Some(href) = href {
            a {
                href,
                "aria-current": if is_active { Some("page") } else { None },
                "aria-expanded": aria_expanded.map(|value| value.to_string()),
                "aria-disabled": if disabled { Some("true") } else { None },
                "aria-label": collapsed_label.clone(),
                title: collapsed_label.clone(),
                tabindex: if disabled { -1 } else { 0 },
                onclick: select,
                class: interactive_class.clone(),
                SidebarMenuContent {
                    icon,
                    badge,
                    collapsed,
                    collapse_style: collapse_style.to_string(),
                    is_active,
                    layout_id: (api.layout_id)(),
                    pill_state: pill_state.clone(),
                    reduce,
                    aria_expanded,
                    rotate,
                    children,
                }
            }
        } else {
            button {
                r#type: "button",
                disabled,
                "aria-current": if is_active { Some("page") } else { None },
                "aria-expanded": aria_expanded.map(|value| value.to_string()),
                "aria-label": collapsed_label.clone(),
                title: collapsed_label.clone(),
                onclick: select,
                class: interactive_class,
                SidebarMenuContent {
                    icon,
                    badge,
                    collapsed,
                    collapse_style: collapse_style.to_string(),
                    is_active,
                    layout_id: (api.layout_id)(),
                    pill_state,
                    reduce,
                    aria_expanded,
                    rotate,
                    children,
                }
            }
        }
    }
}

/// The button's innards: pill, icon, label, badge, chevron.
#[component]
#[allow(clippy::too_many_arguments)]
fn SidebarMenuContent(
    children: Element,
    #[props(default = None)] icon: Option<Element>,
    #[props(default = None)] badge: Option<Element>,
    collapsed: bool,
    collapse_style: String,
    is_active: bool,
    layout_id: String,
    pill_state: String,
    reduce: bool,
    #[props(default = None)] aria_expanded: Option<bool>,
    rotate: f32,
) -> Element {
    rsx! {
        if is_active {
            SidebarMenuPill { layout_id, state: pill_state, reduce }
        }
        if let Some(icon) = icon {
            span {
                "aria-hidden": "true",
                class: "relative z-10 grid size-5 shrink-0 place-items-center",
                {icon}
            }
        }
        span {
            "aria-hidden": collapsed,
            style: collapse_style,
            class: cn!(
                "relative z-10 min-w-0 flex-1 truncate",
                if collapsed { "pointer-events-none opacity-0 -translate-x-1" } else { "opacity-100 translate-x-0" },
            ),
            {children}
        }
        if !collapsed {
            if let Some(badge) = badge {
                span {
                    class: "relative z-10 shrink-0 text-xs text-muted-foreground",
                    {badge}
                }
            }
        }
        if aria_expanded.is_some() {
            span {
                "aria-hidden": "true",
                style: format!("transform:rotate({rotate}deg)"),
                class: cn!(
                    "relative z-10 grid size-4 shrink-0 place-items-center text-muted-foreground transition-opacity",
                    if collapsed { "opacity-0" } else { "opacity-100" },
                ),
                ChevronRightIcon { class: "size-3.5" }
            }
        }
    }
}

/// The open/closed state the submenu's items stagger from.
#[derive(Clone, Copy)]
struct SidebarSubmenu {
    open: Signal<bool>,
}

/// `AnimatedSidebarMenuSub`.
#[component]
pub fn AnimatedSidebarMenuSub(
    open: bool,
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let api = use_animated_sidebar();
    let panel = use_sidebar_panel();
    let reduce = api.reduce;
    let shown = open && !panel.collapsed();
    let open_signal = use_signal(|| shown);
    if shown != (open_signal)() {
        let mut slot = open_signal;
        slot.set(shown);
    }
    use_context_provider(|| SidebarSubmenu { open: open_signal });

    let layers: Vec<&'static str> = if shown {
        vec!["sidebar-submenu"]
    } else {
        Vec::new()
    };
    let class = class.clone();

    rsx! {
        AnimatePresence {
            initial: false,
            mode: PresenceMode::PopLayout,
            for key in layers {
                SidebarSubmenuLayer {
                    key: "{key}",
                    reduce,
                    class: class.clone(),
                    {children.clone()}
                }
            }
        }
    }
}

/// One keyed presence layer, as the hard constraint requires.
#[component]
fn SidebarSubmenuLayer(reduce: bool, class: Option<String>, children: Element) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(submenu_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);
    let submenu = use_context::<SidebarSubmenu>();
    let open_signal = submenu.open;

    // One frame in the open state, so the clip-path tween has somewhere to
    // travel from.
    let entered = use_signal(|| false);
    use_effect(move || {
        if (open_signal)() && !(entered)() {
            let mut slot = entered;
            spawn(async move {
                if Time::delay(Duration::from_millis(16)).await.is_ok() {
                    slot.set(true);
                }
            });
        }
    });
    let shown = (entered)();
    let clip_duration = if reduce { 120 } else { 200 };

    rsx! {
        ul {
            inert: gate.inert_attr(),
            "data-slot": "sidebar-menu-sub",
            style: format!(
                "transition-property:clip-path;transition-duration:{clip_duration}ms;transition-timing-function:cubic-bezier(0.16,1,0.3,1);{}",
                gate.style_with(&css),
            ),
            class: cn!(
                "relative mt-1 ml-5 flex min-w-0 flex-col gap-0.5 border-border border-l pl-3",
                if shown {
                    "[clip-path:inset(0_0_0%_0_round_8px)]"
                } else {
                    "[clip-path:inset(0_0_100%_0_round_8px)]"
                },
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

fn submenu_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(120),
            tween_ease_out(120),
        );
    }
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0 },
        motion_style! { opacity: 1.0 },
        motion_style! { opacity: 0.0 },
        tween_ease_out(235),
        tween_ease_out(140),
    )
}

/// `AnimatedSidebarMenuSubItem` — the staggered entry of one child row
/// (`SUBMENU_ITEM_VARIANTS`: `y: -6` → `0`, `blur(3px)` → `blur(0)`).
#[component]
pub fn AnimatedSidebarMenuSubItem(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let submenu = use_context::<SidebarSubmenu>();
    let open_signal = submenu.open;
    let entered = use_signal(|| false);
    use_effect(move || {
        if (open_signal)() && !(entered)() {
            let mut slot = entered;
            spawn(async move {
                if Time::delay(Duration::from_millis(16)).await.is_ok() {
                    slot.set(true);
                }
            });
        }
    });
    let shown = (entered)();

    rsx! {
        li {
            "data-slot": "sidebar-menu-sub-item",
            class: cn!(
                "relative min-w-0 transition-[opacity,transform,filter] ease-[cubic-bezier(0.16,1,0.3,1)]",
                "[&:nth-child(2)]:delay-[45ms] [&:nth-child(3)]:delay-[90ms] [&:nth-child(4)]:delay-[135ms]",
                "[&:nth-child(5)]:delay-[180ms] [&:nth-child(6)]:delay-[225ms]",
                if shown {
                    "opacity-100 translate-y-0 blur-[0px] duration-[180ms]"
                } else {
                    "opacity-0 -translate-y-1.5 blur-[3px] duration-[140ms]"
                },
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AnimatedSidebarMenuSubButton`.
#[component]
#[allow(clippy::too_many_arguments)]
pub fn AnimatedSidebarMenuSubButton(
    children: Element,
    #[props(default = None)] icon: Option<Element>,
    #[props(default = None)] href: Option<String>,
    #[props(default = false)] is_active: bool,
    #[props(default = false)] disabled: bool,
    #[props(default = true)] close_on_select: bool,
    #[props(default = None)] on_select: Option<EventHandler<()>>,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let api = use_animated_sidebar();
    let reduce = api.reduce;

    let select = move |_| {
        if disabled {
            return;
        }
        if let Some(callback) = on_select {
            callback.call(());
        }
        if (api.is_mobile)() && close_on_select {
            api.set_open_mobile.call(false);
        }
    };

    let interactive_class = cn!(
        "flex min-h-8 w-full min-w-0 items-center gap-2 rounded-lg px-2 text-left text-xs outline-none",
        "text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground",
        "focus-visible:bg-muted/70 focus-visible:ring-2 focus-visible:ring-ring",
        if is_active { "bg-muted/70 text-foreground" } else { "" },
        if disabled { "cursor-not-allowed opacity-40" } else { "" },
        if reduce || disabled { "" } else { "active:scale-[0.98]" },
        class.as_deref().unwrap_or(""),
    );

    let content = rsx! {
        span {
            "aria-hidden": "true",
            class: "grid size-4 shrink-0 place-items-center",
            if let Some(icon) = icon {
                {icon}
            } else {
                span { class: "size-1 rounded-full bg-current" }
            }
        }
        span { class: "min-w-0 flex-1 truncate", {children} }
    };

    rsx! {
        if let Some(href) = href {
            a {
                href,
                "aria-current": if is_active { Some("page") } else { None },
                "aria-disabled": if disabled { Some("true") } else { None },
                tabindex: if disabled { -1 } else { 0 },
                onclick: select,
                class: interactive_class.clone(),
                {content}
            }
        } else {
            button {
                r#type: "button",
                disabled,
                "aria-current": if is_active { Some("page") } else { None },
                onclick: select,
                class: interactive_class,
                {content}
            }
        }
    }
}

/// The header's close affordance for the mobile sheet.
#[component]
pub fn AnimatedSidebarMobileClose(
    children: Element,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let api = use_animated_sidebar();
    rsx! {
        button {
            r#type: "button",
            "aria-label": "Close sidebar",
            onclick: move |_| api.set_open_mobile.call(false),
            class: cn!(
                "inline-flex size-10 shrink-0 items-center justify-center rounded-xl outline-none",
                "focus-visible:ring-2 focus-visible:ring-ring md:hidden",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// The close glyph beUI's preview puts in the header.
#[component]
pub fn AnimatedSidebarCloseGlyph(#[props(default = None)] class: Option<String>) -> Element {
    rsx! {
        XIcon { class: cn!("size-4", class.as_deref().unwrap_or("")) }
    }
}
