//! `motion/card.tsx` — the card surface and its parts.
//!
//! The only motion is the press: an interactive card scales to `0.985` on
//! pointerdown and springs back on release, on `SPRING_PRESS`. The source
//! drives it by hand rather than with `whileTap`, because `whileTap` would give
//! the card `tabIndex 0` — an unnamed tab stop ahead of the control that owns
//! the card. The same reason holds here, so the press is hand-driven too.
//!
//! The size travels through a context so each part pads itself on the inline
//! axis: a part can be left out without the others losing their edges.

use dioxus::html::input_data::MouseButton;
use dioxus::html::point_interaction::PointerInteraction;
use dioxus::prelude::*;

use beui_core::cn;
use beui_core::ease::SPRING_PRESS;
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;

use crate::motion::{animate_number, number, scale_style, spring, use_number_motion};

/// Spread onto the one control that owns a card (a [`Checkbox`](crate::checkbox::Checkbox)
/// or radio root, or a link or button) to stretch its hit area over the whole
/// card. Any other interactive element inside that card needs `relative z-10`
/// to stay on top.
pub const CARD_TARGET_CLASS: &str = "after:absolute after:inset-0";

/// Default `Outlined`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CardVariant {
    #[default]
    Outlined,
    Muted,
}

/// Spacing for every part. Default `Default`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CardSize {
    #[default]
    Default,
    Compact,
}

impl CardVariant {
    const fn class(self) -> &'static str {
        match self {
            Self::Outlined => "border-border bg-background",
            Self::Muted => "border-transparent bg-muted",
        }
    }
}

impl CardSize {
    /// The root owns the block padding and the gap.
    const fn root_class(self) -> &'static str {
        match self {
            Self::Default => "gap-6 py-6",
            Self::Compact => "gap-4 py-4",
        }
    }

    /// Each part pads itself on the inline axis.
    const fn inset_class(self) -> &'static str {
        match self {
            Self::Default => "px-6",
            Self::Compact => "px-4",
        }
    }
}

#[derive(Clone, Copy)]
struct CardSizeContext(CardSize);

/// The card surface.
///
/// `interactive` is hover and press feedback for a card that is one target as a
/// whole. It is visual only: the card never takes focus, so the control that
/// owns it goes inside and stretches with [`CARD_TARGET_CLASS`].
#[component]
pub fn Card(
    #[props(default)] variant: CardVariant,
    #[props(default)] size: CardSize,
    #[props(default)] interactive: bool,
    #[props(default)] selected: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion();
    let mut scale = use_number_motion(1.0);

    use_context_provider(|| CardSizeContext(size));

    let release = move |_event: Event<PointerData>| {
        if interactive && !reduce() {
            animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
        }
    };

    let press = move |event: Event<PointerData>| {
        if !interactive || reduce() {
            return;
        }
        if !event.is_primary() || event.trigger_button() != Some(MouseButton::Primary) {
            return;
        }
        // Capture so the release still lands on the card after the pointer
        // leaves it. Best effort: the selector fallback needs a stable handle,
        // and the card has no scope id, so a refused capture just means the
        // release is missed on the non-web renderers.
        platform::capture_pointer(&event);
        animate_number(&mut scale, 0.985, spring(SPRING_PRESS));
    };

    let style = if interactive {
        scale_style(number(&scale))
    } else {
        String::new()
    };

    rsx! {
        div {
            class: cn!(
                "relative flex flex-col rounded-2xl border text-card-foreground",
                "transition-[border-color,box-shadow,background-color] duration-200",
                variant.class(),
                size.root_class(),
                if interactive { "cursor-pointer hover:border-border-strong has-focus-visible:outline-2 has-focus-visible:outline-offset-2 has-focus-visible:outline-foreground" } else { "" },
                if selected { "border-primary shadow-[0_0_0_1px_var(--primary)] hover:border-primary" } else { "" },
                class,
            ),
            style,
            "data-slot": "card",
            "data-selected": selected.then_some("true"),
            onpointerdown: press,
            onpointerup: release,
            onpointercancel: release,
            onclick: move |event| {
                if let Some(onclick) = onclick {
                    onclick.call(event);
                }
            },
            {children}
        }
    }
}

/// Title and description, with an optional [`CardAction`] pinned to the top right.
#[component]
pub fn CardHeader(
    #[props(default)] id: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let size = use_context::<CardSizeContext>().0;
    rsx! {
        div {
            class: cn!(
                "grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5",
                "has-data-[slot=card-action]:grid-cols-[1fr_auto]",
                size.inset_class(),
                class,
            ),
            "data-slot": "card-header",
            id,
            {children}
        }
    }
}

/// Heading level, when the card starts a section. Default `Div`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CardTitleAs {
    #[default]
    Div,
    H2,
    H3,
    H4,
}

#[component]
pub fn CardTitle(
    #[props(default)] as_: CardTitleAs,
    #[props(default)] id: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let class = cn!(
        "text-[15px] leading-snug font-semibold text-foreground",
        class,
    );
    match as_ {
        CardTitleAs::Div => rsx! { div { id, class, "data-slot": "card-title", {children} } },
        CardTitleAs::H2 => rsx! { h2 { id, class, "data-slot": "card-title", {children} } },
        CardTitleAs::H3 => rsx! { h3 { id, class, "data-slot": "card-title", {children} } },
        CardTitleAs::H4 => rsx! { h4 { id, class, "data-slot": "card-title", {children} } },
    }
}

#[component]
pub fn CardDescription(
    #[props(default)] id: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        p {
            class: cn!("text-sm leading-relaxed text-muted-foreground", class),
            "data-slot": "card-description",
            id,
            {children}
        }
    }
}

/// Sits beside the title and description inside [`CardHeader`].
#[component]
pub fn CardAction(
    #[props(default)] id: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        // Flex, not block: an inline-flex control (Checkbox, Radio) would
        // otherwise sit on a text baseline that moves when its mark appears.
        div {
            class: cn!(
                "col-start-2 row-span-2 row-start-1 flex items-center self-start justify-self-end",
                class,
            ),
            "data-slot": "card-action",
            id,
            {children}
        }
    }
}

#[component]
pub fn CardContent(
    #[props(default)] id: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let size = use_context::<CardSizeContext>().0;
    rsx! {
        div {
            class: cn!("text-sm", size.inset_class(), class),
            "data-slot": "card-content",
            id,
            {children}
        }
    }
}

#[component]
pub fn CardFooter(
    #[props(default)] id: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let size = use_context::<CardSizeContext>().0;
    rsx! {
        div {
            class: cn!("flex items-center gap-2", size.inset_class(), class),
            "data-slot": "card-footer",
            id,
            {children}
        }
    }
}
