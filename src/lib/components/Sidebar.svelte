<script lang="ts">
  import { tick } from "svelte";
  import { nav } from "$lib/navigation.svelte";
  import { playlists } from "$lib/playlists.svelte";
  import { group } from "$lib/group.svelte";

  const items = [
    {
      kind: "library",
      label: "Bibliothèque",
      icon: "M4 4h2v16H4zm4 0h2v16H8zm4.5.3 1.9-.6 5 15.2-1.9.6z",
    },
    {
      kind: "search",
      label: "Rechercher",
      icon: "M10 3a7 7 0 0 1 5.6 11.2l5.1 5.1-1.4 1.4-5.1-5.1A7 7 0 1 1 10 3zm0 2a5 5 0 1 0 0 10 5 5 0 0 0 0-10z",
    },
    {
      kind: "group",
      label: "Groupe d'écoute",
      icon: "M9 4a4 4 0 1 1 0 8 4 4 0 0 1 0-8zm0 2a2 2 0 1 0 0 4 2 2 0 0 0 0-4zm7.5-1.5a3.5 3.5 0 0 1 0 7v-2a1.5 1.5 0 0 0 0-3zM9 13c3.9 0 7 1.8 7 4.5V20H2v-2.5C2 14.8 5.1 13 9 13zm0 2c-2.9 0-5 1.2-5 2.5v.5h10v-.5c0-1.3-2.1-2.5-5-2.5zm8.2-1.6c2.7.4 4.8 1.9 4.8 4.1V20h-4v-2h2v-.5c0-1-1.1-1.8-2.8-2.2z",
    },
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
    if (playlist) nav.go({ kind: "playlist", id: playlist.id });
  }
</script>

<nav class="sidebar">
  {#each items as item (item.kind)}
    {@const active = nav.view.kind === item.kind}
    <button
      class="nav-item"
      class:active
      onclick={() => nav.go({ kind: item.kind })}
      aria-current={active ? "page" : undefined}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={item.icon} /></svg>
      {item.label}
      {#if item.kind === "group" && group.isActive}
        <span
          class="live"
          class:offline={!group.view?.connected}
          title={group.view?.connected ? "Dans un groupe" : "Reconnexion…"}
        ></span>
      {/if}
    </button>
  {/each}

  <div class="section">
    <h2>Playlists</h2>
    <button class="add" onclick={startCreating} aria-label="Créer une playlist" title="Créer une playlist">
      <svg viewBox="0 0 24 24"><path d="M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6z" /></svg>
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
      {@const active = nav.isPlaylist(playlist.id)}
      <button
        class="playlist"
        class:active
        onclick={() => nav.go({ kind: "playlist", id: playlist.id })}
        aria-current={active ? "page" : undefined}
        title={playlist.name}
      >
        <span class="name">{playlist.name}</span>
        <span class="count">{playlist.tracks.length}</span>
      </button>
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

  button {
    display: flex;
    align-items: center;
    padding: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
    text-align: left;
  }

  button:hover,
  button.active {
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
    background: #f59b23;
    box-shadow: 0 0 0 3px rgb(245 155 35 / 0.25);
  }

  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }

  svg {
    flex: none;
    width: 22px;
    height: 22px;
    fill: currentColor;
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

  .add svg {
    width: 18px;
    height: 18px;
  }

  form {
    padding: 0.25rem 0.25rem;
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
