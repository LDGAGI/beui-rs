//! `agents/message-bubble.tsx` — bubble variants, content surface and the
//! collapsible clamp.
//!
//! The bubble box itself is static in the source (`initial={false}`); the
//! surface and the content column play a short pop on mount. That pop is a CSS
//! transition here (`opacity 120ms`, `scale` on the `0.34/1.3/0.64/1` curve in
//! place of `SPRING_LAYOUT`). `render` (replace the content element) is not
//! ported — Dioxus has no `cloneElement`, so content always renders as a `div`;
//! this is deliberate.

use beui::prelude::*;

use crate::common::{use_unique_id, Icon};
use crate::icons::IconName;
use crate::message_context::MessageSide;

/// `MessageBubbleVariant`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MessageBubbleVariant {
    Solid,
    #[default]
    Soft,
    Tint,
    Outline,
    Ghost,
    Danger,
}

/// `MessageBubbleAlign`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MessageBubbleAlign {
    #[default]
    Start,
    End,
}

impl MessageBubbleAlign {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

/// `MessageBubbleContext`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BubbleContext {
    align: MessageBubbleAlign,
    animate_in: bool,
    variant: MessageBubbleVariant,
}

impl Default for BubbleContext {
    fn default() -> Self {
        Self {
            align: MessageBubbleAlign::Start,
            animate_in: true,
            variant: MessageBubbleVariant::Soft,
        }
    }
}

impl BubbleContext {
    fn read() -> Self {
        try_use_context::<BubbleContext>().unwrap_or_default()
    }
}

