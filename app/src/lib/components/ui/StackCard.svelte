<script lang="ts">
  import type { Snippet } from "svelte";
  import Panel from "#lib/components/ui/Panel.svelte";

  /** Glass card laying its content out as a padded column (settings blocks). */
  let { children }: { children: Snippet } = $props();
</script>

<Panel radius={12} class="stack-card">
  {@render children()}
</Panel>

<style>
  :global(.stack-card) {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 22px 24px;
  }
</style>
