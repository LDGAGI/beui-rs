//! A mounted element's on-screen box, cached in a signal.
//!
//! `Magnetic` and `TiltCard` both need `element.getBoundingClientRect()` on
//! every pointer move. Reading it per move would mean an `await` per event;
//! instead the box is measured on mount — and again when the pointer enters the
//! element, which is the moment a box stale from a scroll or a layout change
//! would start to show. The move handler itself only reads the cache.

use crate::prelude::*;
use beui_core::platform::{client_rect, js_string, json, next_scope_id, Rect};

/// A cached `getBoundingClientRect` for one element.
#[derive(Clone, Copy)]
pub struct ElementRect {
    id: u32,
    rect: Signal<Option<Rect>>,
}

/// Create an [`ElementRect`]; put [`ElementRect::attr`] on the element and wire
/// [`ElementRect::measure`] to `onmounted`, [`ElementRect::refresh`] to
/// `onmouseenter`, and read [`ElementRect::get`] inside the move handler.
pub fn use_element_rect() -> ElementRect {
    ElementRect {
        id: use_hook(next_scope_id),
        rect: use_signal(|| None),
    }
}

impl ElementRect {
    /// The value for the element's `data-beui-rect` attribute.
    pub fn attr(&self) -> String {
        self.id.to_string()
    }

    /// Record the element's box. Takes the `Rc` the event hands out, because
    /// the measurement has to be awaited and `MountedData` is not cloneable.
    pub fn measure(&self, data: std::rc::Rc<MountedData>) {
        let mut signal = self.rect;
        spawn(async move {
            if let Some(rect) = client_rect(&data).await {
                signal.set(Some(rect));
            }
        });
    }

    /// Re-measure through a script, for the events (`mouseenter`) that carry no
    /// `MountedData`.
    pub fn refresh(&self) {
        let mut signal = self.rect;
        let selector = format!("[data-beui-rect=\"{}\"]", self.id);
        spawn(async move {
            let script = format!(
                "const el = document.querySelector({});\n\
                 if (!el) return null;\n\
                 const r = el.getBoundingClientRect();\n\
                 return {{ left: r.left, top: r.top, width: r.width, height: r.height }};",
                js_string(&selector)
            );
            if let Some(rect) = json::<Rect>(&script).await {
                signal.set(Some(rect));
            }
        });
    }

    /// The last measured box, if any.
    pub fn get(&self) -> Option<Rect> {
        (self.rect)()
    }
}

/// A live content width, streamed from a `ResizeObserver`.
#[derive(Clone, Copy)]
pub struct ElementWidth {
    id: u32,
    width: Signal<f64>,
}

/// Observe one element's width.
///
/// The masonry layout needs a number that changes with the container, and
/// `measure` on mount only reports one frame of it. The observer streams every
/// resize through the same `document::eval` channel the scroll and in-view
/// helpers use; put [`ElementWidth::attr`] on the element and read
/// [`ElementWidth::width`].
pub fn use_element_width() -> ElementWidth {
    let id = use_hook(next_scope_id);
    let mut width = use_signal(|| 0.0f64);
    let selector = format!("[data-beui-width=\"{id}\"]");
    let script = format!(
        "const el = document.querySelector({selector});\n\
         if (!el) return;\n\
         const send = () => dioxus.send(el.getBoundingClientRect().width);\n\
         send();\n\
         const observer = new ResizeObserver(send);\n\
         observer.observe(el);\n\
         for (;;) {{ if ((await dioxus.recv()) === \"close\") break; }}\n\
         observer.disconnect();",
        selector = js_string(&selector),
    );

    use_effect(move || {
        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(next) = eval.recv::<f64>().await {
                width.set(next);
            }
        });
    });

    ElementWidth { id, width }
}

impl ElementWidth {
    /// The value for the element's `data-beui-width` attribute.
    pub fn attr(&self) -> String {
        self.id.to_string()
    }

    /// The last observed width in px.
    pub fn width(&self) -> f64 {
        (self.width)()
    }
}

/// A live content size, streamed from a `ResizeObserver`.
#[derive(Clone, Copy)]
pub struct ElementSize {
    id: u32,
    size: Signal<(f64, f64)>,
}

/// Observe one element's width and height.
///
/// [`use_element_width`] covers the common case; this is for surfaces whose
/// *height* is the measurement that matters — a panel that grows to fit what it
/// holds, for instance.
pub fn use_element_size() -> ElementSize {
    let id = use_hook(next_scope_id);
    let mut size = use_signal(|| (0.0f64, 0.0f64));
    let selector = format!("[data-beui-size=\"{id}\"]");
    let script = format!(
        "const el = document.querySelector({selector});\n\
         if (!el) return;\n\
         const send = () => dioxus.send([el.offsetWidth, el.offsetHeight]);\n\
         send();\n\
         const observer = new ResizeObserver(send);\n\
         observer.observe(el);\n\
         for (;;) {{ if ((await dioxus.recv()) === \"close\") break; }}\n\
         observer.disconnect();",
        selector = js_string(&selector),
    );

    use_effect(move || {
        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(next) = eval.recv::<Vec<f64>>().await {
                if next.len() >= 2 {
                    size.set((next[0], next[1]));
                }
            }
        });
    });

    ElementSize { id, size }
}

impl ElementSize {
    /// The value for the element's `data-beui-size` attribute.
    pub fn attr(&self) -> String {
        self.id.to_string()
    }

    /// The last observed `(width, height)` in px.
    pub fn size(&self) -> (f64, f64) {
        (self.size)()
    }
}
