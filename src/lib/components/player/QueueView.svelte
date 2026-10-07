<script lang="ts">
  import { resolve } from "$app/paths";
  import type { QueueItem } from "#lib/bindings.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { fileName, formatTime } from "#lib/format.ts";

  let { onclose }: { onclose: () => void } = $props();

  const current = $derived(
    player.queue.current !== null ? player.queue.items[player.queue.current] : undefined,
  );

  function info({ track_id, path }: QueueItem) {
    const track = library.byId.get(track_id);
    return {
      title: track?.title ?? fileName(path),
      artist: track?.artist ?? "Artiste inconnu",
      duration: track?.duration,
    };
  }
</script>

<aside class="queue">
  <header>
    <h2>File d'attente</h2>
    <button class="close" onclick={onclose} aria-label="Fermer" title="Fermer">✕</button>
  </header>

  {#if group.isActive || player.state.group_mode}
    <div class="group-note">
      <p>Tu es dans un groupe d'écoute : la lecture suit la file partagée du groupe.</p>
      <a href={resolve("/group")}>Voir la file du groupe</a>
    </div>
  {:else}
    {#if current}
      {@const t = info(current)}
      <h3 class="section-title">En cours de lecture</h3>
      <div class="item current" title={current.path}>
        <span class="meta">
          <span class="title">{t.title}</span>
          <span class="artist">{t.artist}</span>
        </span>
        {#if t.duration !== undefined}
          <span class="duration">{formatTime(t.duration)}</span>
        {/if}
      </div>
    {/if}

    <div class="next-header">
      <h3 class="section-title">Ensuite</h3>
      {#if player.upcoming.length > 0}
        <button class="clear" onclick={() => player.clearQueue()}>Vider</button>
      {/if}
    </div>

    {#if player.upcoming.length === 0}
      <p class="hint">
        {library.tracks.length > 0
          ? "File vide : un titre au hasard de la bibliothèque sera joué ensuite."
          : "Rien à suivre. Ajoute des titres à ta bibliothèque."}
      </p>
    {:else}
      {#each player.upcoming as item (item.index)}
        {@const t = info(item)}
        <div
          class="item upcoming"
          role="button"
          title={item.path}
          tabindex="0"
          ondblclick={() => player.playIndex(item.index)}
          onkeydown={(e) => e.key === "Enter" && player.playIndex(item.index)}
        >
          <span class="meta">
            <span class="title">{t.title}</span>
            <span class="artist">{t.artist}</span>
          </span>
          {#if t.duration !== undefined}
            <span class="duration">{formatTime(t.duration)}</span>
          {/if}
          <button
            class="remove"
            onclick={() => player.removeFromQueue(item.index)}
            aria-label="Retirer de la file"
            title="Retirer de la file"
          >
            ✕
          </button>
        </div>
      {/each}
    {/if}
  {/if}
</aside>

<style>
  .queue {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 1.25rem 1rem;
    overflow-y: auto;
    background: var(--surface);
    border-left: 1px solid var(--surface-2);
  }

  header,
  .next-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  .section-title {
    margin: 1.25rem 0 0.5rem;
  }

  .close,
  .clear,
  .remove {
    padding: 0;
    background: transparent;
    color: var(--muted);
  }

  .close:hover,
  .clear:hover,
  .remove:hover {
    color: var(--text);
  }

  .clear {
    margin-top: 0.75rem;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.45rem 0.5rem;
    border-radius: 4px;
    outline: none;
  }

  .item.upcoming:hover,
  .item.upcoming:focus-visible {
    background: var(--surface-2);
  }

  .meta {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .title,
  .artist {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-size: 0.9rem;
  }

  .current .title {
    color: var(--accent);
  }

  .artist,
  .duration {
    color: var(--muted);
    font-size: 0.75rem;
  }

  .duration {
    font-variant-numeric: tabular-nums;
  }

  .remove {
    width: 16px;
    font-size: 0.75rem;
    visibility: hidden;
  }

  .item.upcoming:hover .remove {
    visibility: visible;
  }

  .group-note {
    margin-top: 1rem;
    padding: 0.75rem;
    border-radius: 6px;
    background: var(--surface-2);
    font-size: 0.85rem;
  }

  .group-note p {
    margin: 0 0 0.5rem;
    color: var(--muted);
  }

  .group-note a {
    color: var(--accent);
    font-weight: 600;
  }

  .group-note a:hover {
    text-decoration: underline;
  }

  .hint {
    margin: 0.5rem 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
</style>
