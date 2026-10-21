<script lang="ts">
  /** Left panel: playlists, pinned first, drag to reorder (also across sections), collapsible.
   * Order and pinning persist through `reorder_playlists` / `set_playlist_pinned`. */
  import { tick } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { Playlist } from "#lib/bindings.ts";
  import { playlists } from "#lib/stores/playlists.svelte.ts";
  import { ui } from "#lib/stores/ui.svelte.ts";
  import { progressive } from "#lib/progressive.svelte.ts";
  import { flip, liquid } from "#lib/motion.ts";
  import { plural } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  type Entry = { id: number; pinned: boolean };

  let list = $state<HTMLElement>();
  let order = $state<Entry[]>([]);
  let dragging = $state<number | null>(null);
  let lock = 0;

  // Local order mirrors the store until a drag commits it.
  $effect(() => {
    if (dragging === null) order = playlists.items.map((p) => ({ id: p.id, pinned: p.pinned }));
  });

  const byId = $derived(new Map(playlists.items.map((p) => [p.id, p])));
  const section = (pinned: boolean) =>
    order
      .filter((e) => e.pinned === pinned)
      .map((e) => byId.get(e.id)!)
      .filter(Boolean);
  const pinned = $derived(section(true));
  const shownOthers = progressive(() => section(false).length, 25, 40);
  const others = $derived(section(false));

  /** Moves the dragged entry to index `to`, joining the section `pin`. */
  async function move(to: number, pin: boolean) {
    const from = order.findIndex((e) => e.id === dragging);
    if (from < 0) return;
    const done = flip(list!);
    const next = order.slice();
    const [entry] = next.splice(from, 1);
    next.splice(to, 0, { ...entry, pinned: pin });
    order = next;
    lock = performance.now() + 90;
    await tick();
    done();
  }

  function over(e: DragEvent, id: number) {
    e.preventDefault();
    if (dragging === null || dragging === id || performance.now() < lock) return;
    const from = order.findIndex((x) => x.id === dragging);
    const to = order.findIndex((x) => x.id === id);
    // Only swap once the pointer crosses the target's middle (no jitter).
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (from < to ? e.clientY < r.top + r.height / 2 : e.clientY > r.top + r.height / 2) return;
    return move(to, order[to].pinned);
  }

  /** Dropping on a section title (also lets an empty section receive an item). */
  function overTitle(e: DragEvent, pin: boolean) {
    e.preventDefault();
    if (dragging === null || performance.now() < lock) return;
    const cur = order.find((x) => x.id === dragging);
    if (!cur || cur.pinned === pin) return;
    // Index just after the pinned entries (dragged one excluded): end of "Épinglées" or
    // start of "Toutes".
    return move(order.filter((x) => x.pinned && x.id !== dragging).length, pin);
  }

  async function end() {
    const id = dragging;
    dragging = null;
    const entry = order.find((e) => e.id === id);
    if (entry && byId.get(entry.id)?.pinned !== entry.pinned) {
      await playlists.setPinned(entry.id, entry.pinned);
    }
    await playlists.reorder(order.map((e) => e.id));
  }

  async function create() {
    const p = await playlists.create("Nouvelle playlist");
    ui.sideOpen = true;
    if (p) goto(resolve("/playlist/[id]", { id: String(p.id) }));
  }

  function covers(p: Playlist) {
    return [0, 1, 2, 3].map((i) => p.track_ids[i]);
  }
</script>

<aside
  class="panel glass"
  class:drawer={ui.compact}
  class:closed={ui.compact && !ui.sideOpen}
  style:width="{ui.sideWidth}px"
