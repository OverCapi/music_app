<script lang="ts">
  import { resolve } from "$app/paths";
  import type { QueueItem } from "#lib/bindings.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { fileName, formatTime } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  let { onclose }: { onclose: () => void } = $props();

  const current = $derived(
    player.queue.current !== null ? player.queue.items[player.queue.current] : undefined,
  );

  function info({ track_id, path }: QueueItem) {
    const track = library.byId.get(track_id);
    return {
      title: track?.title ?? (path ? fileName(path) : "Titre inconnu"),
      artist: track?.artist ?? "Artiste inconnu",
      duration: track?.duration,
    };
  }
</script>

<aside class="queue">
  <header>
    <h2>File d'attente</h2>
    <button class="icon-btn" onclick={onclose} aria-label="Fermer" title="Fermer">
      <Icon name="close" />
    </button>
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
            <Icon name="close" size={16} />
          </button>
        </div>
      {/each}
    {/if}
  {/if}
</aside>

<style>
  /* Slide-over panel floating above the page, over the dock's top edge. */
  .queue {
    position: absolute;
    top: 16px;
    right: 16px;
    bottom: calc(var(--dock-height) + 32px);
    z-index: 2;
    display: flex;
    flex-direction: column;
    width: 360px;
    box-sizing: border-box;
    padding: 20px;
    overflow-y: auto;
    border-radius: 16px;
    background: var(--surface);
    box-shadow: var(--shadow-float);
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
    font-weight: 700;
  }

  .section-title {
    margin: 1.25rem 0 0.5rem;
  }

  .clear,
  .remove {
    padding: 0;
    background: transparent;
    color: var(--muted);
  }

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
    padding: 0.5rem 0.6rem;
    border-radius: 12px;
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

  .item.current {
    background: var(--accent-soft);
  }

  .current .title {
    font-weight: 700;
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
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    visibility: hidden;
  }

  .item.upcoming:hover .remove {
    visibility: visible;
  }

  .group-note {
    margin-top: 1rem;
    padding: 0.75rem;
    border-radius: 12px;
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
