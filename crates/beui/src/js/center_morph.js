// Focus handling for the centre-unfolding modal: initial focus, a Tab trap,
// Escape, and focus restoration to the trigger.
//
// `__beuiCenterMorphStart(id, contentId, triggerId, dismissible)` installs one
// `keydown` listener and focuses the first focusable control (or the panel);
// `__beuiCenterMorphStop(id)` removes it and hands focus back to the trigger.
// The module and the start call share one eval, so `dioxus.send` is the channel
// the Rust loop reads.
(function () {
  window.__beuiCenterMorphCleanup = window.__beuiCenterMorphCleanup || {};
  const FOCUSABLE =
    'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])';

  window.__beuiCenterMorphStart = function (id, contentId, triggerId, dismissible) {
    window.__beuiCenterMorphStop(id);
    const panel = document.getElementById(contentId);
    const focusables = function () {
      if (!panel) return [];
      return Array.from(panel.querySelectorAll(FOCUSABLE)).filter(function (element) {
        return element.tabIndex >= 0;
      });
    };
    const frame = requestAnimationFrame(function () {
      const list = focusables();
      (list[0] || panel || {}).focus?.();
    });
    const onKey = function (event) {
      if (event.key === "Escape" && dismissible) {
        event.preventDefault();
        dioxus.send("escape");
        return;
      }
      if (event.key !== "Tab") return;
      const list = focusables();
      if (list.length === 0) {
        event.preventDefault();
        panel?.focus();
        return;
      }
      const first = list[0];
      const last = list[list.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    window.__beuiCenterMorphCleanup[id] = function () {
      cancelAnimationFrame(frame);
      window.removeEventListener("keydown", onKey);
      const trigger = document.getElementById(triggerId);
      if (trigger && typeof trigger.focus === "function") trigger.focus();
    };
  };

  window.__beuiCenterMorphStop = function (id) {
    const cleanup = window.__beuiCenterMorphCleanup[id];
    if (cleanup) {
      cleanup();
      delete window.__beuiCenterMorphCleanup[id];
    }
  };
})();
