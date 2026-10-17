<script lang="ts">
  import { feedback } from "#lib/stores/feedback.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  let {
    track,
    artist,
  }: {
    /** Current track; nothing is rendered without a YouTube id. */
    track: { youtube_id: string | null; title: string } | undefined;
    artist: string | null | undefined;
  } = $props();

  const youtubeId = $derived(track?.youtube_id ?? null);
  const artistName = $derived(artist?.trim() ? artist.trim() : null);

  const liked = $derived(youtubeId !== null && feedback.trackKind(youtubeId) === "like");
  const trackDisliked = $derived(youtubeId !== null && feedback.trackKind(youtubeId) === "dislike");
  const artistDisliked = $derived(
    artistName !== null && feedback.artistKind(artistName) === "dislike",
  );
  const disliked = $derived(trackDisliked || artistDisliked);

  let choiceOpen = $state(false);
  let root = $state<HTMLElement>();

  // Close the choice when the track changes.
  $effect(() => {
    void youtubeId;
    choiceOpen = false;
  });

  function current() {
    return youtubeId !== null && track ? { youtube_id: youtubeId, title: track.title } : undefined;
  }

  function like() {
    const t = current();
    if (t) feedback.toggleLike(t);
  }

  function banClick() {
    const t = current();
    if (!t) return;
    if (disliked) feedback.clearDislikes(t, artistName);
    else choiceOpen = !choiceOpen;
  }

  function banTrack() {
    const t = current();
    choiceOpen = false;
    if (t) feedback.toggleTrackDislike(t);
  }

  function banArtist() {
    choiceOpen = false;
    if (artistName) feedback.toggleArtistDislike(artistName);
  }

  function onWindowClick(e: MouseEvent) {
    if (choiceOpen && root && !root.contains(e.target as Node)) choiceOpen = false;
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") choiceOpen = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKeydown} />

{#if youtubeId !== null}
  <div class="feedback" bind:this={root}>
    <button
      class="icon"
      class:active={liked}
      onclick={like}
      aria-pressed={liked}
      aria-label={liked ? "Retirer des titres aimés" : "J'aime"}
      title={liked ? "Retirer des titres aimés" : "J'aime"}
    >
      <Icon name={liked ? "heartFilled" : "heart"} size={18} />
    </button>
    <button
      class="icon"
      class:active={disliked}
      onclick={banClick}
      aria-pressed={disliked}
      aria-expanded={disliked ? undefined : choiceOpen}
      aria-label="Ne plus proposer"
      title="Ne plus proposer"
    >
      <Icon name="ban" size={18} />
    </button>
    {#if choiceOpen}
      <div class="choice" role="group" aria-label="Ne plus proposer">
        <button class="option" onclick={banTrack}>Ce titre</button>
        {#if artistName}
          <button class="option" onclick={banArtist}>Cet artiste</button>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .feedback {
    position: relative;
    display: flex;
    flex: none;
    align-items: center;
    gap: 0.1rem;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--muted);
  }

  .icon:hover {
    color: var(--text);
  }

  .icon.active,
  .icon.active:hover {
    color: var(--accent);
  }

  .choice {
    position: absolute;
    bottom: calc(100% + 0.4rem);
    left: 0;
    z-index: 10;
    display: flex;
    gap: 0.25rem;
    padding: 0.3rem;
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.5);
    white-space: nowrap;
  }

  .option {
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    font-size: 0.8rem;
  }

  .option:hover {
    background: var(--surface);
  }
</style>
