//! Small page-timer bridges for the loading surfaces.
//!
//! beUI drives `AgentProgress` and the `ReasoningText` phrase swap with
//! `setInterval`/`requestAnimationFrame` and React state. The port keeps the
//! clock in the page (a browser timer on web, the WebView's on desktop) and
//! mirrors the values into signals over the evaluator channel — the same pattern
//! `beui-core::hooks::dismiss` uses for its listeners.

use beui_core::platform;
use dioxus::prelude::*;

/// A signal that cycles `0..count` every `interval_ms`, starting at 0.
///
/// The interval is installed once on mount with the values it was given; a
/// changed count/interval does not restart it (deliberate).
pub fn use_interval_index(count: usize, interval_ms: u32) -> Signal<usize> {
    let mut index = use_signal(|| 0usize);
    use_effect(move || {
        if count == 0 {
            return;
        }
        let script = format!(
            "var i=0;dioxus.send(i);setInterval(function(){{i=(i+1)%{count};dioxus.send(i);}},{interval_ms});"
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(value) = eval.recv::<usize>().await {
                index.set(value);
            }
        });
    });
    index
}

/// A signal with the seconds elapsed since mount, advanced every 100 ms.
///
/// `initial` seeds the origin so a remount continues a run that was already in
/// flight; `running == false` installs nothing, mirroring the source's skipped
/// interval.
pub fn use_elapsed_seconds(initial: f64, running: bool) -> Signal<f64> {
    let mut seconds = use_signal(|| initial);
    use_effect(move || {
        if !running {
            return;
        }
        let script = format!(
            "var start=performance.now()-{initial}*1000;dioxus.send((performance.now()-start)/1000);setInterval(function(){{dioxus.send((performance.now()-start)/1000);}},100);",
            initial = initial.max(0.0),
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(value) = eval.recv::<f64>().await {
                seconds.set(value);
            }
        });
    });
    seconds
}

/// A signal with the observed `offsetHeight` of the element matching `selector`.
///
/// Installed once on mount: a `ResizeObserver` in the page reports every height
/// change (the source's `ResizeObserver` on the activity content), which is what
/// lets a streaming list keep its viewport height and glide offset current.
pub fn use_observed_height(selector: String) -> Signal<f64> {
    let mut height = use_signal(|| 0.0f64);
    use_effect(move || {
        let script = format!(
            "(function(){{var el=document.querySelector({sel});if(!el)return;\
             dioxus.send(el.offsetHeight);\
             if(el.__beuiRO&&el.__beuiRO.disconnect)el.__beuiRO.disconnect();\
             var ro=new ResizeObserver(function(){{dioxus.send(el.offsetHeight);}});\
             ro.observe(el);el.__beuiRO=ro;}})();",
            sel = platform::js_string(&selector),
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(value) = eval.recv::<f64>().await {
                height.set(value);
            }
        });
    });
    height
}
