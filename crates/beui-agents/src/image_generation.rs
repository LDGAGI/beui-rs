//! `agents/image-generation.tsx` — the generating-image surface.
//!
//! The media wrapper, status row, resolution badge and retry button keep the
//! source's markup and status-driven filter/opacity/scale. The dither field is a
//! `<canvas>` whose dot-displacement loop runs in the page through
//! `document::eval` (the algorithm is the source's, driven by
//! `requestAnimationFrame`); Rust only renders the canvas and toggles the
//! overlay opacity. Porting the loop to Rust would mean a per-frame wasm→DOM
//! boundary with no benefit, so it stays in JS.

use beui::prelude::*;

use crate::common::{use_unique_id, Icon};
use crate::icons::IconName;

const SPIN_KEYFRAMES: &str = "@keyframes beui-orb-spin{to{transform:rotate(360deg)}}";

/// `ImageGenerationStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImageGenerationStatus {
    Queued,
    #[default]
    Generating,
    Refining,
    Complete,
    Error,
}

impl ImageGenerationStatus {
    fn status_text(self) -> &'static str {
        match self {
            Self::Queued => "Waiting to generate",
            Self::Generating => "Generating image",
            Self::Refining => "Refining details",
            Self::Complete => "Image ready",
            Self::Error => "Generation failed",
        }
    }

    fn media_style(self, reduce: bool) -> String {
        let (filter, opacity, scale) = match self {
            Self::Queued => ("blur(4px) saturate(0.75)", 0.0, 1.02),
            Self::Generating => ("blur(3px) saturate(0.85)", 0.0, 1.015),
            Self::Refining => ("blur(1.5px) saturate(0.95)", 0.62, 1.005),
            Self::Complete => ("blur(0px) saturate(1)", 1.0, 1.0),
            Self::Error => ("blur(2px) saturate(0.5)", 0.28, 1.0),
        };
        if reduce {
            format!("opacity:{opacity};transition:opacity 0ms;")
        } else {
            format!(
                "filter:{filter};opacity:{opacity};transform:scale({scale});transition:filter 400ms cubic-bezier(0.16,1,0.3,1),opacity 400ms cubic-bezier(0.16,1,0.3,1),transform 400ms cubic-bezier(0.16,1,0.3,1);"
            )
        }
    }

    fn overlay_opacity(self) -> f64 {
        match self {
            Self::Refining => 0.48,
            Self::Complete | Self::Error => 0.0,
            _ => 1.0,
        }
    }

    const fn active(self) -> bool {
        matches!(self, Self::Queued | Self::Generating | Self::Refining)
    }
}

