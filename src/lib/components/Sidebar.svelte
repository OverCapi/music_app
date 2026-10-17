<script lang="ts">
  import { tick } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { playlists } from "#lib/stores/playlists.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  const items = [
    { href: resolve("/"), label: "Bibliothèque", icon: "library" },
    { href: resolve("/search"), label: "Rechercher", icon: "search" },
    { href: resolve("/group"), label: "Groupe d'écoute", icon: "group" },
    { href: resolve("/sync"), label: "Synchronisation", icon: "sync" },
  ] as const;

  let creating = $state(false);
  let newName = $state("");
  let input = $state<HTMLInputElement>();

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

<nav class="sidebar">
  {#each items as item (item.href)}
    {@const active = page.url.pathname === item.href}
    <a class="nav-item" class:active href={item.href} aria-current={active ? "page" : undefined}>
      <Icon name={item.icon} size={22} />
      {item.label}
      {#if item.icon === "group" && group.isActive}
        <span
          class="live"
          class:offline={!group.view?.connected}
          title={group.view?.connected ? "Dans un groupe" : "Reconnexion…"}
        ></span>
      {/if}
    </a>
  {/each}

  <div class="section">
    <h2>Playlists</h2>
    <button
      class="add"
      onclick={startCreating}
      aria-label="Créer une playlist"
      title="Créer une playlist"
    >
      <Icon name="plus" size={18} />
    </button>
  </div>

  {#if creating}
    <form onsubmit={create}>
      <input
        bind:this={input}
        bind:value={newName}
        placeholder="Nom de la playlist"
        maxlength="100"
        onblur={() => !newName.trim() && (creating = false)}
        onkeydown={(e) => e.key === "Escape" && (creating = false)}
      />
    </form>
  {/if}

  <div class="playlists">
    {#each playlists.items as playlist (playlist.id)}
      {@const href = resolve("/playlist/[id]", { id: String(playlist.id) })}
      {@const active = page.url.pathname === href}
      <a
        class="playlist"
        class:active
        {href}
        aria-current={active ? "page" : undefined}
        title={playlist.name}
      >
        <span class="name">{playlist.name}</span>
        <span class="count">{playlist.track_ids.length}</span>
      </a>
    {:else}
      {#if !creating}
        <p class="hint">Crée ta première playlist avec le bouton +.</p>
      {/if}
    {/each}
  </div>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-height: 0;
    padding: 1.25rem 0.75rem;
    background: #000;
  }

  a,
  button {
    display: flex;
    align-items: center;
    padding: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
    text-align: left;
  }

  a:hover,
  button:hover,
  a.active {
    color: var(--text);
  }

  .nav-item {
    gap: 0.9rem;
    padding: 0.6rem 0.75rem;
    font-weight: 700;
    font-size: 0.9rem;
  }

  .nav-item.active,
  .playlist.active {
    background: var(--surface-2);
  }

  .live {
    width: 8px;
    height: 8px;
    margin-left: auto;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px rgb(29 185 84 / 0.25);
    animation: pulse 1.6s ease-in-out infinite;
  }

  .live.offline {
    background: var(--warning);
    box-shadow: 0 0 0 3px rgb(245 155 35 / 0.25);
  }

  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }

  .section {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 1.25rem 0 0.25rem;
    padding: 0 0.75rem;
  }

  h2 {
    margin: 0;
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .add {
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
  }

  .add:hover {
    background: var(--surface-2);
  }

  form {
    padding: 0.25rem;
  }

  input {
    width: 100%;
    box-sizing: border-box;
    padding: 0.45rem 0.6rem;
    border: 1px solid var(--muted);
    border-radius: 4px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 0.85rem;
    outline: none;
  }

  .playlists {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-height: 0;
    overflow-y: auto;
  }

  .playlist {
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    flex: none;
    color: var(--muted);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }

  .hint {
    margin: 0.25rem 0.75rem;
    color: var(--muted);
    font-size: 0.8rem;
  }
</style>
