<script lang="ts">
  import type { Snippet } from "svelte";
  import SectionLabel from "#lib/components/ui/SectionLabel.svelte";

  /** Page section: `h2` title (with optional trailing actions) and an optional small
   * uppercase label above the content (e.g. "Que du connu"). */
  let {
    title,
    label,
    actions,
    children,
  }: { title?: string; label?: string; actions?: Snippet; children: Snippet } = $props();
</script>

<section>
  {#if title || actions}
    <div class="head">
      {#if title}<h2>{title}</h2>{/if}
      {@render actions?.()}
    </div>
  {/if}
  {#if label}<SectionLabel>{label}</SectionLabel>{/if}
  {@render children()}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
</style>
