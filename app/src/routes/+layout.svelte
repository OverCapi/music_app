<script lang="ts">
  import "../app.css";
  import { afterNavigate, beforeNavigate } from "$app/navigation";
  import { onMount, tick, type Snippet } from "svelte";
  import { player } from "#lib/stores/player.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { playlists } from "#lib/stores/playlists.svelte.ts";
  import { search } from "#lib/stores/search.svelte.ts";
  import { downloads } from "#lib/stores/downloads.svelte.ts";
  import { account } from "#lib/stores/account.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { feedback } from "#lib/stores/feedback.svelte.ts";
  import { sync } from "#lib/stores/sync.svelte.ts";
  import { storage } from "#lib/stores/storage.svelte.ts";
  import { lives } from "#lib/stores/lives.svelte.ts";
  import { ui } from "#lib/stores/ui.svelte.ts";
  import { errorMessage, toasts } from "#lib/stores/toasts.svelte.ts";
  import { enterPage, exitPage, watchReflow } from "#lib/motion.ts";
  import Sidebar from "#lib/components/Sidebar.svelte";
  import TopBar from "#lib/components/TopBar.svelte";
  import PlayerBar from "#lib/components/player/PlayerBar.svelte";
  import QueueView from "#lib/components/player/QueueView.svelte";
  import FullscreenPlayer from "#lib/components/player/FullscreenPlayer.svelte";
  import LiveOverlay from "#lib/components/live/LiveOverlay.svelte";
  import Toasts from "#lib/components/ui/Toasts.svelte";

  let { children }: { children: Snippet } = $props();
  let pageEl = $state<HTMLElement>();
  let main = $state<HTMLElement>();

  // Page transition: 130 ms exit, then staggered enter once the new route rendered.
  beforeNavigate(({ to, from, type }) => {
    if (!pageEl || type === "popstate" || to?.url.pathname === from?.url.pathname) return;
    exitPage(pageEl);
  });
  afterNavigate(async () => {
    lives.videoOpen = false;
    await tick();
    if (pageEl) enterPage(pageEl);
  });

  // Panels resizing → cards changing row fade back in instead of jumping.
  $effect(() => {
    void ui.sideOpen;
    void ui.queueOpen;
    if (pageEl) watchReflow(pageEl);
  });

  onMount(() => {
    const ro = new ResizeObserver(([e]) => ui.onResize(e.contentRect.width));
    ro.observe(main!);
    account.refresh();
    const unlisteners = [
      player,
      library,
      playlists,
      search,
      downloads,
      group,
      sync,
      feedback,
      storage,
    ].map((store) =>
      store.init().catch((e) => {
        toasts.push(errorMessage(e));
        return () => {};
      }),
    );
    return () => {
      ro.disconnect();
      unlisteners.forEach((p) => p.then((u) => u()));
    };
  });
</script>

<main bind:this={main}>
  <div class="blobs" aria-hidden="true">
    <span class="b1"></span><span class="b2"></span><span class="b3"></span>
  </div>

  <div class="page view" style:left="{ui.left}px" style:right="{ui.right}px">
    <div class="inner" bind:this={pageEl}>
      {@render children()}
    </div>
  </div>

  {#if ui.drawerOpen}
    <button class="backdrop" aria-label="Fermer" onclick={() => ui.closeDrawers()}></button>
  {/if}
  <TopBar />
  <PlayerBar />
  <Sidebar />
  <QueueView />
  {#if lives.videoOpen}<LiveOverlay />{/if}
  {#if ui.fullscreen}<FullscreenPlayer onclose={() => (ui.fullscreen = false)} />{/if}
</main>
<Toasts />

<style>
  main {
    position: relative;
    height: 100vh;
    overflow: hidden;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    z-index: 11;
    background: oklch(0 0 0 / 0.35);
    animation: fade 0.3s;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .page {
    position: absolute;
    top: 0;
    bottom: 0;
    /* No left/right transition: animating them re-lays out the whole page every frame (the
       main cause of lag when a panel opens). Cards that change row fade back in instead
       (`watchReflow`). */
  }
  /* Fixed abstract background: 3 blurred blobs drifting slowly (16–26 s, alternate). */
  .blobs {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }
  .blobs span {
    position: absolute;
    aspect-ratio: 1;
    border-radius: 50%;
    will-change: transform;
    animation: drift 16s ease-in-out infinite alternate;
  }
  .b1 {
    width: 62%;
    left: -12%;
    top: -28%;
    background: radial-gradient(circle, var(--blob-1) 0%, transparent 62%);
  }
  .b2 {
    width: 55%;
    right: -14%;
    top: 10%;
    background: radial-gradient(circle, var(--blob-2) 0%, transparent 62%);
    animation-duration: 21s !important;
    animation-direction: alternate-reverse !important;
  }
  .b3 {
    width: 50%;
    left: 22%;
    bottom: -38%;
    background: radial-gradient(circle, var(--blob-3) 0%, transparent 62%);
    animation-duration: 26s !important;
  }
  @keyframes drift {
    to {
      transform: translate(10%, 8%) scale(1.15);
    }
  }
</style>
