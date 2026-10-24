//! `motion/text-shimmer.tsx` + `lib/text-shimmer.ts` — a moving gradient sweep
//! clipped to the text.
//!
//! Pure CSS: one `@keyframes` block travels with the component (the registry
//! bundles imports only, so an installed copy never receives the global reset)
//! and the sweep itself is an inline `animation` declaration.

use crate::prelude::*;
use crate::text::TextTag;

/// `TEXT_SHIMMER_KEYFRAMES`, verbatim.
///
/// `!important` because the sweep is an inline style, which outranks a plain
/// rule in a media query; the marker class reaches consumers that build their
/// own span out of the exports.
pub const TEXT_SHIMMER_KEYFRAMES: &str =
    "@keyframes beui-text-shimmer{from{background-position:200% 0}to{background-position:-200% 0}}\
     @media (prefers-reduced-motion: reduce){.beui-text-shimmer{animation:none !important}}";

/// `TEXT_SHIMMER_CLASS_NAME`, verbatim.
pub const TEXT_SHIMMER_CLASS_NAME: &str = "beui-text-shimmer bg-[length:200%_100%] bg-clip-text \
     text-transparent bg-[linear-gradient(110deg,var(--muted-foreground)_30%,var(--foreground)_50%,var(--muted-foreground)_70%)]";

/// `textShimmerStyle(duration)`.
pub fn text_shimmer_style(duration: f64) -> String {
    format!("animation:beui-text-shimmer {duration}s linear infinite")
}

/// Props for [`TextShimmer`].
#[derive(Props, Clone, PartialEq)]
pub struct TextShimmerProps {
    pub children: Element,
    /// The element to render. Defaults to `span`.
    #[props(default)]
    pub r#as: TextTag,
    /// Seconds per sweep. Defaults to `2.5`.
    #[props(default = 2.5)]
    pub duration: f64,
    #[props(default)]
    pub class: Option<String>,
}

/// A shimmering text label.
#[component]
pub fn TextShimmer(props: TextShimmerProps) -> Element {
    let TextShimmerProps {
        children,
        r#as,
        duration,
        class,
    } = props;
    let class = cn!("inline-block", TEXT_SHIMMER_CLASS_NAME, class);
    let style = text_shimmer_style(duration);
    rsx! {
        style { dangerous_inner_html: TEXT_SHIMMER_KEYFRAMES }
        {match r#as {
            TextTag::Span => rsx! { span { class, style, {children} } },
            TextTag::P => rsx! { p { class, style, {children} } },
            TextTag::Div => rsx! { div { class, style, {children} } },
            TextTag::H1 => rsx! { h1 { class, style, {children} } },
            TextTag::H2 => rsx! { h2 { class, style, {children} } },
            TextTag::H3 => rsx! { h3 { class, style, {children} } },
            TextTag::H4 => rsx! { h4 { class, style, {children} } },
            TextTag::H5 => rsx! { h5 { class, style, {children} } },
            TextTag::H6 => rsx! { h6 { class, style, {children} } },
        }}
    }
}
