//! Shared helpers for the agents components.
//!
//! beUI's agents set reaches into two shared places on every page: `lucide-react`
//! for icons and `navigator.clipboard` for the copy actions. The icons live in
//! [`crate::icons`] (path data only); [`Icon`] renders them. The clipboard write
//! goes through `document::eval` so it works on the WebView renderers too, and
//! the "copied" flag reset is a `setTimeout` that reports back over the same
//! channel — the source's `window.setTimeout(() => setCopied(false), 1600)`.

use beui_core::platform;
use dioxus::prelude::*;

use crate::icons::IconName;

/// Draw a Lucide icon: the shared `<svg>` shell plus the glyph's path data.
///
/// Lucide's defaults are baked in (`viewBox="0 0 24 24"`, `fill="none"`,
/// `stroke="currentColor"`, width 2, round caps and joins), which is what
/// `lucide-react` puts on every icon. `class` is appended verbatim — the source
/// drives size and colour entirely from Tailwind utilities.
#[component]
pub fn Icon(
    name: IconName,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let inner = name.inner();
    rsx! {
        svg {
            class: "{class}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            dangerous_inner_html: "{inner}",
            ..attributes,
        }
    }
}

/// A stable DOM id for this component instance.
///
/// React's `useId()`; the port's `next_scope_id()` counter gives the same
/// process-unique, mutation-stable value. `prefix` replaces the framework's
/// `:`-delimited form so the id is safe in a CSS selector or a `url(#…)`
/// reference — the source strips the colons for the same reason.
pub fn use_unique_id(prefix: &str) -> String {
    use_hook(|| format!("{prefix}-{}", platform::next_scope_id()))
}

/// `navigator.clipboard.writeText(text)`, fire-and-forget on every renderer.
pub fn copy_to_clipboard(text: &str) {
    platform::fire(&format!(
        "if (navigator.clipboard && navigator.clipboard.writeText) {{ navigator.clipboard.writeText({}); }}",
        platform::js_string(text),
    ));
}

/// Reset a `copied` flag after `ms`, the source's `window.setTimeout` pair.
///
/// The timer lives in the page (so it is a browser/webview timer either way) and
/// reports back over the evaluator channel; a second copy before it fires leaves
/// the first timer to clear the flag that the second copy just set, which is the
/// same race the source has with `copyTimer.current`.
pub fn schedule_reset(mut flag: Signal<bool>, ms: u32) {
    spawn(async move {
        let Some(mut eval) = platform::eval(&format!("setTimeout(() => dioxus.send(true), {ms});"))
        else {
            return;
        };
        if eval.recv::<bool>().await.is_ok() {
            flag.set(false);
        }
    });
}

/// `element.scrollTo({ top: scrollHeight })`, guarded so a non-overflowing box is
/// left untouched. The selector is the port's handle on an element the WebView
/// renderers cannot reach directly.
pub fn scroll_to_bottom(selector: &str, smooth: bool) {
    let behavior = if smooth { "smooth" } else { "auto" };
    platform::fire(&format!(
        "(function(){{var el=document.querySelector({selector});if(!el)return;if(el.scrollHeight<=el.clientHeight)return;el.scrollTo({{top:el.scrollHeight,behavior:{behavior}}});}})();",
        selector = platform::js_string(selector),
        behavior = platform::js_string(behavior),
    ));
}

/// Stand-in for the motion crate's `ActionSwapRollText`.
///
/// The rolling swap animation belongs to the motion lane; the agents components
/// only need the same DOM (`<span>` around the value) so their layout and
/// accessibility tree are unchanged. When `beui::action_swap_roll` lands this
/// call site can switch to it without moving anything.
#[component]
pub fn RollText(value: String) -> Element {
    rsx! {
        span { "{value}" }
    }
}

/// Scroll an element back to its top (the activity disclosure reopening).
pub fn scroll_to_top(selector: &str) {
    platform::fire(&format!(
        "(function(){{var el=document.querySelector({selector});if(!el)return;el.scrollTo({{top:0}});}})();",
        selector = platform::js_string(selector),
    ));
}
