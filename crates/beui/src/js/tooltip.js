// Tooltip positioning: the floating-ui middleware the source uses, reduced to
// the set it actually configures — offset, flip, shift, arrow, hide — at a
// `fixed` strategy, plus an auto-update loop.
//
// The Rust side renders the geometry-free parts (surface, arrow path); this
// script measures, picks the side, writes the transform, the arrow's anchor and
// its cross-axis offset, and reports the placed side back over `dioxus.send` so
// the arrow path can match. `__beuiTooltipStop(id)` tears it down.
//
// The module and the start call share one eval so `dioxus.send` is the channel
// the Rust loop reads.
(function () {
  window.__beuiTooltipCleanup = window.__beuiTooltipCleanup || {};
  const OPPOSITE = { top: "bottom", bottom: "top", left: "right", right: "left" };

  window.__beuiTooltipStart = function (id, anchorSel, floatingSel, opts) {
    window.__beuiTooltipStop(id);
    const anchor = document.querySelector(anchorSel);
    const floating = document.querySelector(floatingSel);
    if (!anchor || !floating) return;
    const arrow = opts.arrow ? floating.querySelector("[data-tooltip-arrow]") : null;
    const padding = 8;
    const arrowPadding = 10;

    const compute = function () {
      const a = anchor.getBoundingClientRect();
      const f = { width: floating.offsetWidth, height: floating.offsetHeight };
      const space = {
        top: a.top,
        bottom: window.innerHeight - a.bottom,
        left: a.left,
        right: window.innerWidth - a.right,
      };
      const neededMain = opts.side === "top" || opts.side === "bottom"
        ? f.height + opts.offset
        : f.width + opts.offset;
      let side = opts.side;
      const opposite = OPPOSITE[side];
      if (space[side] < neededMain + padding && space[opposite] > space[side]) {
        side = opposite;
      }
      const along = function (start, size, floatingSize) {
        if (opts.align === "start") return start;
        if (opts.align === "end") return start + size - floatingSize;
        return start + (size - floatingSize) / 2;
      };
      let x;
      let y;
      if (side === "top" || side === "bottom") {
        x = along(a.left, a.width, f.width);
        y = side === "top" ? a.top - f.height - opts.offset : a.bottom + opts.offset;
      } else {
        y = along(a.top, a.height, f.height);
        x = side === "left" ? a.left - f.width - opts.offset : a.right + opts.offset;
      }
      if (side === "top" || side === "bottom") {
        x = Math.max(padding, Math.min(x, window.innerWidth - f.width - padding));
      } else {
        y = Math.max(padding, Math.min(y, window.innerHeight - f.height - padding));
      }

      const dpr = window.devicePixelRatio || 1;
      const px = function (value) {
        return Math.round(value * dpr) / dpr;
      };
      floating.style.transform =
        "translate3d(" + px(x) + "px, " + px(y) + "px, 0)";
      floating.style.setProperty("--tooltip-origin", opts.origin[side]);
      floating.dataset.placement = side;

      const clipped =
        (a.width > 0 || a.height > 0) &&
        (a.bottom < 0 || a.top > window.innerHeight || a.right < 0 || a.left > window.innerWidth);
      floating.style.visibility = clipped ? "hidden" : "visible";

      if (arrow) {
        if (side === "top" || side === "bottom") {
          const center = a.left + a.width / 2 - x - 6;
          const offset = Math.max(arrowPadding, Math.min(center, f.width - arrowPadding - 12));
          arrow.style.left = offset + "px";
          arrow.style.right = "";
          arrow.style.top = side === "top" ? "calc(100% - 6px)" : "";
          arrow.style.bottom = side === "bottom" ? "calc(100% - 6px)" : "";
        } else {
          const center = a.top + a.height / 2 - y - 6;
          const offset = Math.max(arrowPadding, Math.min(center, f.height - arrowPadding - 12));
          arrow.style.top = offset + "px";
          arrow.style.bottom = "";
          arrow.style.left = side === "left" ? "calc(100% - 6px)" : "";
          arrow.style.right = side === "right" ? "calc(100% - 6px)" : "";
        }
        const surface = arrow.parentElement;
        if (surface) {
          const style = getComputedStyle(surface);
          arrow.style.setProperty("--tooltip-arrow-fill", style.backgroundColor);
          arrow.style.setProperty("--tooltip-arrow-stroke", style.borderTopColor);
        }
      }
      dioxus.send("placement:" + side);
    };

    let frame = 0;
    const schedule = function () {
      if (frame) return;
      frame = requestAnimationFrame(function () {
        frame = 0;
        compute();
      });
    };
    const observer = new ResizeObserver(schedule);
    observer.observe(anchor);
    observer.observe(floating);
    window.addEventListener("scroll", schedule, true);
    window.addEventListener("resize", schedule);
    window.__beuiTooltipCleanup[id] = function () {
      observer.disconnect();
      window.removeEventListener("scroll", schedule, true);
      window.removeEventListener("resize", schedule);
      if (frame) cancelAnimationFrame(frame);
    };
    schedule();
  };

  window.__beuiTooltipStop = function (id) {
    const cleanup = window.__beuiTooltipCleanup[id];
    if (cleanup) {
      cleanup();
      delete window.__beuiTooltipCleanup[id];
    }
  };
})();