/// `DitherField` — the canvas overlay.
#[component]
fn DitherField(interactive: bool, reduce: bool, status: ImageGenerationStatus) -> Element {
    let canvas_id = use_unique_id("dither");
    let canvas_for_selector = canvas_id.clone();
    use_effect(move || {
        let script = format!(
            "(function(){{var canvas=document.querySelector({sel});if(!canvas)return;var ctx=canvas.getContext('2d');if(!ctx)return;\
             var interactive={interactive};var reduce={reduce};var canHover=window.matchMedia('(hover: hover)').matches;\
             var frame=0,width=0,height=0,dotColor='currentColor';var pointer={{x:0,y:0,targetX:0,targetY:0,inside:false}};var pointerEnabled=interactive&&canHover&&!reduce;\
             function resize(){{var rect=canvas.getBoundingClientRect();width=rect.width||canvas.clientWidth||208;height=rect.height||canvas.clientHeight||208;var dpr=Math.min(window.devicePixelRatio||1,2);canvas.width=Math.round(width*dpr);canvas.height=Math.round(height*dpr);ctx.setTransform(dpr,0,0,dpr,0,0);dotColor=window.getComputedStyle(canvas).color;pointer.x=width/2;pointer.y=height/2;pointer.targetX=pointer.x;pointer.targetY=pointer.y;}}\
             function draw(time){{if(!canvas.isConnected)return;ctx.clearRect(0,0,width,height);\
             if(!pointer.inside){{pointer.targetX=width/2+(reduce?0:Math.sin(time/1700)*width*0.12);pointer.targetY=height/2+(reduce?0:Math.cos(time/2100)*height*0.1);}}\
             var follow=reduce?1:(pointer.inside?0.16:0.045);pointer.x+=(pointer.targetX-pointer.x)*follow;pointer.y+=(pointer.targetY-pointer.y)*follow;\
             var radius=Math.min(width,height)*0.38;var columns=Math.ceil(width/10)+1;var rows=Math.ceil(height/10)+1;var offsetX=(width-(columns-1)*10)/2;var offsetY=(height-(rows-1)*10)/2;ctx.fillStyle=dotColor;\
             for(var row=0;row<rows;row++){{for(var column=0;column<columns;column++){{var ax=offsetX+column*10,ay=offsetY+row*10;var dx=ax-pointer.x,dy=ay-pointer.y;var dist=Math.hypot(dx,dy);var prox=Math.max(0,1-dist/radius);var infl=prox*prox*(3-2*prox);var disp=infl*infl*9;var dirx=dist>0?dx/dist:0,diry=dist>0?dy/dist:0;var x=ax+dirx*disp,y=ay+diry*disp;var r=0.65+infl*0.85;ctx.globalAlpha=0.17+infl*0.72;ctx.beginPath();ctx.arc(x,y,r,0,Math.PI*2);ctx.fill();}}}}\
             ctx.globalAlpha=1;if(!reduce)frame=requestAnimationFrame(draw);}}\
             function onMove(e){{if(!pointerEnabled)return;var rect=canvas.getBoundingClientRect();pointer.inside=true;pointer.targetX=e.clientX-rect.left;pointer.targetY=e.clientY-rect.top;}}\
             function onLeave(){{pointer.inside=false;}}\
             var ro=typeof ResizeObserver==='undefined'?null:new ResizeObserver(resize);resize();if(ro)ro.observe(canvas);\
             canvas.addEventListener('pointermove',onMove,{{passive:true}});canvas.addEventListener('pointerleave',onLeave);draw(0);}})();",
            sel = beui_core::platform::js_string(&format!("[data-beui-dither=\"{canvas_for_selector}\"]")),
            interactive = interactive,
            reduce = reduce,
        );
        beui_core::platform::fire(&script);
    });

    let opacity = status.overlay_opacity();
    let duration = if reduce { 0 } else { 400 };

    rsx! {
        div {
            "aria-hidden": "true",
            class: "absolute inset-0 overflow-hidden bg-muted",
            style: "opacity:{opacity};transition:opacity {duration}ms cubic-bezier(0.16,1,0.3,1);",
            canvas {
                "data-beui-dither": "{canvas_id}",
                class: "absolute inset-0 size-full text-foreground",
            }
        }
    }
}

/// `DitherMark`.
#[component]
fn DitherMark(status: ImageGenerationStatus, reduce: bool) -> Element {
    rsx! {
        if status == ImageGenerationStatus::Complete {
            Icon { name: IconName::Check, class: "size-3.5", "aria-hidden": "true" }
        } else if status == ImageGenerationStatus::Error {
            Icon { name: IconName::CircleAlert, class: "size-3.5", "aria-hidden": "true" }
        } else {
            span {
                "aria-hidden": "true",
                class: "grid size-3.5 grid-cols-2 place-items-center gap-0.5",
                style: if reduce { String::new() } else { "animation:beui-orb-spin 2.4s cubic-bezier(0.65,0,0.35,1) infinite;".to_string() },
                span { class: "size-1 rounded-[1px] bg-current" }
                span { class: "size-1 rounded-[1px] bg-current opacity-55" }
                span { class: "size-1 rounded-[1px] bg-current opacity-55" }
                span { class: "size-1 rounded-[1px] bg-current" }
            }
        }
    }
}

