<script lang="ts">
  import { liquid } from "#lib/motion.ts";
  import Cover from "#lib/components/ui/Cover.svelte";

  /** Compact horizontal glass card: 48 px cover, title, artist (recently played). */
  let {
    cover,
    title,
    subtitle,
    onclick,
  }: { cover?: string; title: string; subtitle?: string; onclick: () => void } = $props();
</script>

<button class="glass-flat mini" type="button" data-liquid use:liquid {onclick}>
  <span class="art"><Cover src={cover} /></span>
  <span class="text">
    <span class="title">{title}</span>
    {#if subtitle}<span class="sub">{subtitle}</span>{/if}
  </span>
</button>

<style>
  .mini {
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 8px;
    border-radius: calc(var(--rs) + 6px);
    color: var(--fg);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      background 0.3s,
      transform 0.45s var(--ease-spring);
  }
  .mini:hover {
    background: var(--glass2);
    transform: translateY(-3px);
  }
  .art {
    position: relative;
    flex: none;
    width: 48px;
    height: 48px;
    overflow: hidden;
    border-radius: calc(var(--rs) / 1.5);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .title,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title {
    font-size: 14px;
    font-weight: 600;
  }
  .sub {
    color: var(--mute);
    font-size: 12px;
  }
</style>
