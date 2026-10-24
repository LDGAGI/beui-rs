// Port of `useDismiss`'s listener body from `lib/hooks/use-dismiss.ts`.
//
// The bookkeeping that decides *when* to dismiss (which target is inside, which
// other overlay owns the gesture) has to read the DOM, and on the desktop and
// mobile renderers the only way to touch the DOM is from inside a script like
// this one. So the listener lives here as a port of the source, and Rust owns
// the lifecycle: one script per open scope, `dioxus.send('dismiss')` to ask for
// the close, `'close'` on the channel to make it tear down.
//
// Placeholders substituted by `use_dismiss`: __BEUI_ID__, __BEUI_ESCAPE__,
// __BEUI_CONSUME__, __BEUI_IGNORE__.

window.__beuiDismiss = window.__beuiDismiss || { scopes: new Map() };

const scopeId = __BEUI_ID__;
const ignoreSelectors = __BEUI_IGNORE__;
const dismissOnEscape = __BEUI_ESCAPE__;
const consume = __BEUI_CONSUME__;

const scopeElement = () =>
  document.querySelector('[data-beui-scope="' + scopeId + '"]');

const matchesIgnore = (target) =>
  ignoreSelectors.some((selector) => {
    try {
      return Boolean(target.closest && target.closest(selector));
    } catch (e) {
      return false;
    }
  });

const inside = (target) => {
  const element = scopeElement();
  if (element && element.contains(target)) return true;
  return matchesIgnore(target);
};

// A consumed dismissal reads this to tell a stray gesture from one that belongs
// to an overlay in front of it: overlays have no shared z-order to consult, but
// the one the gesture landed in has said as much by registering it.
const claimedByAnotherScope = (target) => {
  for (const scope of window.__beuiDismiss.scopes.values()) {
    if (scope !== inside && scope(target)) return true;
  }
  return false;
};

// preventDefault on pointerdown does not suppress the click that follows, so
// consuming a gesture means swallowing that click itself. The swallower
// deliberately outlives the scope that installed it — the dismissal it belongs
// to has already unmounted or re-rendered by the time the click lands. It
// releases on that click, or on the next gesture if the pointer is dragged away
// and no click ever arrives, so it can never eat a later one. A keydown releases
// it too: a gesture that ends with neither a click nor a cancel would otherwise
// leave it armed, and the click Enter synthesizes on some focused control is not
// the one this dismissal was owed.
const consumeActivation = (source) => {
  const swallow = (event) => {
    event.preventDefault();
    event.stopPropagation();
    release();
  };
  const restart = (event) => {
    if (event !== source) release();
  };
  const release = () => {
    window.removeEventListener('click', swallow, true);
    window.removeEventListener('pointerdown', restart, true);
    window.removeEventListener('pointercancel', restart, true);
    window.removeEventListener('keydown', release, true);
  };
  window.addEventListener('click', swallow, true);
  window.addEventListener('pointerdown', restart, true);
  window.addEventListener('pointercancel', restart, true);
  window.addEventListener('keydown', release, true);
};

const onKey = (event) => {
  if (dismissOnEscape && event.key === 'Escape') dioxus.send('dismiss');
};

const onPointer = (event) => {
  const target = event.target;
  if (!target || inside(target)) return;
  // Outside this overlay, but inside one that is also open: the gesture is that
  // overlay's to answer, and swallowing its click from behind would cost the
  // user the control they actually aimed at.
  if (consume && !claimedByAnotherScope(target)) consumeActivation(event);
  dioxus.send('dismiss');
};

// The pointerdown listener is capture-phase: a bubble-phase one is blinded by
// any handler in between that stops propagation, and an overlay cannot know what
// it is layered over.
window.__beuiDismiss.scopes.set(scopeId, inside);
window.addEventListener('keydown', onKey);
window.addEventListener('pointerdown', onPointer, true);

for (;;) {
  const message = await dioxus.recv();
  if (message === 'close') break;
}

window.__beuiDismiss.scopes.delete(scopeId);
window.removeEventListener('keydown', onKey);
window.removeEventListener('pointerdown', onPointer, true);
