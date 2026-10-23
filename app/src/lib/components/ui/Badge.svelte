<script lang="ts">
  import type { Snippet } from "svelte";

  /** Small uppercase pill: `live` (red, "EN DIRECT") or `group` (outlined glass, "GROUPE").
   * `sm` is the dock size. Position it with `style` (e.g. over a video). */
  let {
    variant = "live",
    size = "md",
    style,
    children,
  }: {
    variant?: "live" | "group";
    size?: "sm" | "md";
    style?: string;
    children: Snippet;
  } = $props();
</script>

<span class="badge {variant} {size}" {style}>{@render children()}</span>

<style>
  .badge {
    flex: none;
    padding: 3px 8px;
    border-radius: 999px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
  }
  .sm {
    padding: 2px 7px;
    font-size: 9px;
  }
  .live {
    background: var(--live);
    color: #fff;
  }
  .group {
    border: 1px solid var(--border);
    background: var(--glass2);
  }
</style>
