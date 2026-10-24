//! `motion/smooth-scroll.tsx` + `motion/scroll-to.tsx` — a scroll provider and a
//! button that glides to a target.
//!
//! beUI drives this with Lenis (`lenis/react`), which the port plan keeps as JS
//! interop; the smoothing engine is not wired here. What is ported is
//! everything around it: the `SmoothScrollApi` (scroll offset, 0→1 progress and
//! signed velocity), the native scroll listener the source uses on its
//! reduced-motion and no-provider paths, and `ScrollTo`, which resolves a
//! selector to an offset and scrolls the active source. `behavior: "smooth"` is
//! the native glide; a `duration` override therefore has no effect.

use crate::prelude::*;
use beui_core::platform::js_string;

/// The shared scroll state.
#[derive(Clone)]
pub struct SmoothScrollApi {
    /// The scroller's current offset in px.
    pub scroll_y: Signal<f64>,
    /// The offset as 0→1 of the scrollable height.
    pub progress: Signal<f64>,
    /// Signed scroll velocity (px/frame).
    pub velocity: Signal<f64>,
    /// CSS selector of the contained scroller; `None` is the window.
    pub container: Option<String>,
}

impl SmoothScrollApi {
    /// Scroll to `target` (a CSS selector), offset by `offset` px.
    pub fn scroll_to(&self, target: &str, offset: f64, reduce: bool) {
        let root = match &self.container {
            Some(selector) => format!("document.querySelector({})", js_string(selector)),
            None => "null".to_string(),
        };
        let script = format!(
            "const el = document.querySelector({target});\n\
             if (!el) return;\n\
             const root = {root};\n\
             const top = root ? el.offsetTop + {offset} : el.getBoundingClientRect().top + window.scrollY + {offset};\n\
             (root || window).scrollTo({{ top: top, behavior: {behavior} }});",
            target = js_string(target),
            offset = offset,
            behavior = if reduce { "\"auto\"" } else { "\"smooth\"" },
        );
        beui_core::platform::fire(&script);
    }
}

const SCROLL_STATE_JS: &str = r#"
const root = __ROOT__;
let lastY = 0;
let lastT = performance.now();
function compute() {
  let y = 0;
  let max = 0;
  if (root) {
    y = root.scrollTop;
    max = Math.max(0, root.scrollHeight - root.clientHeight);
  } else {
    y = window.scrollY;
    max = Math.max(0, document.documentElement.scrollHeight - window.innerHeight);
  }
  const now = performance.now();
  const dt = (now - lastT) || 16;
  const velocity = ((y - lastY) / dt) * 16;
  lastY = y;
  lastT = now;
  dioxus.send([y, max > 0 ? y / max : 0, velocity]);
}
const scroller = root || window;
scroller.addEventListener("scroll", compute, { passive: true });
window.addEventListener("resize", compute, { passive: true });
compute();
"#;

/// Track one scroller's offset, progress and velocity.
pub fn use_scroll_state(container: Option<String>) -> SmoothScrollApi {
    let mut scroll_y = use_signal(|| 0.0f64);
    let mut progress = use_signal(|| 0.0f64);
    let mut velocity = use_signal(|| 0.0f64);
    let root = container
        .as_deref()
        .map(|selector| format!("document.querySelector({})", js_string(selector)))
        .unwrap_or_else(|| "null".to_string());
    let script = SCROLL_STATE_JS.replace("__ROOT__", &root);

    use_effect(move || {
        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(sample) = eval.recv::<Vec<f64>>().await {
                if sample.len() < 3 {
                    continue;
                }
                scroll_y.set(sample[0]);
                progress.set(sample[1]);
                velocity.set(sample[2]);
            }
        });
    });

    SmoothScrollApi {
        scroll_y,
        progress,
        velocity,
        container,
    }
}

/// Props for [`SmoothScroll`].
#[derive(Props, Clone, PartialEq)]
pub struct SmoothScrollProps {
    /// Drive the window when true, a contained area when false.
    #[props(default = true)]
    pub root: bool,
    #[props(default)]
    pub class: Option<String>,
    pub children: Element,
}

/// A scroll surface that shares its scroll state with descendants.
#[component]
pub fn SmoothScroll(props: SmoothScrollProps) -> Element {
    let SmoothScrollProps {
        root,
        class,
        children,
    } = props;
    let scope = use_hook(|| {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    });
    let selector = format!("[data-beui-smooth-scroll=\"{scope}\"]");
    let api = use_scroll_state(if root { None } else { Some(selector) });
    use_context_provider(|| api);

    if root {
        rsx! { {children} }
    } else {
        rsx! {
            div { "data-beui-smooth-scroll": "{scope}", class, {children} }
        }
    }
}

/// Read the page's scroll state, inside a provider or without one.
pub fn use_smooth_scroll() -> SmoothScrollApi {
    let context = try_use_context::<SmoothScrollApi>();
    let fallback = use_scroll_state(None);
    context.unwrap_or(fallback)
}

/// Props for [`ScrollTo`].
#[derive(Props, Clone, PartialEq)]
pub struct ScrollToProps {
    /// A CSS selector for the target.
    pub to: String,
    /// Extra px offset (e.g. to clear a sticky header).
    #[props(default)]
    pub offset: f64,
    /// Unused: the native glide has no duration knob.
    #[props(default)]
    pub duration: Option<f64>,
    #[props(default)]
    pub class: Option<String>,
    pub children: Element,
}

/// A button that smooth-scrolls to a target.
#[component]
pub fn ScrollTo(props: ScrollToProps) -> Element {
    let ScrollToProps {
        to,
        offset,
        duration,
        class,
        children,
    } = props;
    let _ = duration;
    let api = use_smooth_scroll();
    let reduce = use_reduced_motion();
    let class = cn!(class);

    rsx! {
        button {
            r#type: "button",
            class,
            onclick: move |_| api.scroll_to(&to, offset, reduce()),
            {children}
        }
    }
}
