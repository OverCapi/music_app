<script lang="ts">
  import type { Snippet } from "svelte";
  import { liquid } from "#lib/motion.ts";

  /** Small selectable pill (browser choice). */
  let {
    selected = false,
    children,
    ...rest
  }: { selected?: boolean; children: Snippet; [key: string]: unknown } = $props();
</script>

<button class="chip" class:selected type="button" aria-pressed={selected} use:liquid {...rest}>
  {@render children()}
</button>

<style>
  .chip {
    position: relative;
    overflow: hidden;
    height: 34px;
    padding: 0 16px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--line);
    color: var(--fg);
    font-size: 13px;
    font-weight: 600;
    transition:
      background 0.25s,
      border-color 0.25s,
      color 0.25s;
  }
  .chip:hover:not(:disabled) {
    background: var(--glass2);
  }
  .chip.selected {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }
  .chip:disabled {
    opacity: 0.5;
  }
</style>
