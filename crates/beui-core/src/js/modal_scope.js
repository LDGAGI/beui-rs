// Port of `useModalScope`'s body from `lib/hooks/use-modal-scope.ts`.
//
// Sibling inertness, scroll locking, the focus trap and focus return are all DOM
// operations, and on the desktop and mobile renderers the only way to touch the
// DOM is from inside a script like this one. The module state the source keeps
// in module scope (`inertOwners`, `scrollOwners`, `previousOverflow`) lives on
// `window.__beuiModal` here, so two overlapping modals still nest: the innermost
// one to close restores what it found.
//
// Rust owns the lifecycle: open the scope with one script, `'close'` on the
// channel to tear it down. A scope that finds no panel simply returns — the
// effect can run before the panel has mounted.
//
// Placeholders substituted by `use_modal_scope`: __BEUI_ID__.
//
// One element carries the marker: the panel is what counts as "inside" for a
// dismissal and what the trap holds. beUI's root/panel split only matters
// because it portals a container to `body`.

window.__beuiModal = window.__beuiModal || {
  inertOwners: new Map(),
  scrollOwners: 0,
  previousOverflow: "",
};

const shared = window.__beuiModal;
const scopeId = __BEUI_ID__;

const FOCUSABLE =
  'button:not(:disabled), a[href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

const panel = document.querySelector('[data-beui-scope="' + scopeId + '"]');

if (panel) {
  const returnFocus =
    document.activeElement instanceof HTMLElement ? document.activeElement : null;

  // Everything outside the panel has to go inert — that is the whole point of
  // the scope — but the source reaches that by inerting every child of `body`
  // except the root, which only holds when the overlay has been portaled to
  // `body`. Walking up and inerting the siblings at each level gives exactly the
  // same set (every element outside the subtree is a sibling of one of its
  // ancestors, or a descendant of one), and it stays correct when the overlay is
  // rendered in place. Source behaviour is the degenerate case: a root that is a
  // direct child of `body` inerts every other child of `body`, once.
  const siblings = [];
  for (
    let node = panel;
    node.parentElement && node.parentElement !== document.documentElement;
    node = node.parentElement
  ) {
    for (const element of Array.from(node.parentElement.children)) {
      if (element !== node && element instanceof HTMLElement) siblings.push(element);
    }
  }
  for (const element of siblings) {
    const owner = shared.inertOwners.get(element) ?? {
      count: 0,
      previous: element.inert,
    };
    owner.count += 1;
    shared.inertOwners.set(element, owner);
    element.inert = true;
  }
  if (shared.scrollOwners++ === 0) {
    shared.previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
  }

  const focusables = () =>
    Array.from(panel.querySelectorAll(FOCUSABLE)).filter(
      (element) => element.tabIndex >= 0 && !element.closest("[inert]"),
    );
  const focusFirst = () =>
    (focusables()[0] ?? panel).focus({ preventScroll: true });
  focusFirst();

  const onFocus = (event) => {
    if (
      event.target instanceof Node &&
      !panel.contains(event.target) &&
      !panel.inert
    ) {
      focusFirst();
    }
  };

  const onKey = (event) => {
    if (panel.inert) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      dioxus.send("close");
    }
    if (event.key !== "Tab") return;
    const elements = focusables();
    const first = elements[0];
    const last = elements[elements.length - 1];
    if (!first || !last) {
      event.preventDefault();
      panel.focus();
    } else if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  document.addEventListener("focusin", onFocus);
  document.addEventListener("keydown", onKey);

  for (;;) {
    const message = await dioxus.recv();
    if (message === "close") break;
  }

  document.removeEventListener("focusin", onFocus);
  document.removeEventListener("keydown", onKey);

  for (const element of siblings) {
    const owner = shared.inertOwners.get(element);
    if (owner && --owner.count === 0) {
      element.inert = owner.previous;
      shared.inertOwners.delete(element);
    }
  }
  if (--shared.scrollOwners === 0) {
    document.body.style.overflow = shared.previousOverflow;
  }
  if (returnFocus && returnFocus.isConnected && !returnFocus.closest("[inert]")) {
    returnFocus.focus({ preventScroll: true });
  }
}