/// `ImageGeneration`.
#[component]
pub fn ImageGeneration(
    #[props(default)] children: Option<Element>,
    #[props(default)] status: ImageGenerationStatus,
    #[props(default)] label: Option<String>,
    #[props(default)] prompt: Option<String>,
    #[props(default = "1024 × 1024".to_string())] resolution: String,
    #[props(default = "1 / 1".to_string())] aspect_ratio: String,
    #[props(default = ImageGenerationSize::Compact)] size: ImageGenerationSize,
    #[props(default = true)] interactive: bool,
    #[props(default)] status_text: Option<String>,
    #[props(default = true)] show_status: bool,
    #[props(default)] on_retry: Option<EventHandler<()>>,
    #[props(default)] class: String,
    #[props(default)] media_class_name: String,
    #[props(default)] status_class_name: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let active = status.active();
    let resolved_status_text = status_text.unwrap_or_else(|| status.status_text().to_string());
    let resolved_label = label.unwrap_or_else(|| match &prompt {
        Some(prompt) => format!("{resolved_status_text}: {prompt}"),
        None => resolved_status_text.clone(),
    });
    let media_style = status.media_style(reduce);
    let has_children = children.is_some();
    let show_bottom = show_status || prompt.is_some();
    let retry = status == ImageGenerationStatus::Error && on_retry.is_some();

    rsx! {
        style { dangerous_inner_html: "{SPIN_KEYFRAMES}" }
        div {
            "data-slot": "image-generation",
            "data-state": match status {
                ImageGenerationStatus::Queued => "queued",
                ImageGenerationStatus::Generating => "generating",
                ImageGenerationStatus::Refining => "refining",
                ImageGenerationStatus::Complete => "complete",
                ImageGenerationStatus::Error => "error",
            },
            "aria-busy": if active { "true" } else { "false" },
            class: cn!("w-full", class),
            div {
                class: cn!("w-full", if size == ImageGenerationSize::Compact { "mx-auto max-w-52" } else { "" }),
                div {
                    role: "img",
                    "aria-label": "{resolved_label}",
                    style: "aspect-ratio:{aspect_ratio}",
                    class: "relative isolate w-full overflow-hidden rounded-xl bg-muted",
                    div {
                        "aria-hidden": if has_children { None } else { Some("true") },
                        class: cn!(
                            "absolute inset-0 [&>*]:size-full [&>*]:object-cover [&_img]:size-full [&_img]:object-cover",
                            media_class_name,
                        ),
                        style: "{media_style}",
                        if let Some(children) = children {
                            {children}
                        }
                    }

                    if active {
                        DitherField { interactive, reduce, status }
                    }

                    if !resolution.is_empty() {
                        span { class: "absolute top-2 right-2 z-10 rounded-full bg-background/75 px-2 py-0.5 font-mono text-[10px] tabular-nums text-muted-foreground",
                            "{resolution}"
                        }
                    }
                }

                if show_bottom {
                    div { class: "mt-3 text-left",
                        if show_status {
                            div {
                                "aria-live": "polite",
                                class: cn!(
                                    "flex min-h-5 items-center gap-2 text-sm font-medium text-foreground",
                                    if status == ImageGenerationStatus::Error { "text-destructive" } else { "" },
                                    status_class_name,
                                ),
                                DitherMark { status, reduce }
                                span { "{resolved_status_text}" }
                            }
                        }
                        if let Some(prompt) = prompt {
                            p { class: "mt-0.5 truncate text-xs text-muted-foreground", "“{prompt}”" }
                        }
                    }
                }

                if retry {
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            if let Some(handler) = on_retry {
                                handler.call(());
                            }
                        },
                        class: "mt-3 inline-flex min-h-10 items-center gap-2 rounded-full px-3 text-sm font-medium text-foreground outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.96]",
                        Icon { name: IconName::RotateCcw, class: "size-4", "aria-hidden": "true" }
                        "Try again"
                    }
                }
            }
        }
    }
}

/// `ImageGenerationProps["size"]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImageGenerationSize {
    #[default]
    Compact,
    Fluid,
}
