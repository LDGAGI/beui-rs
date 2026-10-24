// Measure a popover's trigger and its portalled panel, and keep sending while
// anything about their geometry moves.
//
// The source does this with floating-ui's `autoUpdate`; here it is one
// `ResizeObserver` plus a capture-phase scroll listener and a resize listener.
// The Rust side owns the geometry math (it has to, because the same math drives
// the goo clip every frame), so this only reports boxes: a `|`-separated
// `trigger.left|top|width|height|content.width|height`.
//
// `__beuiPopoverStart(id, triggerSel, contentSel)` installs the listeners and
// sends now; `__beuiPopoverStop(id)` removes them.
(function () {
  window.__beuiPopoverCleanup = window.__beuiPopoverCleanup || {};

  window.__beuiPopoverStart = function (id, triggerSel, contentSel) {
    window.__beuiPopoverStop(id);
    const trigger = document.querySelector(triggerSel);
    const content = document.querySelector(contentSel);
    const send = function () {
      const t = trigger ? trigger.getBoundingClientRect() : null;
      const message =
        (t ? t.left : 0) +
        "|" +
        (t ? t.top : 0) +
        "|" +
        (t ? t.width : 0) +
        "|" +
        (t ? t.height : 0) +
        "|" +
        (content ? content.offsetWidth : 0) +
        "|" +
        (content ? content.offsetHeight : 0) +
        "|" +
        window.innerWidth +
        "|" +
        window.innerHeight;
      dioxus.send(message);
    };
    const onScroll = function () {
      send();
    };
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onScroll);
    const observer = new ResizeObserver(send);
    if (trigger) observer.observe(trigger);
    if (content) observer.observe(content);
    send();
    window.__beuiPopoverCleanup[id] = function () {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onScroll);
      observer.disconnect();
    };
  };

  window.__beuiPopoverStop = function (id) {
    const cleanup = window.__beuiPopoverCleanup[id];
    if (cleanup) {
      cleanup();
      delete window.__beuiPopoverCleanup[id];
    }
  };
})();
