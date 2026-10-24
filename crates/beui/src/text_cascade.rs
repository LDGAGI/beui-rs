//! `motion/text-cascade.tsx` — the cascade variant of [`crate::ActionSwapText`]
//! with a text-first API.

use crate::action_swap::{ActionSwapAnimation, ActionSwapText};
use crate::prelude::*;

/// Props for [`TextCascade`].
#[derive(Props, Clone, PartialEq)]
pub struct TextCascadeProps {
    /// Current text. Changing it cascades the letters to the new value.
    pub text: String,
    #[props(default)]
    pub class: Option<String>,
}

/// Letter-by-letter slot roll for standalone text.
#[component]
pub fn TextCascade(props: TextCascadeProps) -> Element {
    let TextCascadeProps { text, class } = props;
    rsx! {
        ActionSwapText {
            value: text.clone(),
            label: text,
            animation: ActionSwapAnimation::Cascade,
            class,
        }
    }
}
