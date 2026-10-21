/** Halo motion helpers: liquid press, FLIP reorder, page enter/exit, reflow fade.
 * All animations use the Web Animations API so they never fight Svelte's DOM updates. */

export const EASE_OUT = "cubic-bezier(.32,.72,0,1)";
export const EASE_ENTER = "cubic-bezier(.2,.85,.25,1)";
export const EASE_SPRING = "cubic-bezier(.3,1.4,.5,1)";

/** `use:liquid` — squash + radial highlight from the pointer on press.
 * Element must be `position: relative; overflow: hidden`. `intensity` 1..5 (default 3). */
export function liquid(node: HTMLElement, intensity = 3) {
  const m = intensity / 3;
  function down(e: PointerEvent) {
    node.animate(
      [
        { transform: "scale(1)" },
        { transform: `scale(${1 - 0.07 * m},${1 - 0.035 * m})`, offset: 0.3 },
        { transform: `scale(${1 + 0.03 * m},${1 - 0.01 * m})`, offset: 0.65 },
        { transform: "scale(1)" },
      ],
      { duration: 520, easing: EASE_SPRING },
    );
    const r = node.getBoundingClientRect();
    const size = Math.max(node.offsetWidth, node.offsetHeight) * 2.2;
    const s = document.createElement("span");
    Object.assign(s.style, {
      position: "absolute",
      left: `${e.clientX - r.left - size / 2}px`,
      top: `${e.clientY - r.top - size / 2}px`,
      width: `${size}px`,
      height: `${size}px`,
      borderRadius: "50%",
      background: "radial-gradient(circle, var(--hi) 0%, transparent 60%)",
      pointerEvents: "none",
    });
    node.appendChild(s);
    s.animate(
      [
        { transform: "scale(0)", opacity: 0.9 },
        { transform: "scale(1)", opacity: 0 },
      ],
      {
        duration: 700,
        easing: "cubic-bezier(.2,.8,.2,1)",
      },
    ).onfinish = () => s.remove();
  }
  node.addEventListener("pointerdown", down);
  return { destroy: () => node.removeEventListener("pointerdown", down) };
}

/** FLIP for vertical lists (`[data-flip="<key>"]` children of `root`).
 * Call `const done = flip(root)` BEFORE the state change, then `await tick(); done()`. */
export function flip(root: HTMLElement) {
  const before = new Map<string, number>();
  root.querySelectorAll<HTMLElement>("[data-flip]").forEach((el) => {
    before.set(el.dataset.flip!, el.getBoundingClientRect().top);
  });
  return () => {
    root.querySelectorAll<HTMLElement>("[data-flip]").forEach((el) => {
      el.getAnimations().forEach((a) => a.cancel());
      const old = before.get(el.dataset.flip!);
      const now = el.getBoundingClientRect().top;
      if (old === undefined) {
        el.animate(
          [
            { opacity: 0, transform: "translateY(10px) scale(.96)" },
            { opacity: 1, transform: "none" },
          ],
          {
            duration: 320,
            easing: "cubic-bezier(.2,.8,.2,1)",
          },
        );
      } else if (Math.abs(old - now) > 0.5) {
        el.animate([{ transform: `translateY(${old - now}px)` }, { transform: "none" }], {
          duration: 340,
          easing: "cubic-bezier(.2,.8,.2,1)",
        });
      }
    });
  };
}

/** Page exit (130 ms) — run in `beforeNavigate`/before switching tabs. */
export function exitPage(el: HTMLElement) {
  el.getAnimations().forEach((a) => a.cancel());
  return el.animate(
    [
      { opacity: 1, transform: "none" },
      { opacity: 0, transform: "scale(.985) translateY(-6px)" },
    ],
    { duration: 130, easing: "cubic-bezier(.4,0,1,1)", fill: "forwards" },
  ).finished;
}

/** Page enter: container fade + stagger of visible items (h1, h2, inputs, cards, rows). */
export function enterPage(el: HTMLElement, selector = "h1, h2, input, [data-liquid], [data-art]") {
  el.getAnimations().forEach((a) => a.cancel());
  el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 180, easing: "ease-out" });
  const top = el.getBoundingClientRect().top;
  const vh = window.innerHeight;
  Array.from(el.querySelectorAll<HTMLElement>(selector))
    .filter((n) => n.getBoundingClientRect().top - top < vh)
    .slice(0, 20)
    .forEach((n, i) =>
      n.animate(
        [
          { opacity: 0, transform: "translateY(14px) scale(.96)" },
          { opacity: 1, transform: "none" },
        ],
        { duration: 360, delay: Math.min(i * 18, 280), easing: EASE_ENTER, fill: "backwards" },
      ),
    );
}

/** While a side panel resizes (~1 s), grid cards that jump to another row
 * fade/scale back in with a small stagger instead of teleporting. */
export function watchReflow(container: HTMLElement, ms = 1000) {
  const pos = new Map<HTMLElement, [number, number]>();
  const items = () => Array.from(container.querySelectorAll<HTMLElement>("[data-liquid]"));
  items().forEach((n) => pos.set(n, [n.offsetLeft, n.offsetTop]));
  const until = performance.now() + ms;
  let k = 0;
  function frame(t: number) {
    items().forEach((n) => {
      const p = pos.get(n);
      const x = n.offsetLeft,
        y = n.offsetTop;
      if (p && (Math.abs(p[1] - y) > 8 || Math.abs(p[0] - x) > 60)) {
        n.getAnimations().forEach((a) => a.cancel());
        n.animate(
          [
            { opacity: 0, transform: "scale(.82)" },
            { opacity: 1, transform: "none" },
          ],
          {
            duration: 520,
            delay: (k++ % 6) * 40,
            easing: "cubic-bezier(.25,.8,.25,1)",
            fill: "backwards",
          },
        );
      }
      pos.set(n, [x, y]);
    });
    if (t < until) requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}
