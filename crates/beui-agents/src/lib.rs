//! beUI agent components (`components/agents/`).
//!
//! Structured-conversation pieces: message surfaces, streaming responses,
//! tool/approval cards, citations, code blocks and the task list. The port
//! keeps the source's DOM shape and Tailwind class strings byte-for-byte; the
//! only shared helper added here is [`Icon`], which draws the `lucide-react`
//! paths from [`icons`] in one `<svg>` shell.
//!
//! Each component's module docs record where the port deviates from the
//! source.

pub mod agent_activity;
pub mod agent_code;
pub mod ai_sidebar;
pub mod approval_card;
pub mod chat_app;
pub mod citations;
pub mod code_block;
pub mod common;
pub mod disclosure;
pub mod file_diff;
pub mod icons;
pub mod image_generation;
pub mod loading_states;
pub mod message;
pub mod message_bubble;
pub mod message_context;
pub mod message_scroller;
pub mod prompt_input;
pub mod standins;
pub mod streaming_response;
pub mod timers;
pub mod todo_list;
pub mod tool_approval;
pub mod tool_result;
pub mod voice_orb;

pub use common::Icon;
pub use icons::IconName;

/// The surface components import.
pub mod prelude {
    pub use crate::common::{copy_to_clipboard, use_unique_id, Icon, RollText};
    pub use crate::icons::IconName;
    pub use beui::prelude::*;
}
