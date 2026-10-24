// The label clip mask for the pill/segment variants.
//
// beUI draws the active tab's label twice: the button paints it in
// `text-muted-foreground`, and an `inert` overlay paints the *same* label in
// `text-primary-foreground`, clipped to the indicator's rectangle. As the
// indicator springs between tabs, the clip is the only thing that moves, so the
// text appears to hand its colour over at the indicator's edge.
//
// motion/react drives this from `frame.postRender`, reading the projected pill
// rectangle every frame. The port does the same with `requestAnimationFrame`:
// the flip pin is written synchronously, so `getBoundingClientRect` already
// reports the animated position. The loop stops the same way — two still frames
// after a couple of settling frames — so it never spins.
(function () {
  window.__beuiTabsClip = function (list, reduce) {
    if (!list) return;
    if (list.__beuiTabsClipFrame) {
      cancelAnimationFrame(list.__beuiTabsClipFrame);
      list.__beuiTabsClipFrame = 0;
    }
    const labels = Array.from(list.querySelectorAll("[data-tabs-label]"));
    const indicator = list.querySelector("[data-tabs-indicator]");
    const target = list.querySelector('[role="tab"][aria-selected="true"]');
    if (!indicator || !target) {
      for (const label of labels) label.style.clipPath = "inset(0 100% 0 0)";
      return;
    }
    let frames = 0;
    let stillFrames = 0;
    let previous;
    const sync = function () {
      // One read pass for every rectangle before the write pass; interleaving
      // them would force a style flush per label.
      const pill = (reduce ? target : indicator).getBoundingClientRect();
      const clips = labels.map(function (label) {
        const bounds = label.getBoundingClientRect();
        const left = Math.max(0, Math.min(bounds.width, pill.left - bounds.left));
        const right = Math.max(0, Math.min(bounds.width, bounds.right - pill.right));
        return left + right >= bounds.width
          ? "inset(0 100% 0 0)"
          : "inset(0 " + right + "px 0 " + left + "px)";
      });
      labels.forEach(function (label, index) {
        if (label.style.clipPath !== clips[index]) label.style.clipPath = clips[index];
      });
      frames += 1;
      stillFrames =
        previous &&
        Math.abs(pill.left - previous.left) < 0.01 &&
        Math.abs(pill.right - previous.right) < 0.01
          ? stillFrames + 1
          : 0;
      previous = { left: pill.left, right: pill.right };
      if (reduce || (frames > 2 && stillFrames >= 2)) {
        list.__beuiTabsClipFrame = 0;
        return;
      }
      list.__beuiTabsClipFrame = requestAnimationFrame(sync);
    };
    list.__beuiTabsClipFrame = requestAnimationFrame(sync);
  };
})();
