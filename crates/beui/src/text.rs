//! The `as` prop beUI's text components take.
//!
//! React passes an arbitrary `ElementType`; here the handful of tags the
//! previews and usage sites actually reach for is an enum, which keeps the
//! `rsx!` match exhaustive and the rendered tag static.

/// The element a text component renders as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextTag {
    #[default]
    Span,
    P,
    Div,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl TextTag {
    /// The tag name, for `data-*` or debugging.
    pub const fn name(self) -> &'static str {
        match self {
            TextTag::Span => "span",
            TextTag::P => "p",
            TextTag::Div => "div",
            TextTag::H1 => "h1",
            TextTag::H2 => "h2",
            TextTag::H3 => "h3",
            TextTag::H4 => "h4",
            TextTag::H5 => "h5",
            TextTag::H6 => "h6",
        }
    }
}
