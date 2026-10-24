//! `lib/hooks/use-dismiss`.

use dioxus::prelude::*;

use crate::platform;

/// The listener body, ported from `use-dismiss.ts`; see the file for why the DOM
/// half lives in JavaScript.
const DISMISS_JS: &str = include_str!("../js/dismiss.js");

/// What the dismissing gesture does to the control it landed on.
///
/// `PassThrough` is the platform norm (native popover light-dismiss): the tap
/// closes the overlay *and* activates whatever was under it. Use `Consume` where
/// the open overlay sits over or beside controls that would be costly to trigger
/// by accident — the dismissal then swallows the activation too, so the gesture
/// only closes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DismissBehavior {
    #[default]
    PassThrough,
    Consume,
}

/// Options for [`use_dismiss`].
#[derive(Clone, Debug)]
pub struct DismissOptions {
    /// Default [`DismissBehavior::PassThrough`].
    pub behavior: DismissBehavior,
    /// Dismiss on Escape as well. Default `true`.
    pub escape: bool,
    /// CSS selectors for outside targets that should *not* dismiss.
    ///
    /// The source takes an `ignore` predicate over elements; a selector is the
    /// form that can cross the renderer boundary, and it covers every use in
    /// beUI (a trigger is marked with a `data-` attribute anyway). A target
    /// inside one of these keeps the overlay open.
    pub ignore_selectors: Vec<String>,
}

impl Default for DismissOptions {
    fn default() -> Self {
        Self {
            behavior: DismissBehavior::PassThrough,
            escape: true,
            ignore_selectors: Vec::new(),
        }
    }
}

impl DismissOptions {
    pub fn consume(mut self) -> Self {
        self.behavior = DismissBehavior::Consume;
        self
    }

    pub fn without_escape(mut self) -> Self {
        self.escape = false;
        self
    }

    pub fn ignore(mut self, selector: impl Into<String>) -> Self {
        self.ignore_selectors.push(selector.into());
        self
    }
}

/// An overlay's identity, shared by the hooks that own one.
///
/// The source passes a `ref` to `useDismiss` and two more to `useModalScope`;
/// Dioxus element refs cannot cross to the desktop renderer, so the equivalent
/// is a marker attribute the scripts find by selector.
///
/// One element carries it for both hooks: the thing that counts as "inside" for
/// a dismissal is the panel, and the panel is also what the focus trap holds.
/// beUI splits them because it portals an overlay *container* to `body` and
/// traps focus inside the panel within it; without a portal the two collapse
/// into one element, and an overlay's backdrop stays outside the scope — which
/// is what makes a click on it dismiss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayScope {
    id: u32,
}

impl OverlayScope {
    /// The value for `data-beui-scope` on the overlay's root element.
    pub fn attr(&self) -> String {
        self.id.to_string()
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    /// The selector a script uses to find the scope root.
    pub fn selector(&self) -> String {
        format!("[data-beui-scope=\"{}\"]", self.id)
    }
}

/// Close an open overlay on Escape or a pointerdown outside the scope element.
///
/// `open` is a signal rather than a `bool` because the effect has to read it
/// inside its own body to be re-run when it flips; pass the signal itself, not
/// `open()`.
///
/// The returned [`OverlayScope`] must be put on the overlay root as
/// `"data-beui-scope": scope.attr()`.
pub fn use_dismiss(
    open: Signal<bool>,
    on_dismiss: EventHandler<()>,
    options: DismissOptions,
) -> OverlayScope {
    let scope = use_hook(|| OverlayScope {
        id: platform::next_scope_id(),
    });
    let mut script_handle: Signal<Option<document::Eval>> = use_signal(|| None);

    use_effect(move || {
        let should_open = open();

        if should_open {
            // Already listening (a re-render that did not change `open`).
            if script_handle.peek().is_some() {
                return;
            }
            let script = DISMISS_JS
                .replace("__BEUI_ID__", &scope.id().to_string())
                .replace(
                    "__BEUI_ESCAPE__",
                    if options.escape { "true" } else { "false" },
                )
                .replace(
                    "__BEUI_CONSUME__",
                    if options.behavior == DismissBehavior::Consume {
                        "true"
                    } else {
                        "false"
                    },
                )
                .replace(
                    "__BEUI_IGNORE__",
                    &serde_json::to_string(&options.ignore_selectors)
                        .unwrap_or_else(|_| "[]".to_string()),
                );

            let Some(mut eval) = platform::eval(&script) else {
                return;
            };
            script_handle.set(Some(eval));
            spawn(async move {
                while let Ok(message) = eval.recv::<String>().await {
                    if message == "dismiss" {
                        on_dismiss.call(());
                    }
                }
            });

            // Closed again before the script finished installing: stop it now,
            // or the listener would outlive the overlay that asked for it.
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

    scope
}
