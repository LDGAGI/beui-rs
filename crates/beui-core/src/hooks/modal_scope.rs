//! `lib/hooks/use-modal-scope`.

use dioxus::prelude::*;

use crate::platform;

use super::dismiss::OverlayScope;

/// The listener and focus body, ported from `use-modal-scope.ts`; see the file
/// for why the DOM half lives in JavaScript.
const MODAL_SCOPE_JS: &str = include_str!("../js/modal_scope.js");

/// Own a modal scope for as long as `open` is true.
///
/// While open, this:
///
/// - sets `inert` on everything outside the scope root, so nothing behind the
///   modal can be focused, clicked or read by a screen reader;
/// - locks `body` overflow, counting owners so nested modals do not unlock each
///   other;
/// - keeps focus inside the panel — focusing the first focusable on open,
///   pulling focus back when it escapes, and cycling `Tab` / `Shift+Tab` at the
///   ends;
/// - closes on Escape, swallowing the key so nothing behind it sees it either;
/// - restores focus to whatever was focused before the modal opened, and
///   restores the previous `inert` state of every sibling, on the way out.
///
/// "Outside the panel" is every sibling of the panel at every level up to
/// `body`, not just `body`'s own children. The source can assume the overlay has been
/// portaled to `body`; there is no portal yet (moving a rendered subtree breaks
/// Dioxus's diffing, which still points at the original parent), and inerting the
/// siblings at each level is the same set of elements either way. See
/// `src/js/modal_scope.js`.
///
/// Two overlapping modals nest correctly, because the counters live on `window`
/// and each scope restores what it found.
///
/// The wiring the component owes, transcribed from the source: the panel — the
/// element that counts as "inside" for dismissal and that the trap holds focus
/// within — carries the scope marker.
///
/// ```ignore
/// div { "data-beui-scope": scope.attr(), role: "dialog", "aria-modal": "true", .. }
/// ```
///
/// `open` is a signal rather than a `bool` because the effect has to read it
/// inside its own body to be re-run when it flips; pass the signal itself.
pub fn use_modal_scope(open: Signal<bool>, on_close: EventHandler<()>, scope: OverlayScope) {
    let mut script_handle: Signal<Option<document::Eval>> = use_signal(|| None);

    use_effect(move || {
        let should_open = open();

        if should_open {
            // Already open (a re-render that did not change `open`).
            if script_handle.peek().is_some() {
                return;
            }
            let script = MODAL_SCOPE_JS.replace("__BEUI_ID__", &scope.id().to_string());

            let Some(mut eval) = platform::eval(&script) else {
                return;
            };
            script_handle.set(Some(eval));
            spawn(async move {
                while let Ok(message) = eval.recv::<String>().await {
                    if message == "close" {
                        on_close.call(());
                    }
                }
            });

            // Closed again before the script finished installing: tear it down
            // now, or the trap would outlive the modal that asked for it.
            if !*open.peek() {
                let pending = *script_handle.peek();
                if let Some(eval) = pending {
                    let _ = eval.send("close");
                }
                script_handle.set(None);
            }
        } else {
            let pending = *script_handle.peek();
            if let Some(eval) = pending {
                let _ = eval.send("close");
                script_handle.set(None);
            }
        }
    });
}
