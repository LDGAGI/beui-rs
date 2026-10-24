// The imperative half of `morphing-tabs`: a spring-driven "liquid" SVG surface
// whose notch follows the active (or dragged) tab, plus the pointer drag that
// reorders the tabs.
//
// The source runs all of this through motion values, which are per-frame writes
// on the DOM; the port keeps the same shape by owning the springs and the path
// here and leaving Rust to render the static DOM and own the value/order. The
// module and the init call share one eval so `dioxus.send` is the channel the
// Rust loop reads; `order:...` commits a reorder.
(function () {
  const MAX_TAB_WIDTH = 176;
  const MIN_TAB_WIDTH = 96;
  const TAB_HEIGHT = 56;
  const TAB_TOP = 24;
  const TAB_RADIUS = 24;
  const RAIL_HEIGHT = 80;
  const SURFACE_INSET = 16;
  const LIQUID_JOIN = 24;
  const PANEL_RADIUS = 28;
  const DRAG_THRESHOLD = 5;

  // SPRING_GLIDE: stiffness 700, damping 50, mass 0.5.
  const K = 700;
  const C = 50;
  const M = 0.5;

  const states = (window.__beuiMorphTabsStates = window.__beuiMorphTabsStates || {});

  function liquidTabPath(tabLeft, surfaceWidth, tabWidth) {
    const panelLeft = SURFACE_INSET;
    const panelRight = surfaceWidth - SURFACE_INSET;
    const left = Math.max(panelLeft, Math.min(panelRight - tabWidth, tabLeft));
    const right = left + tabWidth;
    const top = RAIL_HEIGHT - TAB_HEIGHT;
    const bottom = RAIL_HEIGHT;
    const leftJoin = Math.max(panelLeft, left - LIQUID_JOIN);
    const rightJoin = Math.min(panelRight, right + LIQUID_JOIN);
    const leftDepth = Math.min(LIQUID_JOIN, left - leftJoin);
    const rightDepth = Math.min(LIQUID_JOIN, rightJoin - right);
    const leftControl = leftDepth * 0.55;
    const rightControl = rightDepth * 0.55;
    const leftPanelRadius = Math.min(PANEL_RADIUS, leftJoin - panelLeft);
    const rightPanelRadius = Math.min(PANEL_RADIUS, panelRight - rightJoin);
    return [
      "M" + panelLeft + " " + (bottom + PANEL_RADIUS),
      "V" + (bottom + leftPanelRadius),
      "Q" + panelLeft + " " + bottom + " " + (panelLeft + leftPanelRadius) + " " + bottom,
      "H" + leftJoin,
      "C" + (leftJoin + leftControl) + " " + bottom + " " + left + " " + (bottom - leftDepth + leftControl) + " " + left + " " + (bottom - leftDepth),
      "V" + (top + TAB_RADIUS),
      "Q" + left + " " + top + " " + (left + TAB_RADIUS) + " " + top,
      "H" + (right - TAB_RADIUS),
      "Q" + right + " " + top + " " + right + " " + (top + TAB_RADIUS),
      "V" + (bottom - rightDepth),
      "C" + right + " " + (bottom - rightDepth + rightControl) + " " + (rightJoin - rightControl) + " " + bottom + " " + rightJoin + " " + bottom,
      "H" + (panelRight - rightPanelRadius),
      "Q" + panelRight + " " + bottom + " " + panelRight + " " + (bottom + rightPanelRadius),
      "V" + (bottom + PANEL_RADIUS),
      "Z",
    ].join(" ");
  }

  function springStep(spring, target, dt) {
    const acceleration = (-K * (spring.x - target) - C * spring.v) / M;
    spring.v += acceleration * dt;
    spring.x += spring.v * dt;
  }

  function settled(spring, target) {
    return Math.abs(spring.x - target) < 0.5 && Math.abs(spring.v) < 10;
  }

  function measure(state) {
    const root = state.root;
    const rail = state.rail;
    if (!root || !rail) return;
    state.width = root.clientWidth;
    const gap = parseFloat(getComputedStyle(rail).columnGap);
    state.gap = Number.isFinite(gap) ? gap : 12;
    state.tabs = Array.from(root.querySelectorAll("[data-morph-tab]"));
    state.ids = state.tabs.map(function (tab) {
      return tab.getAttribute("data-morph-tab");
    });
    const count = state.ids.length;
    const inner = state.width - SURFACE_INSET * 2;
    const widthAt = function (g) {
      return Math.floor((inner - g * (count - 1)) / count);
    };
    if (!state.width || count === 0) {
      state.tabWidth = MAX_TAB_WIDTH;
      state.slotGap = state.gap;
    } else if (widthAt(state.gap) >= MIN_TAB_WIDTH) {
      state.tabWidth = Math.min(MAX_TAB_WIDTH, widthAt(state.gap));
      state.slotGap = state.gap;
    } else if (count > 1 && widthAt(0) >= MIN_TAB_WIDTH) {
      state.tabWidth = MIN_TAB_WIDTH;
      state.slotGap = Math.max(0, Math.floor((inner - MIN_TAB_WIDTH * count) / (count - 1)));
    } else {
      state.tabWidth = Math.max(0, widthAt(0));
      state.slotGap = 0;
    }
    state.slotLefts = state.ids.map(function (_, index) {
      return SURFACE_INSET + index * (state.tabWidth + state.slotGap);
    });
    if (state.host) {
      const svg = state.host.querySelector("svg");
      if (svg) svg.setAttribute("viewBox", "0 0 " + state.width + " 108");
    }
    state.tabs.forEach(function (tab, index) {
      const body = tab.querySelector("[data-morph-tab-body]");
      if (body) {
        body.style.width = state.tabWidth + "px";
        body.style.height = TAB_HEIGHT + "px";
        body.style.marginTop = TAB_TOP + "px";
      }
      const id = state.ids[index];
      if (!state.springs[id]) state.springs[id] = { x: state.slotLefts[index], v: 0 };
    });
  }

  function targetFor(state, index) {
    if (state.dragIndex < 0 || state.dragTargetIndex < 0) return index;
    if (index === state.dragIndex) return state.dragTargetIndex;
    if (state.dragTargetIndex > state.dragIndex && index > state.dragIndex && index <= state.dragTargetIndex) {
      return index - 1;
    }
    if (state.dragTargetIndex < state.dragIndex && index >= state.dragTargetIndex && index < state.dragIndex) {
      return index + 1;
    }
    return index;
  }

  function activeIndex(state) {
    const index = state.ids.indexOf(state.activeId);
    return index < 0 ? -1 : targetFor(state, index);
  }

  function render(state) {
    state.ids.forEach(function (id, index) {
      const spring = state.springs[id];
      if (!spring) return;
      const target = state.slotLefts[targetFor(state, index)] || SURFACE_INSET;
      if (state.draggingId === id) {
        spring.x = state.dragLeft;
        spring.v = 0;
      } else {
        springStep(spring, target, state.dt);
      }
      const tab = state.tabs[index];
      if (tab) tab.style.transform = "translate3d(" + spring.x + "px, 0, 0)";
    });
    const index = activeIndex(state);
    if (state.draggingId && state.draggingId === state.activeId) {
      state.surface.x = state.dragLeft;
      state.surface.v = 0;
    } else if (index >= 0) {
      springStep(state.surface, state.slotLefts[index] || SURFACE_INSET, state.dt);
    }
    if (state.path && state.tabWidth > 0) {
      state.path.setAttribute("d", liquidTabPath(state.surface.x, state.width, state.tabWidth));
    }
    if (state.host) state.host.style.zIndex = state.draggingId ? "20" : "0";
  }

  function allSettled(state) {
    if (state.draggingId) return false;
    const index = activeIndex(state);
    if (index >= 0 && !settled(state.surface, state.slotLefts[index] || SURFACE_INSET)) return false;
    return state.ids.every(function (id, i) {
      const spring = state.springs[id];
      return !spring || settled(spring, state.slotLefts[targetFor(state, i)] || SURFACE_INSET);
    });
  }

  function start(handle) {
    state = handle;
    if (handle.running) return;
    handle.running = true;
    handle.last = performance.now();
    handle.frame = requestAnimationFrame(function tick(now) {
      if (!handle.running) return;
      const dt = Math.min(1 / 30, Math.max(1 / 240, (now - handle.last) / 1000));
      handle.last = now;
      handle.dt = dt;
      render(handle);
      if (allSettled(handle)) {
        handle.running = false;
        return;
      }
      handle.frame = requestAnimationFrame(tick);
    });
  }

  function finishDrag(handle, pointerId) {
    const drag = handle.drag;
    if (!drag || drag.pointerId !== pointerId || drag.finishing) return;
    if (!drag.moved) {
      handle.drag = null;
      return;
    }
    drag.finishing = true;
    const targetLeft = handle.slotLefts[drag.targetIndex];
    const commit = function () {
      const next = drag.order.slice();
      const moved = next.splice(drag.startIndex, 1)[0];
      next.splice(drag.targetIndex, 0, moved);
      handle.draggingId = null;
      handle.dragIndex = -1;
      handle.dragTargetIndex = -1;
      handle.drag = null;
      if (next.join(",") !== drag.order.join(",")) {
        dioxus.send("order:" + next.join(","));
      }
      measure(handle);
      start(handle);
    };
    // Settle the dragged tab on its slot, then commit.
    const animate = function (now) {
      const dt = Math.min(1 / 30, Math.max(1 / 240, (now - (handle.lastDrag || now)) / 1000));
      handle.lastDrag = now;
      handle.dragLeft += (targetLeft - handle.dragLeft) * Math.min(1, dt * 18);
      render(handle);
      if (Math.abs(handle.dragLeft - targetLeft) < 0.5) {
        handle.dragLeft = targetLeft;
        commit();
        return;
      }
      requestAnimationFrame(animate);
    };
    requestAnimationFrame(animate);
  }

  window.__beuiMorphTabsInit = function (uid) {
    window.__beuiMorphTabsDestroy(uid);
    const root = document.querySelector('[data-morph-tabs="' + uid + '"]');
    if (!root) return;
    const handle = {
      uid,
      root,
      rail: root.querySelector("[data-morph-rail]"),
      host: root.querySelector("[data-morph-surface-host]"),
      path: root.querySelector("[data-morph-surface-path]"),
      tabs: [],
      ids: [],
      springs: {},
      surface: { x: SURFACE_INSET, v: 0 },
      slotLefts: [],
      tabWidth: MAX_TAB_WIDTH,
      slotGap: 12,
      gap: 12,
      width: 0,
      activeId: null,
      draggingId: null,
      dragIndex: -1,
      dragTargetIndex: -1,
      drag: null,
      dragLeft: SURFACE_INSET,
      dt: 1 / 60,
      running: false,
      frame: 0,
      last: 0,
    };
    states[uid] = handle;
    measure(handle);
    start(handle);

    const observer = new ResizeObserver(function () {
      measure(handle);
      start(handle);
    });
    observer.observe(root);
    handle.observer = observer;

    handle.onPointerDown = function (event) {
      const tab = event.target.closest("[data-morph-tab]");
      if (!tab || event.button !== 0) return;
      if (tab.getAttribute("data-disabled") === "true") return;
      if (handle.drag) return;
      const id = tab.getAttribute("data-morph-tab");
      const startIndex = handle.ids.indexOf(id);
      if (startIndex < 0) return;
      const slots = handle.ids.map(function (_, index) {
        return SURFACE_INSET + index * (handle.tabWidth + handle.slotGap);
      });
      handle.drag = {
        id,
        pointerId: event.pointerId,
        originX: event.clientX,
        startLeft: slots[startIndex],
        startIndex,
        targetIndex: startIndex,
        moved: false,
        finishing: false,
        order: handle.ids.slice(),
        slots,
      };
    };
    handle.onPointerMove = function (event) {
      const drag = handle.drag;
      if (!drag || drag.finishing || drag.pointerId !== event.pointerId) return;
      const delta = event.clientX - drag.originX;
      if (!drag.moved && Math.abs(delta) < DRAG_THRESHOLD) return;
      event.preventDefault();
      if (!drag.moved) {
        drag.moved = true;
        try {
          event.target.closest("[data-morph-tab]").setPointerCapture(event.pointerId);
        } catch (error) {
          void error;
        }
        handle.draggingId = drag.id;
        handle.dragIndex = drag.startIndex;
        handle.dragTargetIndex = drag.startIndex;
      }
      const minLeft = drag.slots[0];
      const maxLeft = drag.slots[drag.slots.length - 1];
      const visualLeft = Math.max(minLeft, Math.min(maxLeft, drag.startLeft + delta));
      let targetIndex = drag.startIndex;
      if (visualLeft >= drag.startLeft) {
        for (let index = drag.startIndex + 1; index < drag.slots.length; index += 1) {
          if (visualLeft + handle.tabWidth / 2 >= drag.slots[index]) targetIndex = index;
        }
      } else {
        for (let index = drag.startIndex - 1; index >= 0; index -= 1) {
          if (visualLeft <= drag.slots[index] + handle.tabWidth / 2) targetIndex = index;
        }
      }
      handle.dragLeft = visualLeft;
      if (targetIndex !== drag.targetIndex) {
        drag.targetIndex = targetIndex;
        handle.dragTargetIndex = targetIndex;
      }
      start(handle);
    };
    handle.onPointerUp = function (event) {
      finishDrag(handle, event.pointerId);
    };
    handle.onLostCapture = function (event) {
      if (event.target !== event.currentTarget) return;
      finishDrag(handle, event.pointerId);
    };

    root.addEventListener("pointerdown", handle.onPointerDown, true);
    root.addEventListener("pointermove", handle.onPointerMove, true);
    root.addEventListener("pointerup", handle.onPointerUp, true);
    root.addEventListener("pointercancel", handle.onPointerUp, true);
    root.addEventListener("lostpointercapture", handle.onLostCapture, true);
  };

  window.__beuiMorphTabsSetActive = function (uid, activeId) {
    const handle = states[uid];
    if (!handle) return;
    handle.activeId = activeId;
    start(handle);
  };

  window.__beuiMorphTabsMeasure = function (uid) {
    const handle = states[uid];
    if (!handle) return;
    measure(handle);
    start(handle);
  };

  window.__beuiMorphTabsDestroy = function (uid) {
    const handle = states[uid];
    if (!handle) return;
    handle.running = false;
    if (handle.frame) cancelAnimationFrame(handle.frame);
    if (handle.observer) handle.observer.disconnect();
    delete states[uid];
  };
})();