>
  <header class:collapsed={!ui.sideExpanded}>
    <span class="fade title" class:on={ui.sideExpanded}>Playlists</span>
    <button
      class="halo-btn chev"
      class:flipped={!ui.sideExpanded}
      use:liquid
      onclick={() => (ui.sideOpen = !ui.sideOpen)}
      aria-label={ui.sideOpen ? "Replier" : "Déplier"}
    >
      <Icon name="chevronLeft" size={16} />
    </button>
  </header>

  <button class="row item new" use:liquid onclick={create}>
    <span class="tile dashed"><Icon name="plus" size={16} /></span>
    <span class="fade" class:on={ui.sideExpanded}>Nouvelle playlist</span>
  </button>

  <div class="list" bind:this={list}>
    <div
      class="section-title fade"
      class:on={ui.sideExpanded}
      role="presentation"
      ondragover={(e) => overTitle(e, true)}
    >
      Épinglées
    </div>
    {#each pinned as p (p.id)}{@render item(p)}{/each}
    <div
      class="section-title fade"
      class:on={ui.sideExpanded}
      role="presentation"
      ondragover={(e) => overTitle(e, false)}
    >
      Toutes
    </div>
    {#each others.slice(0, shownOthers.count) as p (p.id)}{@render item(p)}{/each}
  </div>
</aside>

{#snippet item(p: Playlist)}
  {@const href = resolve("/playlist/[id]", { id: String(p.id) })}
  <a
    class="row item"
    data-flip={p.id}
    data-liquid
    {href}
    draggable="true"
    use:liquid
    ondragstart={(e) => {
      e.dataTransfer!.effectAllowed = "move";
      dragging = p.id;
    }}
    ondragover={(e) => over(e, p.id)}
    ondragend={end}
    aria-current={page.url.pathname === href ? "page" : undefined}
  >
    <span class="hl" class:on={page.url.pathname === href || dragging === p.id}></span>
    <span class="tile mosaic">
      {#each covers(p) as id, i (i)}
        {#if id !== undefined}<img
            src={convertFileSrc(String(id), "cover")}
            alt=""
            loading="lazy"
          />{:else}<span></span>{/if}
      {/each}
    </span>
    <span class="meta fade" class:on={ui.sideExpanded}>
      <span class="name">{p.name}</span>
      <span class="count">{plural(p.track_ids.length, "titre")}</span>
    </span>
  </a>
{/snippet}

<style>
  .panel {
    position: absolute;
    top: 16px;
    bottom: 16px;
    left: 16px;
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
    transform: translateX(calc(-100% - 24px));
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
  header.collapsed .title {
    flex: 0 0 0;
    padding: 0;
  }
  .title {
    flex: 1;
    padding-left: 4px;
    color: var(--mute);
    font-size: 13px;
    font-weight: 600;
  }
  .chev :global(svg) {
    transition: transform 0.5s;
  }
  .chev.flipped :global(svg) {
    transform: rotate(180deg);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 8px 12px;
  }
  .section-title {
    flex: none;
    padding: 12px 8px 6px;
    white-space: nowrap;
  }
  .item {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px;
    background: transparent;
    color: var(--fg);
    text-align: left;
  }
  .new {
    margin: 0 8px 4px;
  }
  .hl {
    position: absolute;
    inset: 0;
    border-radius: var(--rs);
    background: var(--glass2);
    box-shadow: inset 0 1px 0 var(--hi);
    opacity: 0;
    transition: opacity 0.3s;
    pointer-events: none;
  }
  .hl.on {
    opacity: 1;
  }
  .tile {
    position: relative;
    flex: none;
    width: 40px;
    height: 40px;
    overflow: hidden;
    border-radius: calc(var(--rs) - 2px);
  }
  .dashed {
    display: grid;
    place-items: center;
    box-sizing: border-box;
    border: 1px dashed var(--mute);
    color: var(--accent);
  }
  .mosaic {
    display: grid;
    grid-template: 1fr 1fr / 1fr 1fr;
    background: var(--line);
  }
  .mosaic img {
    width: 100%;
    height: 100%;
    object-fit: cover;
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
    flex-direction: column;
    gap: 2px;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 14px;
    font-weight: 600;
  }
  .count {
    color: var(--mute);
    font-size: 12px;
  }
</style>
