<script lang="ts">
  import { tick } from "svelte";
  import { playlists } from "$lib/playlists.svelte";

  let { path, exclude }: { path: string; /** Playlist to leave out (the one being viewed). */ exclude?: string } =
    $props();

  let open = $state(false);
  let creating = $state(false);
  let newName = $state("");
  let pos = $state({ top: 0, left: 0 });
  let button: HTMLButtonElement;
  let menu = $state<HTMLDivElement>();
  let input = $state<HTMLInputElement>();

  const MENU_WIDTH = 220;
  const choices = $derived(playlists.items.filter((p) => p.id !== exclude));

  function toggle() {
    if (open) return close();
    // Fixed position, so the scrolling list can't clip the menu.
    const rect = button.getBoundingClientRect();
    const below = rect.bottom + 4;
    pos = {
      top: below + 300 > window.innerHeight ? Math.max(8, rect.top - 4 - 300) : below,
      left: Math.max(8, rect.right - MENU_WIDTH),
    };
    open = true;
  }

  function close() {
    open = false;
    creating = false;
    newName = "";
  }

  async function add(id: string) {
    close();
    await playlists.addTracks(id, [path]);
  }

  async function startCreating() {
    creating = true;
    await tick();
    input?.focus();
  }

  async function createAndAdd(e: SubmitEvent) {
    e.preventDefault();
    const created = await playlists.create(newName);
    if (created) await add(created.id);
  }

  function onWindowClick(e: MouseEvent) {
    const target = e.target as Node;
    if (open && !menu?.contains(target) && !button.contains(target)) close();
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => open && e.key === "Escape" && close()} />

<button
  bind:this={button}
  class="row-action"
  class:open
  onclick={toggle}
  aria-label="Ajouter à une playlist"
  aria-haspopup="menu"
  aria-expanded={open}
  title="Ajouter à une playlist"
>
  <svg viewBox="0 0 24 24"><path d="M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6z" /></svg>
</button>

{#if open}
  <div
    bind:this={menu}
    class="menu"
    role="menu"
    tabindex="-1"
    onkeydown={(e) => e.key !== "Escape" && e.stopPropagation()}
    ondblclick={(e) => e.stopPropagation()}
    style:top="{pos.top}px"
    style:left="{pos.left}px"
    style:width="{MENU_WIDTH}px"
  >
    <p class="label">Ajouter à une playlist</p>
    {#if creating}
      <form onsubmit={createAndAdd}>
        <input bind:this={input} bind:value={newName} placeholder="Nom de la playlist" maxlength="100" />
      </form>
    {:else}
      <button class="item new" role="menuitem" onclick={startCreating}>+ Nouvelle playlist</button>
    {/if}
    {#if choices.length > 0}
      <div class="separator"></div>
      <div class="list">
        {#each choices as playlist (playlist.id)}
          {@const already = playlist.tracks.includes(path)}
          <button
            class="item"
            role="menuitem"
            disabled={already}
            onclick={() => add(playlist.id)}
            title={already ? "Déjà dans cette playlist" : undefined}
          >
            <span>{playlist.name}</span>
            {#if already}<span class="check">✓</span>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 10;
    display: flex;
    flex-direction: column;
    max-height: 300px;
    padding: 0.35rem;
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.6);
    cursor: default;
  }

  .label {
    margin: 0.35rem 0.6rem 0.4rem;
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
  }

  .list {
    overflow-y: auto;
  }

  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 0.6rem;
    border-radius: 3px;
    background: transparent;
    color: var(--text);
    font-size: 0.85rem;
    text-align: left;
  }

  .item span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item:not(:disabled):hover {
    background: #3e3e3e;
  }

  .item:disabled {
    opacity: 0.6;
  }

  .new {
    font-weight: 600;
  }

  .check {
    color: var(--accent);
  }

  .separator {
    height: 1px;
    margin: 0.3rem 0;
    background: #3e3e3e;
  }

  input {
    width: 100%;
    box-sizing: border-box;
    padding: 0.45rem 0.6rem;
    border: 1px solid var(--muted);
    border-radius: 3px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 0.85rem;
    outline: none;
  }
</style>
