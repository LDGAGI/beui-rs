// Move a rendered element under `<body>` — React's `createPortal(…, document.body)`,
// which is what every beUI overlay does.
//
// This is safe to do to a Dioxus subtree, and that is not obvious. Dioxus's DOM
// mutations are *node-relative*: `remove` is `node.remove()`, `insert_before` is
// `node.before(…)`, `insert_after` is `node.after(…)`, and `append_children` is
// `this.nodes[parentId].appendChild(…)` — every operation resolves the node it is
// acting on from the interpreter's id map, and none of them ask "who is your
// parent?". So a moved element keeps being updated where it is, its own children
// keep being appended to it, and removing it later removes it from wherever it
// lives now.
//
// Placeholders substituted by `use_portal`: __BEUI_SELECTOR__.

const element = document.querySelector(__BEUI_SELECTOR__);
if (element && element.parentElement !== document.body) {
  element.setAttribute("data-beui-portaled", "");
  document.body.appendChild(element);
}
