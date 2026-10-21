<script lang="ts">
  /** Right panel: now playing (cover + visualizer), remaining time, "Ensuite" list with
   * duration and drag-to-reorder. Collapsed (66 px): round cover + thumbnails only. */
  import { tick } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { QueueItem } from "#lib/bindings.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { ui } from "#lib/stores/ui.svelte.ts";
  import { fileName, formatTime, formatTotal } from "#lib/format.ts";
  import { flip, liquid } from "#lib/motion.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import Visualizer from "#lib/components/ui/Visualizer.svelte";

  let list = $state<HTMLElement>();
  let dragging = $state<number | null>(null); // queue index being dragged
  let lock = 0;

  const s = $derived(player.state);
  const track = $derived(s.track_id !== null ? library.byId.get(s.track_id) : undefined);
  // The panel lists at most the next 15 tracks (the engine queue keeps the rest).
  const MAX_UPCOMING = 15;
  const all = $derived(player.upcoming);
  const upcoming = $derived(all.slice(0, MAX_UPCOMING));
  const remaining = $derived(
    all.reduce((n, it) => n + (library.byId.get(it.track_id)?.duration ?? 0), 0) +
      Math.max(0, player.duration - player.positionAt(Date.now())),
  );

  function info({ track_id, path }: QueueItem) {
    const t = library.byId.get(track_id);
    return {
      title: t?.title ?? (path ? fileName(path) : "Titre inconnu"),
      artist: t?.artist ?? "Artiste inconnu",
      duration: t?.duration,
    };
  }

  async function over(e: DragEvent, index: number) {
    e.preventDefault();
    if (dragging === null || dragging === index || performance.now() < lock) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (dragging < index ? e.clientY < r.top + r.height / 2 : e.clientY > r.top + r.height / 2)
      return;
    const done = flip(list!);
    const from = dragging;
    dragging = index;
    lock = performance.now() + 90;
    await player.moveInQueue(from, index);
    await tick();
    done();
  }
</script>

<aside
  class="panel glass"
  class:drawer={ui.compact}
  class:closed={ui.compact && !ui.queueOpen}
  style:width="{ui.queueWidth}px"
