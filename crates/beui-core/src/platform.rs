//! The one place where renderers differ.
//!
//! The web renderer owns the DOM: `web-sys` is available and every read is
//! synchronous. The desktop and mobile renderers put the same DOM inside a
//! WebView that the Rust side cannot reach — `web-sys` compiles there but
//! panics ("cannot access imported statics on non-wasm targets") the moment it
//! is called. What those renderers do have is [`dioxus::document::eval`].
//!
//! So: synchronous reads are `Some` on web and `None` elsewhere, and every
//! caller that needs a value on desktop uses the eval-based path. [`fire`] and
//! [`json`] work on every renderer; they are the portable fallback.
//!
//! Everything below is deliberately fire-and-forget or one-shot. Per-frame DOM
//! writes must not go through here — those belong in the reactive `style`
//! attribute so the renderer can apply them natively.

use dioxus::document;
use dioxus::prelude::{MountedData, PointerData};
use serde::de::DeserializeOwned;

/// A handle to a running JavaScript task, or `None` outside a Dioxus runtime.
pub fn eval(script: &str) -> Option<document::Eval> {
    dioxus_core::Runtime::try_current()?;
    Some(document::eval(script))
}

/// Run `script` and ignore the result.
pub fn fire(script: &str) {
    let _ = eval(script);
}

/// Run `script`, expecting it to `return` a JSON value.
pub async fn json<T: DeserializeOwned>(script: &str) -> Option<T> {
    let eval = eval(script)?;
    eval.join::<T>().await.ok()
}

/// Escape a Rust string for interpolation into a single-quoted JS string.
pub fn js_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

/// `matchMedia(query).matches`, synchronously. `None` off the web renderer.
#[cfg(target_arch = "wasm32")]
pub fn media_matches(query: &str) -> Option<bool> {
    let window = web_sys::window()?;
    window
        .match_media(query)
        .ok()
        .flatten()
        .map(|mq| mq.matches())
}

/// `matchMedia(query).matches`, synchronously. `None` off the web renderer.
#[cfg(not(target_arch = "wasm32"))]
pub fn media_matches(_query: &str) -> Option<bool> {
    None
}

/// A monotonic clock in milliseconds, for velocity sampling.
///
/// `std::time::Instant` on wasm32-unknown-unknown imports `env.now`, which the
/// wasm-bindgen output cannot resolve — so this goes through the same `instant`
/// crate `dioxus-motion` uses, where the backend (and therefore whether it is a
/// JS clock at all) is a feature of the target. Only differences are used, so
/// the origin is arbitrary and never leaves this function.
pub fn now_ms() -> f64 {
    // On the web renderer take `performance.now()`, which motion uses: it is
    // sub-millisecond, and a flick delivered inside one millisecond would
    // otherwise measure a frame delta of zero and therefore a velocity of zero.
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Some(performance) = window.performance() {
            return performance.now();
        }
    }
    use instant::Instant;
    thread_local! {
        static ORIGIN: Instant = Instant::now();
    }
    ORIGIN.with(|origin| origin.elapsed().as_secs_f64() * 1000.0)
}

/// The next id for an overlay scope.
///
/// Scopes are addressed by `data-*` attribute from scripts, because that is the
/// only handle the WebView renderers have on an element. One counter for all of
/// them keeps the ids short and the attribute values unambiguous.
pub fn next_scope_id() -> u32 {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// A `getBoundingClientRect`-shaped box in CSS pixels, relative to the viewport.
///
/// `Deserialize` is what lets a script return one straight over the evaluator
/// channel, which is the only measurement route the WebView renderers have.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// The ratio of `x` along the box's width, clamped to `[0, 1]`.
    ///
    /// This is `(clientX - rect.left) / rect.width` from the source, and it is
    /// how every pointer-driven slider here turns a position into a value.
    pub fn ratio_x(&self, x: f64) -> f64 {
        if self.width == 0.0 {
            return 0.0;
        }
        ((x - self.left) / self.width).clamp(0.0, 1.0)
    }
}

/// Measure a mounted element — the cross-renderer `getBoundingClientRect`.
///
/// Dioxus carries this itself: [`MountedData`] is implemented by every renderer
/// (web answers synchronously, the WebView renderers round-trip to their
/// interpreter), so this is one of the few DOM reads that needs no `cfg`.
pub async fn client_rect(data: &MountedData) -> Option<Rect> {
    let rect = data.get_client_rect().await.ok()?;
    Some(Rect {
        left: rect.origin.x,
        top: rect.origin.y,
        width: rect.size.width,
        height: rect.size.height,
    })
}

/// `setPointerCapture` on the node behind a pointer event, best effort.
///
/// `true` when the capture was taken; `false` off the web renderer, where the
/// bundle has no synchronous handle on the node — callers fall back to
/// [`capture_pointer_selector`].
///
/// The capture goes on the event's *target* rather than the element holding the
/// listener. The browser retargets the pointer to the captured node and the
/// events still bubble to the listener, so the gesture survives a drag that
/// leaves the element either way.
#[cfg(target_arch = "wasm32")]
pub fn capture_pointer(event: &PointerData) -> bool {
    use dioxus_web::WebEventExt;
    use wasm_bindgen::JsCast;

    let Some(raw) = event.try_as_web_event() else {
        return false;
    };
    let Some(target) = raw
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return false;
    };
    target.set_pointer_capture(event.pointer_id()).is_ok()
}

