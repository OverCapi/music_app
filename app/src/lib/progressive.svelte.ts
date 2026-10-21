/** Renders a long list in chunks: the first `initial` items at once (so the page appears
 * immediately), the rest a few frames later. Use `items.slice(0, p.count)`. */
export function progressive(total: () => number, initial = 30, step = 60) {
  let count = $state(initial);
  $effect(() => {
    const t = total();
    if (count >= t) return;
    const id = setTimeout(() => (count = Math.min(t, count + step)), 16);
    return () => clearTimeout(id);
  });
  return {
    get count() {
      return count;
    },
  };
}
