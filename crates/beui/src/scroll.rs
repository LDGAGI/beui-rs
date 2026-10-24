//! Scroll- and viewport-driven primitives the lane-d components share.
//!
//! Motion's `useInView` and `useScroll` are the two hooks beUI reaches for
//! whenever an effect is tied to the page rather than to a gesture. There is no
//! renderer-agnostic Rust equivalent, so both are built on `document::eval`:
//! the script installs the observer or the scroll listener, streams the value
//! back over the eval channel, and the component renders it reactively. That is
//! the one place in this port where a DOM read is not a one-shot — but the DOM
//! write is still the reactive `style` attribute, never a script.

use crate::prelude::*;
use beui_core::platform;
use dioxus_motion::prelude::AnimationManager;

/// The `amount` a reveal needs before it fires, in motion's three spellings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum InViewAmount {
    /// Any part of the element is visible (`"some"`).
    #[default]
    Some,
    /// The whole element is visible (`"all"`).
    All,
    /// A fraction of the element is visible.
    Fraction(f64),
}

impl InViewAmount {
    /// The `IntersectionObserver` threshold.
    pub fn threshold(self) -> f64 {
        match self {
            InViewAmount::Some => 0.0,
            InViewAmount::All => 1.0,
            InViewAmount::Fraction(value) => value.clamp(0.0, 1.0),
        }
    }
}

/// `useInView` options.
#[derive(Clone, Debug, Default)]
pub struct InViewOptions {
    /// Reveal once and disconnect (default) or keep reporting both edges.
    pub once: bool,
    pub amount: InViewAmount,
    /// CSS selector for the scroll root; `None` is the viewport.
    pub root: Option<String>,
}

/// A handle to one element's visibility.
#[derive(Clone, Copy)]
pub struct InView {
    id: u32,
    value: Signal<bool>,
}

impl InView {
    /// The value for the element's `data-beui-inview` attribute.
    pub fn attr(&self) -> String {
        self.id.to_string()
    }

    /// Whether the element is in view.
    pub fn value(&self) -> bool {
        (self.value)()
    }

    /// The underlying signal, for callers that want to read it reactively.
    pub fn signal(&self) -> Signal<bool> {
        self.value
    }
}

const IN_VIEW_JS: &str = r#"
(function () {
  const el = document.querySelector(__SELECTOR__);
  if (!el) { dioxus.send(false); return; }
  const root = __ROOT__;
  const once = __ONCE__;
  const observer = new IntersectionObserver(function (entries) {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        dioxus.send(true);
        if (once) observer.disconnect();
      } else if (!once) {
        dioxus.send(false);
      }
    }
  }, { root: root, threshold: __THRESHOLD__ });
  observer.observe(el);
})();
"#;

/// `useInView`. Returns the attribute value to put on the observed element.
pub fn use_in_view(options: InViewOptions) -> InView {
    let id = use_hook(platform::next_scope_id);
    let value = use_signal(|| false);
    let selector = format!("[data-beui-inview=\"{id}\"]");
    let root = options
        .root
        .as_deref()
        .map(|selector| format!("document.querySelector({})", platform::js_string(selector)))
        .unwrap_or_else(|| "null".to_string());
    let script = IN_VIEW_JS
        .replace("__SELECTOR__", &platform::js_string(&selector))
        .replace("__ROOT__", &root)
        .replace("__ONCE__", if options.once { "true" } else { "false" })
        .replace("__THRESHOLD__", &options.amount.threshold().to_string());

    use_effect(move || {
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        let mut value = value;
        spawn(async move {
            while let Ok(next) = eval.recv::<bool>().await {
                value.set(next);
            }
        });
    });

    InView { id, value }
}

