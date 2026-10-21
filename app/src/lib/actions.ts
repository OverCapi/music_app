/** Moves the node to `document.body` (escapes `overflow` / `backdrop-filter` ancestors that
 * would clip or re-anchor `position: fixed`). */
export function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return { destroy: () => node.remove() };
}