>
  <header class:collapsed={!ui.queueExpanded}>
    <button
      class="halo-btn chev"
      class:flipped={!ui.queueExpanded}
      use:liquid
      onclick={() => (ui.queueOpen = !ui.queueOpen)}
      aria-label={ui.queueOpen ? "Replier" : "Déplier"}
    >
      <Icon name="chevronRight" size={16} />
    </button>
    <span class="fade head" class:on={ui.queueExpanded}>
      <span class="title">En lecture</span>
      <span class="mono remaining">≈ {formatTotal(remaining)}</span>
    </span>
  </header>

  <div class="list" bind:this={list}>
    {#if group.isActive || s.group_mode}
      <p class="muted note fade" class:on={ui.queueExpanded}>La lecture suit la file du groupe.</p>
    {:else}
      <div class="now" class:open={ui.queueExpanded}>
        <button class="cover" onclick={() => (ui.fullscreen = true)} aria-label="Plein écran">
          {#if s.track_id !== null}
            {#key s.track_id}<img
                class="swap"
                src={convertFileSrc(String(s.track_id), "cover")}
                alt=""
              />{/key}
          {/if}
          <span class="viz"
            ><Visualizer playing={s.playing} levels={player.levels} bars={22} /></span
          >
        </button>
        <span class="now-meta">
          <span class="t">{track?.title ?? ""}</span>
          <span class="a">{track?.artist ?? ""}</span>
        </span>
      </div>

      <div class="section-title fade" class:on={ui.queueExpanded}>Ensuite</div>
      {#each upcoming as item (item.track_id + ":" + item.index)}
        {@const t = info(item)}
        <div
          class="row item"
          data-flip={item.track_id + ":" + item.index}
          role="button"
          tabindex="0"
          draggable="true"
          use:liquid
          ondragstart={(e) => {
            e.dataTransfer!.effectAllowed = "move";
            dragging = item.index;
          }}
          ondragover={(e) => over(e, item.index)}
          ondragend={() => (dragging = null)}
          onclick={() => player.playIndex(item.index)}
          onkeydown={(e) => e.key === "Enter" && player.playIndex(item.index)}
        >
          <span class="ghost" class:on={dragging === item.index}></span>
          <img
            class="thumb"
            src={convertFileSrc(String(item.track_id), "cover")}
            alt=""
            loading="lazy"
          />
          <span class="fade meta" class:on={ui.queueExpanded}>
            <span class="text">
              <span class="t">{t.title}</span>
              <span class="a">{t.artist}</span>
            </span>
            {#if t.duration !== undefined}<span class="mono dur">{formatTime(t.duration)}</span
              >{/if}
            <span class="grip"><Icon name="grip" size={14} /></span>
          </span>
        </div>
      {/each}
      {#if all.length > MAX_UPCOMING}
        <p class="muted rest">+ {all.length - MAX_UPCOMING} autres titres</p>
      {/if}
    {/if}
  </div>
</aside>

<style>
  .panel {
    position: absolute;
    top: 16px;
    right: 16px;
    bottom: 16px;
    z-index: 12;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    overflow: hidden;
    border-radius: calc(var(--rs) + 10px);
    transition: width 0.55s var(--ease-out);
  }
  .panel.drawer {
    transition:
      transform 0.55s var(--ease-out),
      opacity 0.3s;
  }
  .panel.closed {
    opacity: 0;
    pointer-events: none;
    transform: translateX(calc(100% + 24px));
  }
  header {
    display: flex;
    flex: none;
    align-items: center;
    padding: 14px 14px 8px;
    transition: padding 0.55s var(--ease-out);
  }
  /* Collapsed panel is 66 px wide: the 36 px button sits exactly in the middle. */
  header.collapsed {
    padding-inline: 15px;
  }
  header.collapsed .head {
    flex: 0 0 0;
  }
  .chev :global(svg) {
    transition: transform 0.5s;
  }
  .chev.flipped :global(svg) {
    transform: rotate(180deg);
  }
  .head {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
  }
  .title {
    flex: 1;
    padding-left: 8px;
    color: var(--mute);
    font-size: 13px;
    font-weight: 600;
  }
  .remaining {
    padding-right: 6px;
    color: var(--mute);
    font-size: 11px;
  }
  .list {
    /* Fixed content width: the panel animates its width without re-laying out every row. */
    min-width: min(300px, calc(100vw - 32px));
    flex: 1;
    min-height: 0;
    overflow: hidden auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px 8px 12px;
  }
  .now {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 2px 4px 6px;
    transition:
      padding 0.55s var(--ease-out),
      gap 0.55s var(--ease-out);
  }
  .now.open {
    gap: 12px;
    padding: 2px 6px 8px;
  }
  .cover {
    position: relative;
    flex: none;
    width: 40px;
    height: 40px;
    padding: 0;
    overflow: hidden;
    border-radius: 50%;
    background: var(--line);
    box-shadow: 0 0 0 2px var(--accent);
    transition:
      width 0.55s var(--ease-out),
      height 0.55s var(--ease-out),
      border-radius 0.55s var(--ease-out),
      box-shadow 0.4s;
  }
  .now.open .cover {
    width: min(272px, 100%);
    height: auto;
    aspect-ratio: 1;
    border-radius: var(--rs);
    box-shadow: 0 20px 40px -16px var(--shadow);
  }
  .cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .swap {
    animation: swap 0.45s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  @keyframes swap {
    from {
      opacity: 0.3;
      transform: scale(1.1);
      filter: blur(10px);
    }
  }
  .viz {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 34%;
    box-sizing: border-box;
    padding: 0 16px;
    background: linear-gradient(to top, oklch(0 0 0 / 0.4), transparent);
    opacity: 0;
    transition: opacity 0.3s;
  }
  .now.open .viz {
    opacity: 1;
  }
  .now-meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    max-height: 0;
    overflow: hidden;
    opacity: 0;
    transition:
      max-height 0.55s var(--ease-out),
      opacity 0.3s;
  }
  .now.open .now-meta {
    max-height: 60px;
    opacity: 1;
  }
  .now-meta .t {
    font-size: 16px;
    font-weight: 600;
  }
  .section-title {
    flex: none;
    padding: 10px 8px 6px;
    white-space: nowrap;
  }
  .rest {
    margin: 6px 8px;
    font-size: 12px;
    white-space: nowrap;
  }
  .item {
    flex: none;
    /* Rows outside the viewport are not laid out or painted. */
    content-visibility: auto;
    contain-intrinsic-size: auto 48px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px;
    cursor: grab;
  }
  .ghost {
    position: absolute;
    inset: 0;
    border-radius: var(--rs);
    background: var(--glass2);
    box-shadow:
      inset 0 0 0 1px var(--border),
      0 8px 20px -10px var(--shadow);
    opacity: 0;
    transition: opacity 0.2s;
    pointer-events: none;
  }
  .ghost.on {
    opacity: 1;
  }
  .thumb {
    position: relative;
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: calc(var(--rs) - 2px);
    object-fit: cover;
    background: var(--line);
  }
  .fade {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .fade.on {
    opacity: 1;
    transition: opacity 0.35s 0.12s;
  }
  .meta {
    position: relative;
    display: flex;
    flex: 1;
    align-items: center;
    gap: 10px;
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .t,
  .a {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .t {
    font-size: 14px;
    font-weight: 600;
  }
  .a {
    color: var(--mute);
    font-size: 12px;
  }
  .dur {
    color: var(--mute);
    font-size: 11px;
  }
  .grip {
    margin-right: 4px;
    opacity: 0.45;
  }
  .note {
    margin: 8px;
    font-size: 13px;
  }
</style>
