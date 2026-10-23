/** Wall-clock `now` (ms) refreshed every `ms` while `active()`; call during component init.
 * Feeds position interpolation (`player.positionAt(now)`). */
export function ticker(active: () => boolean, ms = 250) {
  let now = $state(Date.now());
  $effect(() => {
    if (!active()) return;
    now = Date.now();
    const t = setInterval(() => (now = Date.now()), ms);
    return () => clearInterval(t);
  });
  return {
    get now() {
      return now;
    },
  };
}
