// The FLIP commit for `layout` / `layoutId`.
//
// framer-motion does this synchronously inside React's layout effect, between
// the DOM change and the paint: measure the new box, write a transform that
// keeps the element where it *was*, then animate that transform away. Rust
// cannot do either half synchronously on the desktop renderers, and on the web
// renderer it cannot write a style attribute between Dioxus's mutation and the
// paint, so the two DOM operations that must happen *now* happen here, in the
// same task as the effect that fires this script:
//
//   1. `getBoundingClientRect()` — the new layout box, with any transform still
//      in flight cleared first so the number is the layout box and not the
//      visual one.
//   2. `style.transform` — the pin, written immediately so the browser never
//      paints the element at its new box. The reactive `style` attribute
//      carries the same values on the next render, so nothing moves twice.
//
// Everything else — the delta arithmetic, the registry of shared-layout boxes,
// the curve, the interruption case — is in `flip.rs`.
//
// Placeholders substituted by `use_flip`: __BEUI_SELECTOR__, __BEUI_FROM__,
// __BEUI_SCALE__.

const element = document.querySelector(__BEUI_SELECTOR__);
const from = __BEUI_FROM__;
const applyScale = __BEUI_SCALE__;

if (!element) {
  dioxus.send(null);
} else {
  // Measure the layout box: clear the transform Rust is animating (it will be
  // rewritten this same task) so `getBoundingClientRect` reports where the
  // element *is*, not where it currently looks.
  element.style.transform = "";
  element.style.transformOrigin = "";
  const to = element.getBoundingClientRect();

  let dx = 0;
  let dy = 0;
  let sx = 1;
  let sy = 1;
  if (from && from.width > 0 && from.height > 0) {
    dx = from.left - to.left;
    dy = from.top - to.top;
    if (applyScale) {
      if (to.width > 0) sx = from.width / to.width;
      if (to.height > 0) sy = from.height / to.height;
    }
  }

  const moved =
    Math.abs(dx) > 0.5 ||
    Math.abs(dy) > 0.5 ||
    Math.abs(sx - 1) > 0.005 ||
    Math.abs(sy - 1) > 0.005;

  if (moved) {
    // A top-left origin makes `translate + scale` map the new box onto the old
    // one exactly, with no origin correction to get wrong. framer-motion keeps
    // the default origin and corrects the delta instead; the visual box ends up
    // the same either way.
    element.style.transformOrigin = "top left";
    element.style.transform =
      "translateX(" + dx + "px) translateY(" + dy + "px) scaleX(" + sx + ") scaleY(" + sy + ")";
  }

  dioxus.send({
    left: to.left,
    top: to.top,
    width: to.width,
    height: to.height,
    dx: dx,
    dy: dy,
    sx: sx,
    sy: sy,
    moved: moved,
  });
}
