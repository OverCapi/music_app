<script lang="ts">
  import Badge from "#lib/components/ui/Badge.svelte";
  import type { LiveStream } from "#lib/bindings.ts";
  import Card from "#lib/components/ui/Card.svelte";

  /** 16:9 thumbnail with a red "EN DIRECT" badge, 2-line title, channel · viewers.
   * `playing` draws an accent ring (the live currently in the player). */
  let {
    stream,
    playing = false,
    onplay,
  }: { stream: LiveStream; playing?: boolean; onplay: () => void } = $props();
</script>

<Card onclick={onplay} class={playing ? "playing" : ""}>
  <span class="thumb">
    <img src={stream.thumbnail} alt="" loading="lazy" />
    <Badge style="position:absolute;top:8px;left:8px">EN DIRECT</Badge>
  </span>
  <span class="text">
    <span class="title">{stream.title}</span>
    <span class="sub">
      {stream.channel ?? ""}{#if stream.viewers !== null}
        · {stream.viewers.toLocaleString("fr-FR")} spectateurs{/if}
    </span>
  </span>
</Card>

<style>
  .thumb {
    position: relative;
    display: block;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border-radius: var(--rs);
    background: var(--line);
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    padding: 0 4px 4px;
  }
  .title {
    display: -webkit-box;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    font-size: 14px;
    font-weight: 600;
    line-height: 1.3;
  }
  .sub {
    color: var(--mute);
    font-size: 12px;
  }
  :global(.card.playing) {
    box-shadow:
      0 0 0 2px var(--accent),
      0 20px 50px -18px var(--shadow) !important;
  }
</style>
