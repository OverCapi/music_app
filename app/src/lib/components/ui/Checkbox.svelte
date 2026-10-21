<script lang="ts">
  /** Round-cornered checkbox whose check mark draws itself. */
  let { checked, onchange, label }: { checked: boolean; onchange: () => void; label: string } =
    $props();
</script>

<button
  class="box"
  class:on={checked}
  type="button"
  role="checkbox"
  aria-checked={checked}
  aria-label={label}
  onclick={(e) => {
    e.stopPropagation();
    onchange();
  }}
>
  <svg
    width="12"
    height="12"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="3.2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M5 12.5l4.5 4.5L19 7.5" />
  </svg>
</button>

<style>
  .box {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 1.5px solid var(--mute);
    border-radius: 6px;
    background: transparent;
    color: var(--on-accent);
    transition:
      background 0.25s,
      border-color 0.25s,
      transform 0.35s var(--ease-spring);
  }
  .box.on {
    border-color: var(--accent);
    background: var(--accent);
    transform: scale(1.08);
  }
  path {
    stroke-dasharray: 24;
    stroke-dashoffset: 24;
    transition: stroke-dashoffset 0.35s 0.05s var(--ease-out);
  }
  .on path {
    stroke-dashoffset: 0;
  }
</style>
