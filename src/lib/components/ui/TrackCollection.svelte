<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon, { type IconName } from "#lib/components/ui/Icon.svelte";

  let {
    kind,
    name,
    meta,
    icon,
    playLabel,
    playDisabled = false,
    onplay,
    title,
    toolbar,
    children,
  }: {
    /** Small label above the name ("Playlist", "Bibliothèque"). */
    kind: string;
    name: string;
    meta: string;
    icon: IconName;
    playLabel: string;
    playDisabled?: boolean;
    onplay: () => void;
    /** Replaces the default `<h1>{name}</h1>` (e.g. an inline rename form). */
    title?: Snippet;
    /** Extra controls next to the play button. */
    toolbar?: Snippet;
    /** Content under the toolbar: usually a `TrackTable` or an empty message. */
    children: Snippet;
  } = $props();
</script>

<header>
  <div class="cover" aria-hidden="true"><Icon name={icon} size={64} /></div>
  <div class="info">
    <p class="kind">{kind}</p>
    {#if title}
      {@render title()}
    {:else}
      <h1>{name}</h1>
    {/if}
    <p class="meta">{meta}</p>
  </div>
</header>

<div class="toolbar">
  <button
    class="play"
    onclick={onplay}
    disabled={playDisabled}
    aria-label={playLabel}
    title={playLabel}
  >
    <Icon name="play" size={26} />
  </button>
  {@render toolbar?.()}
</div>

{@render children()}

<style>
  header {
    display: flex;
    align-items: flex-end;
    gap: 1.5rem;
    margin-bottom: 1.5rem;
  }

  .cover {
    display: grid;
    flex: none;
    place-items: center;
    width: 160px;
    height: 160px;
    border-radius: 6px;
    background: linear-gradient(135deg, #450af5, #8e8ee5);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.5);
    color: rgb(255 255 255 / 0.85);
  }

  .info {
    min-width: 0;
  }

  .kind {
    margin: 0;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .info :global(h1) {
    margin: 0.3rem 0;
    font-size: 2.5rem;
  }

  .meta {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .play {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    padding: 0;
    border-radius: 50%;
    background: var(--accent);
    color: #000;
  }

  .play:not(:disabled):hover {
    transform: scale(1.05);
  }
</style>
