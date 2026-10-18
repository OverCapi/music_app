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
    border-radius: 16px;
    background: var(--accent-soft);
    box-shadow: var(--shadow-cover);
    color: var(--accent);
  }

  .info {
    min-width: 0;
  }

  .kind {
    margin: 0;
    color: var(--muted);
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .info :global(h1) {
    margin: 0.3rem 0;
    font-size: 2.5rem;
    font-weight: 800;
    letter-spacing: -0.02em;
    line-height: 1.1;
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
    width: 56px;
    height: 56px;
    padding: 0;
    border-radius: 50%;
    background: var(--accent);
    color: var(--on-accent);
  }

  .play:not(:disabled):hover {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }
</style>