/// `MessageBubble`.
#[component]
pub fn MessageBubble(
    #[props(default)] variant: MessageBubbleVariant,
    #[props(default)] align: Option<MessageBubbleAlign>,
    #[props(default)] animate_in: bool,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let message_side = try_use_context::<MessageSide>();
    let resolved_align = align.unwrap_or(match message_side {
        Some(MessageSide::Start) => MessageBubbleAlign::Start,
        Some(MessageSide::End) => MessageBubbleAlign::End,
        None => MessageBubbleAlign::Start,
    });
    use_context_provider(|| BubbleContext {
        align: resolved_align,
        animate_in,
        variant,
    });

    rsx! {
        div {
            "data-slot": "message-bubble",
            "data-align": resolved_align.as_str(),
            "data-variant": variant_name(variant),
            class: cn!(
                "group/bubble flex w-full flex-col",
                if resolved_align == MessageBubbleAlign::End { "items-end" } else { "items-start" },
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

fn variant_name(variant: MessageBubbleVariant) -> &'static str {
    match variant {
        MessageBubbleVariant::Solid => "solid",
        MessageBubbleVariant::Soft => "soft",
        MessageBubbleVariant::Tint => "tint",
        MessageBubbleVariant::Outline => "outline",
        MessageBubbleVariant::Ghost => "ghost",
        MessageBubbleVariant::Danger => "danger",
    }
}

fn bubble_content_class(variant: MessageBubbleVariant, interactive: bool) -> String {
    cn!(
        "relative z-0 min-w-9 max-w-[82%] rounded-2xl px-3.5 py-2.5 text-sm leading-6 text-foreground",
        "[&_a]:font-medium [&_a]:underline [&_a]:underline-offset-4 [&_code]:rounded [&_code]:bg-background/60 [&_code]:px-1 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[0.9em] [&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-5 [&_p+p]:mt-2 [&_pre]:my-2 [&_pre]:overflow-x-auto [&_pre]:rounded-xl [&_pre]:bg-background/60 [&_pre]:p-3 [&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-5",
        if variant == MessageBubbleVariant::Solid { "text-background" } else { "" },
        if variant == MessageBubbleVariant::Ghost { "w-full max-w-none rounded-none px-0 py-0" } else { "" },
        if variant == MessageBubbleVariant::Danger { "text-destructive" } else { "" },
        if interactive { "cursor-pointer text-left outline-none transition-[background-color,color,transform] duration-150 hover:brightness-[0.98] focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.99]" } else { "" },
    )
}

fn bubble_surface_class(variant: MessageBubbleVariant, align: MessageBubbleAlign) -> String {
    cn!(
        "pointer-events-none absolute inset-0 -z-10 rounded-[inherit]",
        if align == MessageBubbleAlign::End {
            "origin-bottom-right"
        } else {
            "origin-bottom-left"
        },
        match variant {
            MessageBubbleVariant::Solid => "bg-foreground",
            MessageBubbleVariant::Soft | MessageBubbleVariant::Tint => "bg-muted",
            MessageBubbleVariant::Outline => "border border-border/70 bg-background",
            MessageBubbleVariant::Danger => "bg-destructive/10",
            MessageBubbleVariant::Ghost => "",
        },
    )
}

/// `MessageBubbleContent`.
#[component]
pub fn MessageBubbleContent(
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let ctx = BubbleContext::read();
    let mut mounted = use_signal(|| false);
    use_effect(move || mounted.set(true));

    let animate = ctx.animate_in && !reduce;
    let settled = mounted() || !animate;
    let surface_opacity = if settled { 1 } else { 0 };
    let surface_scale = if settled { 1.0 } else { 0.92 };
    let surface_style = if animate {
        format!(
            "opacity:{surface_opacity};transform:scale({surface_scale});transition:opacity 120ms cubic-bezier(0.16,1,0.3,1),transform 300ms cubic-bezier(0.34,1.3,0.64,1);"
        )
    } else {
        String::new()
    };
    let content_style = if animate {
        format!(
            "opacity:{surface_opacity};transition:opacity 120ms cubic-bezier(0.16,1,0.3,1) 40ms;"
        )
    } else {
        String::new()
    };

    rsx! {
        div {
            "data-slot": "message-bubble-content",
            class: cn!(bubble_content_class(ctx.variant, false), class),
            ..attributes,
            if ctx.variant != MessageBubbleVariant::Ghost {
                span {
                    "aria-hidden": "true",
                    class: bubble_surface_class(ctx.variant, ctx.align),
                    style: "{surface_style}",
                }
            }
            div { class: "relative", style: "{content_style}", {children} }
        }
    }
}

/// `MessageBubbleGroup`.
#[component]
pub fn MessageBubbleGroup(
    #[props(default = MessageBubbleSpacing::Compact)] spacing: MessageBubbleSpacing,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            "data-slot": "message-bubble-group",
            class: cn!(
                "flex w-full flex-col",
                if spacing == MessageBubbleSpacing::Compact { "gap-1.5" } else { "gap-3" },
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

/// `MessageBubbleGroupProps["spacing"]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MessageBubbleSpacing {
    #[default]
    Compact,
    Default,
}

/// `MessageBubbleCollapsible`.
#[component]
pub fn MessageBubbleCollapsible(
    #[props(default)] open: Option<bool>,
    #[props(default)] default_open: bool,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = 4)] collapsed_lines: u8,
    #[props(default)] more_label: Option<Element>,
    #[props(default)] less_label: Option<Element>,
    #[props(default)] content_class_name: String,
    #[props(default)] trigger_class_name: String,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let content_id = use_unique_id("bubble-collapsible");
    let mut internal_open = use_signal(|| default_open);
    let current_open = open.unwrap_or(internal_open());

    let mut set_open = move |next: bool| {
        if open.is_none() {
            internal_open.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    };

    let clamp = match collapsed_lines {
        2 => "line-clamp-2",
        3 => "line-clamp-3",
        5 => "line-clamp-5",
        6 => "line-clamp-6",
        _ => "line-clamp-4",
    };
    let rotate = if current_open { 180 } else { 0 };
    let duration = if reduce { 0 } else { 200 };
    let more = more_label.unwrap_or_else(|| rsx! { "Show more" });
    let less = less_label.unwrap_or_else(|| rsx! { "Show less" });
    let label = if current_open { less } else { more };

    rsx! {
        div {
            "data-slot": "message-bubble-collapsible",
            "data-state": if current_open { "open" } else { "closed" },
            class: cn!("w-full", class),
            ..attributes,
            div {
                id: "{content_id}",
                class: cn!(
                    "transition-[mask-image] duration-200",
                    if current_open { "" } else { clamp },
                    if current_open { "" } else { "[mask-image:linear-gradient(to_bottom,#000_68%,transparent_100%)]" },
                    content_class_name,
                ),
                {children}
            }
            button {
                r#type: "button",
                "aria-expanded": if current_open { "true" } else { "false" },
                "aria-controls": "{content_id}",
                onclick: move |_| set_open(!current_open),
                class: cn!(
                    "mt-2 inline-flex h-7 items-center gap-1 rounded-full px-2 text-xs font-medium text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                    trigger_class_name,
                ),
                span { {label} }
                span {
                    "aria-hidden": "true",
                    style: "transform:rotate({rotate}deg);transition:transform {duration}ms cubic-bezier(0.34,1.56,0.64,1);",
                    Icon { name: IconName::ChevronDown, class: "size-3.5" }
                }
            }
        }
    }
}