const SCROLL_Y_JS: &str = r#"
(function () {
  const root = __ROOT__;
  function compute() {
    let p = 0;
    if (root) {
      const max = root.scrollHeight - root.clientHeight;
      p = max > 0 ? root.scrollTop / max : 0;
    } else {
      const doc = document.documentElement;
      const max = doc.scrollHeight - window.innerHeight;
      p = max > 0 ? window.scrollY / max : 0;
    }
    dioxus.send(p);
  }
  const scroller = root || window;
  scroller.addEventListener("scroll", compute, { passive: true });
  window.addEventListener("resize", compute, { passive: true });
  compute();
})();
"#;

/// `useScroll({ container }).scrollYProgress` — the scroller's own 0→1 progress.
///
/// `container` is a CSS selector for a contained scroll area; `None` tracks the
/// page. Unlike motion's hook this is a plain signal rather than a motion value,
/// so a consumer that wants smoothing runs it through [`use_spring_value`].
pub fn use_scroll_y_progress(container: Option<String>) -> Signal<f64> {
    let mut value = use_signal(|| 0.0f64);
    let root = container
        .as_deref()
        .map(|selector| format!("document.querySelector({})", platform::js_string(selector)))
        .unwrap_or_else(|| "null".to_string());
    let script = SCROLL_Y_JS.replace("__ROOT__", &root);

    use_effect(move || {
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(next) = eval.recv::<f64>().await {
                value.set(next);
            }
        });
    });

    value
}

const PARALLAX_JS: &str = r#"
(function () {
  const el = document.querySelector(__SELECTOR__);
  if (!el) { dioxus.send(0); return; }
  const root = __ROOT__;
  function compute() {
    const t = el.getBoundingClientRect();
    const c = root
      ? root.getBoundingClientRect()
      : { top: 0, bottom: window.innerHeight, height: window.innerHeight };
    const total = c.height + t.height;
    const p = total > 0 ? (c.bottom - t.top) / total : 0;
    dioxus.send(p);
  }
  const scroller = root || window;
  scroller.addEventListener("scroll", compute, { passive: true });
  window.addEventListener("resize", compute, { passive: true });
  compute();
})();
"#;

/// `useScroll({ target, offset: ["start end", "end start"] })` — how far the
/// element has travelled across its scroll root, `0` at the bottom edge and `1`
/// at the top.
#[derive(Clone, Copy)]
pub struct ParallaxProgress {
    id: u32,
    value: Signal<f64>,
}

impl ParallaxProgress {
    /// The value for the element's `data-beui-parallax` attribute.
    pub fn attr(&self) -> String {
        self.id.to_string()
    }

    /// The progress signal.
    pub fn signal(&self) -> Signal<f64> {
        self.value
    }
}

/// Track one element's travel through its scroll root.
pub fn use_parallax_progress(container: Option<String>) -> ParallaxProgress {
    let id = use_hook(platform::next_scope_id);
    let mut value = use_signal(|| 0.0f64);
    let selector = format!("[data-beui-parallax=\"{id}\"]");
    let root = container
        .as_deref()
        .map(|selector| format!("document.querySelector({})", platform::js_string(selector)))
        .unwrap_or_else(|| "null".to_string());
    let script = PARALLAX_JS
        .replace("__SELECTOR__", &platform::js_string(&selector))
        .replace("__ROOT__", &root);

    use_effect(move || {
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(next) = eval.recv::<f64>().await {
                value.set(next);
            }
        });
    });

    ParallaxProgress { id, value }
}

/// Follow `source` with a spring, the way motion's `useSpring(value, config)`
/// does: each new value re-targets the spring from wherever it currently is.
///
/// Returns the current value as a plain `f32`, read reactively during render.
pub fn use_spring_value(
    source: Signal<f64>,
    spring: dioxus_motion::prelude::Spring,
    enabled: bool,
) -> f32 {
    let mut motion =
        dioxus_motion::prelude::use_motion(0.0f32).expect("0.0 is a finite animation value");
    use_effect(move || {
        let target = source();
        if !enabled {
            return;
        }
        let _ = motion.animate_to(
            target as f32,
            dioxus_motion::prelude::AnimationConfig::spring(spring),
        );
    });
    use dioxus::prelude::ReadableExt;
    *motion.current().read()
}
