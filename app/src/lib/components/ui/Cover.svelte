<script lang="ts">
  import Icon from "./Icon.svelte";

  /** Fills its parent (which sets size, radius and `position: relative`; `overflow: hidden`):
   * one image, or a 2×2 mosaic when `mosaic` and at least 2 sources are given (missing tiles
   * stay blank). */
  let { src, mosaic = false }: { src?: (string | undefined)[] | string; mosaic?: boolean } =
    $props();

  const list = $derived(Array.isArray(src) ? src : src ? [src] : []);
  const tiles = $derived(mosaic && list.length >= 2 ? [0, 1, 2, 3].map((i) => list[i]) : [list[0]]);
  let failed = $state<Record<string, boolean>>({});
</script>

<span class="cover" class:mosaic={tiles.length === 4}>
  {#each tiles as url, i (i)}
    {#if url && !failed[url]}
      <img src={url} alt="" loading="lazy" onerror={() => (failed[url] = true)} />
    {:else}
      <span class="blank"><Icon name="note" size={24} /></span>
    {/if}
  {/each}
</span>

<style>
  .cover {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template: 1fr / 1fr;
    background: var(--line);
  }
  .cover.mosaic {
    grid-template: 1fr 1fr / 1fr 1fr;
  }
  .blank {
    display: grid;
    place-items: center;
    color: var(--muted);
    opacity: 0.6;
  }
  img,
  .blank {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    object-fit: cover;
  }
</style>
