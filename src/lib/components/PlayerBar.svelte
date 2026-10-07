<script lang="ts">
  import { player } from "$lib/player.svelte";
  import { library } from "$lib/library.svelte";
  import { fileName, formatTime } from "$lib/utils";
  import { group } from "$lib/group.svelte";

  let { queueOpen = $bindable(false) }: { queueOpen?: boolean } = $props();

  // While the user drags the progress bar, show the drag value instead of the polled position.
  let scrubbing = $state(false);
  let scrubValue = $state(0);

  const s = $derived(player.state);
  const track = $derived(library.find(s.path));

  // In a listening group the controls drive the shared playback, not the local queue.
  const groupMode = $derived(group.isActive || s.group_mode);
  const entry = $derived(group.currentEntry);
  const groupStatus = $derived(group.playback.status);
  // The engine knows the real position once the group track is loaded locally;
  // before that, estimate it from the shared state (re-evaluated on every poll of `s`).
  const groupLoaded = $derived(s.group_mode && s.loaded && s.duration > 0);
  const duration = $derived(groupMode && !groupLoaded ? (entry?.duration ?? 0) : s.duration);
  const livePosition = $derived.by(() => {
    const polled = s; // read first: re-estimates on every poll
    return groupMode && !groupLoaded ? group.positionAt(Date.now()) : polled.position;
  });
  const playing = $derived(
    groupMode ? groupStatus === "playing" || groupStatus === "preparing" : s.playing,
  );
  const canSeek = $derived(
    groupMode
      ? entry !== undefined && duration > 0 && (groupStatus === "playing" || groupStatus === "paused")
      : s.loaded && s.duration > 0,
  );
  const position = $derived(scrubbing ? scrubValue : Math.min(livePosition, duration || livePosition));
  const progress = $derived(duration > 0 ? (position / duration) * 100 : 0);

  function togglePlay() {
    return groupMode ? group.togglePlay() : player.togglePlay();
  }

  function next() {
    return groupMode ? group.skip() : player.next();
  }

  function previous() {
    return groupMode ? group.seek(0) : player.previous();
  }

  function onScrubInput(e: Event) {
    scrubbing = true;
    scrubValue = Number((e.currentTarget as HTMLInputElement).value);
  }

  async function onScrubChange(e: Event) {
    const secs = Number((e.currentTarget as HTMLInputElement).value);
    await (groupMode ? group.seek(secs) : player.seek(secs));
    scrubbing = false;
  }
</script>

<footer class="player-bar">
  <div class="now-playing">
    {#if groupMode}
      {#if entry}
        <div class="cover" aria-hidden="true">♪</div>
        <div class="meta">
          <span class="title" title={entry.title}>{entry.title}</span>
          <span class="artist">{entry.artist ?? "Artiste inconnu"}</span>
        </div>
      {/if}
      <span class="badge" title="Lecture synchronisée avec le groupe d'écoute">Groupe</span>
    {:else if s.path}
      <div class="cover" aria-hidden="true">♪</div>
      <div class="meta">
        <span class="title" title={s.path}>{track?.title ?? fileName(s.path)}</span>
        <span class="artist">{track?.artist ?? "Artiste inconnu"}</span>
      </div>
    {/if}
  </div>

  <div class="center">
    <div class="controls">
      {#if !groupMode}
        <button
          class="icon"
          onclick={() => player.stop()}
          disabled={!s.loaded}
          aria-label="Stop"
          title="Stop"
        >
          <svg viewBox="0 0 24 24"><rect x="6" y="6" width="12" height="12" rx="1" /></svg>
        </button>
      {/if}
      <button
        class="icon"
        onclick={previous}
        disabled={groupMode ? !canSeek : s.current === null}
        aria-label={groupMode ? "Revenir au début" : "Précédent"}
        title={groupMode ? "Revenir au début" : "Précédent"}
      >
        <svg viewBox="0 0 24 24"><path d="M6 5h2v14H6zm3.5 7L19 5v14z" /></svg>
      </button>
      <button
        class="icon play"
        onclick={togglePlay}
        disabled={groupMode
          ? (group.state?.queue.length ?? 0) === 0
          : player.queue.length === 0 && library.tracks.length === 0}
        aria-label={playing ? "Pause" : "Lecture"}
        title={playing ? "Pause" : "Lecture"}
      >
        {#if playing}
          <svg viewBox="0 0 24 24"><rect x="6" y="5" width="4" height="14" rx="1" /><rect x="14" y="5" width="4" height="14" rx="1" /></svg>
        {:else}
          <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>
        {/if}
      </button>
      <button
        class="icon"
        onclick={next}
        disabled={groupMode
          ? entry === undefined
          : player.upcoming.length === 0 && library.tracks.length === 0}
        aria-label="Suivant"
        title="Suivant"
      >
        <svg viewBox="0 0 24 24"><path d="M16 5h2v14h-2zM5 5l9.5 7L5 19z" /></svg>
      </button>
    </div>

    <div class="progress">
      <span class="time">{formatTime(position)}</span>
      <input
        type="range"
        min="0"
        max={duration || 0}
        step="0.1"
        value={position}
        disabled={!canSeek}
        oninput={onScrubInput}
        onchange={onScrubChange}
        style:--fill="{progress}%"
        aria-label="Position"
      />
      <span class="time">{formatTime(duration)}</span>
    </div>
  </div>

  <div class="volume">
    <button
      class="icon"
      class:active={queueOpen}
      onclick={() => (queueOpen = !queueOpen)}
      aria-label="File d'attente"
      aria-pressed={queueOpen}
      title="File d'attente"
    >
      <svg viewBox="0 0 24 24"><path d="M3 6h12v2H3zm0 5h12v2H3zm0 5h8v2H3zm13-3v8l6-4z" /></svg>
    </button>
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9v6h4l5 4V5L8 9z" /></svg>
    <input
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={s.volume}
      oninput={(e) => player.setVolume(Number(e.currentTarget.value))}
      style:--fill="{s.volume * 100}%"
      aria-label="Volume"
    />
  </div>
</footer>

<style>
  .player-bar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 2fr) minmax(0, 1fr);
    align-items: center;
    gap: 1.5rem;
    padding: 0.75rem 1rem;
    background: var(--surface);
    border-top: 1px solid var(--surface-2);
  }

  .now-playing {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
  }

  .badge {
    flex: none;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    background: var(--accent);
    color: #000;
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .cover {
    display: grid;
    flex: none;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 4px;
    background: linear-gradient(135deg, #2a2a2a, #3e3e3e);
    color: var(--muted);
    font-size: 1.4rem;
  }

  .meta {
    display: flex;
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

  .artist {
    color: var(--muted);
    font-size: 0.75rem;
  }

  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--muted);
  }

  .icon:not(:disabled):hover {
    color: var(--text);
  }

  .icon.active {
    color: var(--accent);
  }

  .icon.active:hover {
    color: var(--accent);
  }

  .icon.play {
    width: 36px;
    height: 36px;
    background: var(--text);
    color: #000;
  }

  .icon.play:not(:disabled):hover {
    transform: scale(1.06);
    color: #000;
  }

  svg {
    width: 20px;
    height: 20px;
    fill: currentColor;
  }

  .progress,
  .volume {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
  }

  .volume {
    justify-content: flex-end;
    color: var(--muted);
  }

  .volume input {
    flex: 0 1 120px;
  }

  .time {
    min-width: 3ch;
    color: var(--muted);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
</style>
