<script lang="ts">
  import { tick } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { playlists } from "#lib/stores/playlists.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  // Search and settings live in the top bar, so they are not rail items.
  const items = [
    { href: resolve("/"), label: "Accueil", icon: "home" },
    { href: resolve("/library"), label: "Bibliothèque", icon: "library" },
    { href: resolve("/lives"), label: "Lives", icon: "live" },
    { href: resolve("/group"), label: "Groupe d'écoute", icon: "group" },
  ] as const;

  let creating = $state(false);
  let newName = $state("");
  let input = $state<HTMLInputElement>();

  /** Two-letter monogram of a playlist name. */
  function initials(name: string) {
    const words = name.trim().split(/\s+/);
    return (words.length > 1 ? words[0][0] + words[1][0] : name.slice(0, 2)).toUpperCase();
  }

  /** Stable tile hue per playlist; dark tile under light text, readable in every theme. */
  function tileColor(id: number) {
    return `hsl(${(id * 67) % 360} 38% 32%)`;
  }

  async function startCreating() {
    creating = true;
    newName = "";
    await tick();
    input?.focus();
  }

  async function create(e: SubmitEvent) {
    e.preventDefault();
    if (!newName.trim()) return;
    const playlist = await playlists.create(newName);
    creating = false;
    if (playlist) goto(resolve("/playlist/[id]", { id: String(playlist.id) }));
  }
</script>

<nav class="rail" aria-label="Navigation principale">
  {#each items as item (item.href)}
    {@const active = page.url.pathname === item.href}
    <a
      class="item"
      class:active
      href={item.href}
      aria-label={item.label}
      aria-current={active ? "page" : undefined}
    >
      <Icon name={item.icon} size={24} />
      <span class="tip">{item.label}</span>
      {#if item.icon === "group" && group.isActive}
        <span
          class="live"
          class:offline={!group.view?.connected}
          title={group.view?.connected ? "Dans un groupe" : "Reconnexion…"}
        ></span>
      {/if}
    </a>
  {/each}

  <div class="divider"></div>

  <div class="playlists">
    {#each playlists.items as playlist (playlist.id)}
      {@const href = resolve("/playlist/[id]", { id: String(playlist.id) })}
      {@const active = page.url.pathname === href}
      <a
        class="tile"
        class:active
        {href}
        style:background={tileColor(playlist.id)}
        aria-label={playlist.name}
        aria-current={active ? "page" : undefined}
        title={`${playlist.name} · ${playlist.track_ids.length}`}
      >
        {initials(playlist.name)}
      </a>
    {/each}
  </div>

  <div class="create">
    <button
      class="add"
      onclick={startCreating}
      aria-label="Créer une playlist"
      title="Créer une playlist"
    >
      <Icon name="plus" size={20} />
    </button>
    {#if creating}
      <form onsubmit={create}>
        <input
          class="field"
          bind:this={input}
          bind:value={newName}
          placeholder="Nom de la playlist"
          maxlength="100"
          onblur={() => !newName.trim() && (creating = false)}
          onkeydown={(e) => e.key === "Escape" && (creating = false)}
        />
      </form>
    {/if}
  </div>
</nav>

<style>
  .rail {
    display: flex;
    flex: none;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 72px;
    min-height: 0;
    box-sizing: border-box;
    padding: 16px 0;
    border-radius: 24px;
    background: var(--surface);
    box-shadow: var(--shadow-float);
  }

  .item,
  .tile,
  .add {
    position: relative;
    display: grid;
    flex: none;
    place-items: center;
    width: 48px;
    height: 48px;
    padding: 0;
    border-radius: 12px;
  }

  .item {
    color: var(--muted);
  }

  .item:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .item.active {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .tip {
    position: absolute;
    left: 60px;
    top: 50%;
    z-index: 10;
    padding: 6px 10px;
    border-radius: 8px;
    background: var(--surface-2);
    box-shadow: var(--shadow-float);
    color: var(--text);
    font-size: 0.8rem;
    font-weight: 600;
    white-space: nowrap;
    opacity: 0;
    pointer-events: none;
    transform: translateY(-50%);
    transition: opacity 0.12s;
  }

  .item:hover .tip,
  .item:focus-visible .tip {
    opacity: 1;
  }

  .live {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--coral);
    animation: pulse 1.6s ease-in-out infinite;
  }

  .live.offline {
    background: var(--warning);
  }

  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }

  .divider {
    flex: none;
    width: 32px;
    height: 1px;
    margin: 8px 0;
    background: var(--line);
  }

  .playlists {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    min-height: 0;
    /* Room for the active ring, which overflow would clip. */
    padding: 3px;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .tile {
    color: #fff;
    font-size: 0.8rem;
    font-weight: 800;
    transition: box-shadow 0.12s;
  }

  .tile:hover,
  .tile.active {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .create {
    position: relative;
  }

  .add {
    border: 1px dashed var(--track);
    background: transparent;
    color: var(--muted);
  }

  .add:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  form {
    position: absolute;
    left: 60px;
    top: 0;
    z-index: 10;
    padding: 8px;
    border-radius: 12px;
    background: var(--surface);
    box-shadow: var(--shadow-float);
  }

  input {
    width: 220px;
  }
</style>
