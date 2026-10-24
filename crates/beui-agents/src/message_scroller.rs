//! `agents/message-scroller.tsx` — the transcript viewport that follows
//! streamed output.
//!
//! The source drives everything from a `ResizeObserver` on the content plus a
//! scroll listener, with a programmatic-scroll guard so a follow-scroll is not
//! mistaken for the reader leaving the live edge. That logic lives in the page
//! here (installed through `document::eval`, like `beui-core::dismiss`) and
//! reports the follow state back over the evaluator channel; Rust only mirrors it
//! for `on_follow_change`.
//!
//! `navigation="rail"` is not ported: the rail is the motion crate's
//! `PreviewRail`, which is out of this lane. The prop is accepted and ignored so
//! call sites compile unchanged .

use beui::prelude::*;

use crate::common::use_unique_id;

/// `MessageScroller`.
#[component]
pub fn MessageScroller(
    #[props(default = true)] follow_output: bool,
    #[props(default = 56)] follow_threshold: i32,
    #[props(default = true)] smooth: bool,
    #[props(default)] on_follow_change: Option<EventHandler<bool>>,
    #[props(default = "Conversation".to_string())] label: String,
    #[props(default)] busy: bool,
    #[props(default)] viewport_class_name: String,
    #[props(default)] content_class_name: String,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let viewport_id = use_unique_id("beui-scroller");
    let selector = format!("[data-beui-scroller-vp=\"{viewport_id}\"]");

    use_effect(move || {
        let script = format!(
            "(function(){{var vp=document.querySelector({selector});if(!vp)return;\
             if(vp.__beuiScroller&&vp.__beuiScroller.disconnect)vp.__beuiScroller.disconnect();\
             var content=vp.firstElementChild;var threshold={threshold};var follow={follow};\
             var smooth={smooth};var programmatic=false;\
             function scrollEnd(behavior){{programmatic=true;try{{vp.scrollTo({{top:vp.scrollHeight,behavior:behavior}});}}catch(e){{vp.scrollTop=vp.scrollHeight;}}\
             if(vp.__beuiTimer)clearTimeout(vp.__beuiTimer);vp.__beuiTimer=setTimeout(function(){{programmatic=false;}},behavior==='smooth'?320:0);}}\
             if(follow)scrollEnd('auto');\
             var ro=new ResizeObserver(function(){{if(follow)scrollEnd(smooth?'smooth':'auto');}});\
             if(content)ro.observe(content);vp.__beuiScroller=ro;\
             var reported=false;\
             function report(next){{if(!reported){{reported=true;dioxus.send(next);}}else if(next!==follow){{dioxus.send(next);}}follow=next;}}\
             vp.addEventListener('scroll',function(){{if(programmatic)return;\
             var d=vp.scrollHeight-vp.scrollTop-vp.clientHeight;report(d<=threshold);}});\
             vp.addEventListener('wheel',function(){{programmatic=false;}},{{passive:true}});\
             vp.addEventListener('touchstart',function(){{programmatic=false;}},{{passive:true}});\
             }})();",
            selector = beui_core::platform::js_string(&selector),
            threshold = follow_threshold,
            follow = follow_output,
            smooth = smooth,
        );

        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(value) = eval.recv::<bool>().await {
                if let Some(handler) = on_follow_change {
                    handler.call(value);
                }
            }
        });
    });

    rsx! {
        div {
            "data-slot": "message-scroller",
            class: cn!("min-h-0", class),
            ..attributes,
            section {
                "data-beui-scroller-vp": "{viewport_id}",
                "aria-label": "{label}",
                class: cn!(
                    "h-full overflow-y-auto overscroll-contain outline-none [overflow-anchor:none] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring [scrollbar-gutter:stable]",
                    viewport_class_name,
                ),
                div {
                    role: "log",
                    "aria-live": "polite",
                    "aria-relevant": "additions text",
                    "aria-busy": if busy { "true" } else { "false" },
                    class: content_class_name,
                    {children}
                }
            }
        }
    }
}
