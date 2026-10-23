<script lang="ts">
  import type { Snippet } from "svelte";

  /** Small uppercase muted label above a group of items. `inset`: padded for use inside a
   * panel list. Extra attributes go to the `<div>`. */
  let {
    inset = false,
    class: klass = "",
    children,
    ...rest
  }: { inset?: boolean; class?: string; children: Snippet; [key: string]: unknown } = $props();
</script>

<div class="label {klass}" class:inset {...rest}>{@render children()}</div>

<style>
  .label {
    color: var(--mute);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .inset {
    flex: none;
    padding: 12px 8px 6px;
    white-space: nowrap;
  }
</style>
