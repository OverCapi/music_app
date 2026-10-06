<script lang="ts">
  import { player } from "$lib/player.svelte";
  import { library } from "$lib/library.svelte";
  import { fileName, formatTime } from "$lib/utils";

  let { queueOpen = $bindable(false) }: { queueOpen?: boolean } = $props();

  // While the user drags the progress bar, show the drag value instead of the polled position.
  let scrubbing = $state(false);
  let scrubValue = $state(0);

  const s = $derived(player.state);
  const track = $derived(library.find(s.path));
  const position = $derived(scrubbing ? scrubValue : s.position);
  const progress = $derived(s.duration > 0 ? (position / s.duration) * 100 : 0);

  function onScrubInput(e: Event) {
    scrubbing = true;
    scrubValue = Number((e.currentTarget as HTMLInputElement).value);
  }

  async function onScrubChange(e: Event) {
    await player.seek(Number((e.currentTarget as HTMLInputElement).value));
    scrubbing = false;
  }
</script>

<footer class="player-bar">
  <div class="now-playing">
    {#if s.path}
      <div class="cover" aria-hidden="true">♪</div>
      <div class="meta">
        <span class="title" title={s.path}>{track?.title ?? fileName(s.path)}</span>
        <span class="artist">{track?.artist ?? "Artiste inconnu"}</span>
      </div>
    {/if}
  </div>

  <div class="center">
    <div class="controls">
      <button
        class="icon"
        onclick={() => player.stop()}
        disabled={!s.loaded}
        aria-label="Stop"
        title="Stop"
      >
        <svg viewBox="0 0 24 24"><rect x="6" y="6" width="12" height="12" rx="1" /></svg>
      </button>
      <button
        class="icon"
        onclick={() => player.previous()}
        disabled={s.current === null}
        aria-label="Précédent"
        title="Précédent"
      >
        <svg viewBox="0 0 24 24"><path d="M6 5h2v14H6zm3.5 7L19 5v14z" /></svg>
      </button>
      <button
        class="icon play"
        onclick={() => player.togglePlay()}
        disabled={player.queue.length === 0}
        aria-label={s.playing ? "Pause" : "Lecture"}
        title={s.playing ? "Pause" : "Lecture"}
      >
        {#if s.playing}
          <svg viewBox="0 0 24 24"><rect x="6" y="5" width="4" height="14" rx="1" /><rect x="14" y="5" width="4" height="14" rx="1" /></svg>
        {:else}
          <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>
        {/if}
      </button>
      <button
        class="icon"
        onclick={() => player.next()}
        disabled={player.upcoming.length === 0}
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
        max={s.duration || 0}
        step="0.1"
        value={position}
        disabled={!s.loaded || s.duration === 0}
        oninput={onScrubInput}
        onchange={onScrubChange}
        style:--fill="{progress}%"
        aria-label="Position"
      />
      <span class="time">{formatTime(s.duration)}</span>
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
