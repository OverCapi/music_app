<script lang="ts">
  import { fileSrc } from '../utils'
  import Icon from './Icon.svelte'

  let { path, size = 40 }: { path: string | null; size?: number } = $props()
  let failed = $state(false)
  const src = $derived(fileSrc(path))

  $effect(() => {
    void path
    failed = false
  })
</script>

<div class="cover" style:width="{size}px" style:height="{size}px">
  {#if src && !failed}
    <img {src} alt="" loading="lazy" onerror={() => (failed = true)} />
  {:else}
    <Icon name="note" size={Math.round(size * 0.45)} />
  {/if}
</div>

<style>
  .cover {
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 6px;
    overflow: hidden;
    background: var(--surface-3);
    color: var(--text-dim);
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
