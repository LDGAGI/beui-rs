// The command palette's global shortcut and Escape.
//
// `__beuiCommandStart(id, shortcut)` installs one `keydown` listener that sends
// `"toggle"` for Cmd/Ctrl+`shortcut` and `"escape"` for Escape; Rust decides
// what each means from its own open state, so the listener never needs
// rebinding when the palette opens or closes. `__beuiCommandStop(id)` removes
// it. The module and the start call share one eval, so `dioxus.send` is the
// channel the Rust loop reads.
(function () {
  window.__beuiCommandCleanup = window.__beuiCommandCleanup || {};

  window.__beuiCommandStart = function (id, shortcut) {
    window.__beuiCommandStop(id);
    const onKey = function (event) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === shortcut.toLowerCase()) {
        event.preventDefault();
        dioxus.send("toggle");
        return;
      }
      if (event.key === "Escape") {
        dioxus.send("escape");
      }
    };
    window.addEventListener("keydown", onKey);
    window.__beuiCommandCleanup[id] = function () {
      window.removeEventListener("keydown", onKey);
    };
  };

  window.__beuiCommandStop = function (id) {
    const cleanup = window.__beuiCommandCleanup[id];
    if (cleanup) {
      cleanup();
      delete window.__beuiCommandCleanup[id];
    }
  };
})();
