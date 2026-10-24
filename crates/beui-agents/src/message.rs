//! `agents/message.tsx` — the message row and its layout primitives.
//!
//! `Message` provides the alignment context the bubble layer reads
//! (`message-context`), renders the `group/message` flex row and plays the
//! trailing-edge pop-up on mount when `animate_in`. The pop-up is the source's
//! spring (`stiffness 480 / damping 32 / mass 0.62`) expressed as a CSS
//! transition — see the module notes.

use beui::prelude::*;

use crate::message_context::{MessageFrom, MessageSide};

/// `Message`.
#[component]
pub fn Message(
    from: MessageFrom,
    #[props(default)] animate_in: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] class: String,
    #[props(default)] style: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let side = if from == MessageFrom::User {
        MessageSide::End
    } else {
        MessageSide::Start
    };
    use_context_provider(|| side);
    use_context_provider(|| from);

    let mut mounted = use_signal(|| false);
    use_effect(move || mounted.set(true));

    let animate = animate_in && !reduce;
    let settled = mounted() || !animate;
    let opacity = if settled { 1 } else { 0 };
    let transform = if settled {
        "translateY(0px) scale(1)"
    } else {
        "translateY(8px) scale(0.95)"
    };
    let origin = if from == MessageFrom::User {
        "100% 100%"
    } else {
        "0% 100%"
    };
    let motion_style = if animate {
        format!(
            "transform-origin:{origin};opacity:{opacity};transform:{transform};transition:opacity 220ms cubic-bezier(0.16,1,0.3,1),transform 300ms cubic-bezier(0.34,1.3,0.64,1);"
        )
    } else {
        format!("transform-origin:{origin};")
    };
    let label = aria_label.unwrap_or_else(|| format!("{} message", from.as_str()));

    rsx! {
        article {
            "data-slot": "message",
            "data-from": from.as_str(),
            "aria-label": "{label}",
            class: cn!(
                "group/message flex w-full items-start gap-2",
                if from == MessageFrom::User { "flex-row-reverse" } else { "flex-row" },
                class,
            ),
            style: "{motion_style}{style}",
            ..attributes,
            {children}
        }
    }
}

/// `MessageGroup`.
#[component]
pub fn MessageGroup(
    #[props(default = MessageGroupSpacing::Compact)] spacing: MessageGroupSpacing,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            "data-slot": "message-group",
            class: cn!(
                "flex w-full flex-col",
                if spacing == MessageGroupSpacing::Compact { "gap-1.5" } else { "gap-4" },
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

/// `MessageGroupProps["spacing"]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MessageGroupSpacing {
    #[default]
    Compact,
    Default,
}

/// `MessageAvatar`.
#[component]
pub fn MessageAvatar(
    #[props(default)] placeholder: bool,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            "data-slot": "message-avatar",
            "aria-hidden": if placeholder { Some("true") } else { None },
            class: cn!(
                "grid size-7 shrink-0 place-items-center overflow-hidden rounded-full bg-muted text-xs font-medium text-muted-foreground [&_img]:size-full [&_img]:object-cover [&_svg]:size-3.5",
                if placeholder { "invisible" } else { "" },
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

/// `MessageContent`.
#[component]
pub fn MessageContent(
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let from = try_use_context::<MessageFrom>().unwrap_or_default();
    rsx! {
        div {
            "data-slot": "message-content",
            class: cn!(
                "flex min-w-0 flex-1 flex-col gap-1.5",
                if from == MessageFrom::User { "items-end" } else { "items-start" },
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

/// `MessageHeader`.
#[component]
pub fn MessageHeader(
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let from = try_use_context::<MessageFrom>().unwrap_or_default();
    rsx! {
        div {
            "data-slot": "message-header",
            class: cn!(
                "flex items-center gap-1.5 px-1 text-[11px] leading-none text-muted-foreground",
                if from == MessageFrom::User { "justify-end" } else { "justify-start" },
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

/// `MessageFooter`.
#[component]
pub fn MessageFooter(
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let from = try_use_context::<MessageFrom>().unwrap_or_default();
    rsx! {
        div {
            "data-slot": "message-footer",
            class: cn!(
                "flex min-h-5 items-center gap-1 px-1 text-[11px] text-muted-foreground",
                if from == MessageFrom::User { "justify-end" } else { "justify-start" },
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

/// `MessageMarker`.
#[component]
pub fn MessageMarker(
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            "data-slot": "message-marker",
            class: cn!(
                "mx-auto flex w-fit max-w-[88%] items-center gap-1.5 rounded-full bg-muted/70 px-2.5 py-1 text-center text-xs text-muted-foreground",
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

/// `MessageTyping`.
#[component]
pub fn MessageTyping(
    #[props(default = "Responding".to_string())] label: String,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let reduce = use_reduced_motion()();
    rsx! {
        span {
            "data-slot": "message-typing",
            class: cn!("inline-flex h-5 items-center gap-1", class),
            ..attributes,
            span { class: "sr-only", "{label}" }
            for index in 0..3 {
                span {
                    key: "{index}",
                    "aria-hidden": "true",
                    class: if reduce { "size-1 rounded-full bg-current opacity-45" } else { "size-1 rounded-full bg-current animate-bounce" },
                    style: if reduce { String::new() } else { format!("animation-duration:1.05s;animation-delay:{}ms;", index * 140) },
                }
            }
        }
    }
}

/// Re-export so `message` is one import site, as in the source's `message.tsx`.
pub use crate::message_bubble::{
    MessageBubble, MessageBubbleCollapsible, MessageBubbleContent, MessageBubbleGroup,
};
pub use crate::message_scroller::MessageScroller;
