//! `agents/chat-app.tsx` — the sidebar shell around an agent conversation.
//!
//! The source composes motion's `AnimatedSidebarProvider` with a `ShellFit`
//! probe that folds the sidebar away when the shell's own box is too narrow. The
//! motion sidebar is not in this lane, so [`crate::standins`]-style here: this
//! module carries the provider contract the agents set needs — `open` /
//! `openMobile` with controlled+uncontrolled support, `--sidebar-width` on the
//! wrapper, the ⌘/Ctrl-B shortcut, and the `ShellFit` resize probe — and leaves
//! the sidebar chrome itself to the motion lane.
//!
//! Differing from the source: `isMobile` is a media query (767px) rather than
//! `matchMedia` inside the provider, and the sidebar's own layout/animation is
//! not rendered here.

use std::cell::Cell;
use std::rc::Rc;

use beui::prelude::*;

/// Shell width below which a docked sidebar leaves too little room.
const MIN_DOCKED_WIDTH: f64 = 600.0;

/// `AnimatedSidebar` context, as much as the agents set reads.
#[derive(Clone, Copy)]
pub struct AnimatedSidebar {
    open: Signal<bool>,
    open_mobile: Signal<bool>,
    is_mobile: Signal<bool>,
    reduce: Signal<bool>,
    set_open: Callback<bool>,
    set_open_mobile: Callback<bool>,
}

impl AnimatedSidebar {
    pub fn open(&self) -> bool {
        (self.open)()
    }

    pub fn open_mobile(&self) -> bool {
        (self.open_mobile)()
    }

    pub fn is_mobile(&self) -> bool {
        (self.is_mobile)()
    }

    pub fn reduce(&self) -> bool {
        (self.reduce)()
    }

    pub fn set_open(&self, next: bool) {
        self.set_open.call(next);
    }

    pub fn set_open_mobile(&self, next: bool) {
        self.set_open_mobile.call(next);
    }

    /// `toggleSidebar`: mobile toggles the sheet, desktop the docked rail.
    pub fn toggle(&self) {
        if self.is_mobile() {
            let next = !self.open_mobile();
            self.set_open_mobile(next);
        } else {
            let next = !self.open();
            self.set_open(next);
        }
    }
}

/// `useAnimatedSidebar()`.
pub fn use_animated_sidebar() -> AnimatedSidebar {
    use_context::<AnimatedSidebar>()
}

/// `ShellFit` — folds the sidebar when the shell is too narrow to carry both
/// panes, and restores it when it is not.
///
/// Only crossings are acted on. Too narrow at first measurement *is* the fold
/// condition, so that applies immediately; a shell that merely has room says
/// nothing about the caller's intent, so the first wide measurement is ignored.
#[component]
fn ShellFit(min_width: f64) -> Element {
    let sidebar = use_animated_sidebar();
    let marker = crate::common::use_unique_id("shell-fit");
    let marker_for_selector = marker.clone();
    let narrow = use_hook(|| Rc::new(Cell::new(None::<bool>)));

    use_effect(move || {
        let selector = format!("[data-beui-shell-fit=\"{marker_for_selector}\"]");
        let script = format!(
            "(function(){{var el=document.querySelector({sel});if(!el)return;var shell=el.parentElement;if(!shell)return;\
             dioxus.send(shell.clientWidth||shell.getBoundingClientRect().width);\
             if(el.__beuiFitRO&&el.__beuiFitRO.disconnect)el.__beuiFitRO.disconnect();\
             var ro=new ResizeObserver(function(){{dioxus.send(shell.clientWidth||shell.getBoundingClientRect().width);}});\
             ro.observe(shell);el.__beuiFitRO=ro;}})();",
            sel = beui_core::platform::js_string(&selector),
        );
        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        let narrow = narrow.clone();
        spawn(async move {
            while let Ok(width) = eval.recv::<f64>().await {
                let is_narrow = width < min_width;
                let first = narrow.get().is_none();
                narrow.set(Some(is_narrow));
                if first && !is_narrow {
                    continue;
                }
                let wanted = !is_narrow;
                if sidebar.open() == wanted {
                    continue;
                }
                sidebar.set_open(wanted);
            }
        });
    });

    rsx! {
        div { "data-beui-shell-fit": "{marker}", class: "hidden" }
    }
}

/// `ChatApp`.
#[component]
pub fn ChatApp(
    #[props(default)] open: Option<bool>,
    #[props(default = true)] default_open: bool,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(default)] open_mobile: Option<bool>,
    #[props(default)] default_open_mobile: bool,
    #[props(default)] on_open_mobile_change: Option<EventHandler<bool>>,
    #[props(default = "17rem".to_string())] sidebar_width: String,
    #[props(default = MIN_DOCKED_WIDTH)] collapse_sidebar_below: f64,
    #[props(default)] class: String,
    #[props(default)] style: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let mut open_signal = use_signal(|| open.unwrap_or(default_open));
    let mut mobile_signal = use_signal(|| open_mobile.unwrap_or(default_open_mobile));
    let open_controlled = open.is_some();
    let mobile_controlled = open_mobile.is_some();

    use_effect(use_reactive((&open,), move |(open,)| {
        if let Some(open) = open {
            open_signal.set(open);
        }
    }));
    use_effect(use_reactive((&open_mobile,), move |(open_mobile,)| {
        if let Some(open_mobile) = open_mobile {
            mobile_signal.set(open_mobile);
        }
    }));

    let set_open = use_callback(move |next: bool| {
        if !open_controlled {
            open_signal.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    });
    let set_open_mobile = use_callback(move |next: bool| {
        if !mobile_controlled {
            mobile_signal.set(next);
        }
        if let Some(handler) = on_open_mobile_change {
            handler.call(next);
        }
    });

    let is_mobile = use_media_query("(max-width: 767px)");
    let reduce = use_reduced_motion();
    let sidebar = AnimatedSidebar {
        open: open_signal,
        open_mobile: mobile_signal,
        is_mobile,
        reduce,
        set_open,
        set_open_mobile,
    };
    use_context_provider(|| sidebar);

    // ⌘/Ctrl-B toggles, the source's window shortcut.
    use_effect(move || {
        let script = "(function(){window.addEventListener('keydown',function(e){if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='b'){e.preventDefault();dioxus.send('toggle');}});})();";
        let Some(mut eval) = beui_core::platform::eval(script) else {
            return;
        };
        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                if message == "toggle" {
                    sidebar.toggle();
                }
            }
        });
    });

    let wrapper_style = format!(
        "--sidebar-width:{sidebar_width};--sidebar-width-icon:4.25rem;--sidebar-width-mobile:18rem;{style}"
    );
    let state = if open_signal() {
        "expanded"
    } else {
        "collapsed"
    };
    let show_fit = !open_controlled;

    rsx! {
        div {
            "data-slot": "sidebar-wrapper",
            "data-state": "{state}",
            class: cn!(
                "group/sidebar-wrapper flex min-h-svh w-full min-w-0",
                "min-h-0 overflow-hidden rounded-2xl border border-border bg-background",
                class,
            ),
            style: "{wrapper_style}",
            ..attributes,
            if show_fit {
                ShellFit { min_width: collapse_sidebar_below }
            }
            {children}
        }
    }
}
