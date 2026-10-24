//! `agents/message-context.tsx` — the alignment hand-off from a message row to
//! the bubbles inside it.
//!
//! React provides `MessageSide | undefined`; here the same value is a plain
//! context read with [`try_use_context`], falling back to `None` exactly like
//! the source's default. `MessageFrom` rides along so `MessageContent`,
//! `MessageHeader` and `MessageFooter` can flip their own alignment.

/// `MessageFrom`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MessageFrom {
    User,
    #[default]
    Assistant,
}

impl MessageFrom {
    /// `Message`'s `data-from`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// `MessageSide`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageSide {
    Start,
    End,
}

impl MessageSide {
    /// `MessageBubble`'s `data-align`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}
