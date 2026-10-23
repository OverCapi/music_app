<script lang="ts">
  import Badge from "#lib/components/ui/Badge.svelte";
  /** The live currently playing, shown as a 16:9 panel (max 820 px). Click to expand it
   * fullscreen, animated from this panel. */
  import { lives } from "#lib/stores/lives.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import LiveStage from "#lib/components/live/LiveStage.svelte";

  const live = $derived(player.state.live);
  let panel = $state<HTMLElement>();
</script>

{#if live}
  <button
    bind:this={panel}
    class="panel"
    type="button"
    aria-label="Agrandir la vidéo"
    onclick={() => lives.expandVideo(panel!.getBoundingClientRect())}
  >
    {#if !lives.videoOpen}<LiveStage />{/if}
    <Badge style="position:absolute;top:14px;left:14px;z-index:1">EN DIRECT</Badge>
    <span class="bar">
      <span class="title">{live.title}</span>
      <Icon name="expand" size={18} />
    </span>
  </button>
{/if}

<style>
  .panel {
    position: relative;
    display: block;
    width: 100%;
    max-width: 820px;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border-radius: calc(var(--rs) + 6px);
    background: #000;
    box-shadow: 0 30px 60px -24px var(--shadow);
    color: #fff;
    cursor: zoom-in;
  }
  .bar {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    z-index: 1;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 40px 18px 16px;
    background: linear-gradient(to top, oklch(0 0 0 / 0.6), transparent);
    text-align: left;
  }
  .title {
    overflow: hidden;
    font-size: 17px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
