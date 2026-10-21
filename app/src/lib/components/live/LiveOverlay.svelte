<script lang="ts">
  /** Fullscreen live video. Grows from the Lives page panel (`lives.videoFrom`) and shrinks
   * back on close. Closes by itself when the live ends. */
  import { onMount } from "svelte";
  import { lives } from "#lib/stores/lives.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import IconButton from "#lib/components/ui/IconButton.svelte";
  import LiveStage from "#lib/components/live/LiveStage.svelte";

  let root = $state<HTMLElement>();
  const live = $derived(player.state.live);
  const from = lives.videoFrom;

  /** `clip-path` that shows only the panel's rectangle. */
  function inset(r: DOMRect | null) {
    if (!r) return "inset(50% 50% 50% 50% round 24px)";
    return `inset(${r.top}px ${innerWidth - r.right}px ${innerHeight - r.bottom}px ${r.left}px round 22px)`;
  }

  onMount(() => {
    root?.animate([{ clipPath: inset(from) }, { clipPath: "inset(0px round 0px)" }], {
      duration: 520,
      easing: "cubic-bezier(.32,.72,0,1)",
    });
  });

  async function close() {
    const target = lives.videoFrom;
    if (root) {
      await root.animate([{ clipPath: "inset(0px round 0px)" }, { clipPath: inset(target) }], {
        duration: 420,
        easing: "cubic-bezier(.32,.72,0,1)",
        fill: "forwards",
      }).finished;
    }
    lives.videoOpen = false;
  }

  // The live ended (stopped, or another track started): nothing left to show.
  $effect(() => {
    if (!live) lives.videoOpen = false;
  });
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && close()} />

<div class="overlay" bind:this={root}>
  <LiveStage fit="contain" />
  <div class="close">
    <IconButton icon="chevronDown" label="Réduire" size={44} iconSize={20} onclick={close} />
  </div>
  <div class="info">
    <span class="badge-live">EN DIRECT</span>
    <span class="title">{live?.title ?? ""}</span>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: #000;
    color: #fff;
  }
  .close {
    position: absolute;
    top: 22px;
    left: 24px;
    z-index: 2;
    display: grid;
    border-radius: 50%;
    background: var(--glass);
    -webkit-backdrop-filter: blur(var(--blur));
    backdrop-filter: blur(var(--blur));
    border: 1px solid var(--border);
    box-shadow: inset 0 1px 0 var(--hi);
  }
  .info {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 48px 28px 22px;
    background: linear-gradient(to top, oklch(0 0 0 / 0.65), transparent);
    pointer-events: none;
  }
  .title {
    overflow: hidden;
    font-size: 17px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
