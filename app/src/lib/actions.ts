/** Moves the node to `document.body` (escapes `overflow` / `backdrop-filter` ancestors that
 * would clip or re-anchor `position: fixed`). */
export function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return { destroy: () => node.remove() };
}

/** Horizontal position of the pointer inside `el`, from 0 (left edge) to 1 (right edge). */
export function pointerFraction(e: { clientX: number }, el: HTMLElement): number {
  const r = el.getBoundingClientRect();
  return Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
}

/** Slider behaviour for any element: press or drag calls `onvalue` with the pointer fraction
 * (0–1); `ondragging` reports when a drag starts / ends. */
export function dragValue(
  node: HTMLElement,
  opts: { onvalue: (fraction: number) => void; ondragging?: (on: boolean) => void },
) {
  let o = opts;
  const down = (e: PointerEvent) => {
    o.ondragging?.(true);
    o.onvalue(pointerFraction(e, node));
    const move = (ev: PointerEvent) => o.onvalue(pointerFraction(ev, node));
    const up = () => {
      removeEventListener("pointermove", move);
      removeEventListener("pointerup", up);
      o.ondragging?.(false);
    };
    addEventListener("pointermove", move);
    addEventListener("pointerup", up);
  };
  node.addEventListener("pointerdown", down);
  return {
    update: (next: typeof opts) => (o = next),
    destroy: () => node.removeEventListener("pointerdown", down),
  };
}