/// `setPointerCapture` on the node behind a pointer event, best effort.
#[cfg(not(target_arch = "wasm32"))]
pub fn capture_pointer(_event: &PointerData) -> bool {
    false
}

/// `releasePointerCapture`, best effort. `false` when there was nothing to do.
#[cfg(target_arch = "wasm32")]
pub fn release_pointer(event: &PointerData) -> bool {
    use dioxus_web::WebEventExt;
    use wasm_bindgen::JsCast;

    let Some(raw) = event.try_as_web_event() else {
        return false;
    };
    let Some(target) = raw
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return false;
    };
    if target.has_pointer_capture(event.pointer_id()) {
        target.release_pointer_capture(event.pointer_id()).is_ok()
    } else {
        false
    }
}

/// `releasePointerCapture`, best effort. `false` when there was nothing to do.
#[cfg(not(target_arch = "wasm32"))]
pub fn release_pointer(_event: &PointerData) -> bool {
    false
}

/// Capture the pointer on an element found by `selector`.
///
/// The non-web fallback for [`capture_pointer`], with the same best-effort
/// try/catch the source uses: WebKit throws `NotFoundError` when the pointer is
/// already gone by the time the handler runs, which is routine on iOS, and an
/// uncaught throw takes the rest of the gesture down with it.
pub fn capture_pointer_selector(selector: &str, pointer_id: i32) {
    fire(&format!(
        "try {{ const el = document.querySelector({}); if (el) el.setPointerCapture({pointer_id}); }} catch (e) {{}}",
        js_string(selector)
    ));
}

/// Release a capture taken with [`capture_pointer_selector`].
pub fn release_pointer_selector(selector: &str, pointer_id: i32) {
    fire(&format!(
        "try {{ const el = document.querySelector({}); if (el && el.hasPointerCapture({pointer_id})) el.releasePointerCapture({pointer_id}); }} catch (e) {{}}",
        js_string(selector)
    ));
}

/// Move keyboard focus onto the first element matching `selector`.
///
/// `prevent_scroll` matches the source's `focus({ preventScroll: true })`, which
/// keeps a click on a slider track from scrolling the page to the handle.
pub fn focus_selector(selector: &str, prevent_scroll: bool) {
    fire(&format!(
        "try {{ const el = document.querySelector({}); if (el) el.focus({{ preventScroll: {} }}); }} catch (e) {{}}",
        js_string(selector),
        if prevent_scroll { "true" } else { "false" }
    ));
}

/// `localStorage.getItem(key)`. `None` off the web renderer or when unset.
#[cfg(target_arch = "wasm32")]
pub fn storage_get(key: &str) -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok().flatten()?;
    storage.get_item(key).ok().flatten()
}

/// `localStorage.getItem(key)`. `None` off the web renderer or when unset.
#[cfg(not(target_arch = "wasm32"))]
pub fn storage_get(_key: &str) -> Option<String> {
    None
}

/// `localStorage.setItem(key, value)`; `null` removes.
pub fn storage_set(key: &str, value: Option<&str>) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                match value {
                    Some(value) => {
                        let _ = storage.set_item(key, value);
                    }
                    None => {
                        let _ = storage.remove_item(key);
                    }
                }
                return;
            }
        }
    }
    match value {
        Some(value) => fire(&format!(
            "try {{ localStorage.setItem({}, {}) }} catch (e) {{}}",
            js_string(key),
            js_string(value)
        )),
        None => fire(&format!(
            "try {{ localStorage.removeItem({}) }} catch (e) {{}}",
            js_string(key)
        )),
    }
}

/// Add or remove a class on `<html>`, matching `next-themes`' class strategy.
pub fn html_class(class: &str, on: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Some(element) = window.document().and_then(|d| d.document_element()) {
                let list = element.class_list();
                if on {
                    let _ = list.add_1(class);
                } else {
                    let _ = list.remove_1(class);
                }
                return;
            }
        }
    }
    fire(&format!(
        "document.documentElement.classList.{}('{}')",
        if on { "add" } else { "remove" },
        class
    ));
}

/// Set (or with `None` remove) an attribute on `<html>`.
pub fn html_attr(name: &str, value: Option<&str>) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Some(element) = window.document().and_then(|d| d.document_element()) {
                match value {
                    Some(value) => {
                        let _ = element.set_attribute(name, value);
                    }
                    None => {
                        let _ = element.remove_attribute(name);
                    }
                }
                return;
            }
        }
    }
    match value {
        Some(value) => fire(&format!(
            "document.documentElement.setAttribute({}, {})",
            js_string(name),
            js_string(value)
        )),
        None => fire(&format!(
            "document.documentElement.removeAttribute({})",
            js_string(name)
        )),
    }
}
