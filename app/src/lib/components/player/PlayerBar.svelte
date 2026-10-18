<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { player } from "#lib/stores/player.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { lives } from "#lib/stores/lives.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { fileName, formatTime } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import FeedbackButtons from "#lib/components/player/FeedbackButtons.svelte";
  import Slider from "#lib/components/ui/Slider.svelte";

  let { queueOpen = $bindable(false) }: { queueOpen?: boolean } = $props();

  // While the user drags the progress bar, show the drag value instead of the live position.
  let scrubbing = $state(false);
  let scrubValue = $state(0);

  const s = $derived(player.state);
  // A live plays outside the queue: no progress bar, no seeking.
  const live = $derived(s.group_mode ? null : s.live);
  const track = $derived(s.track_id !== null ? library.byId.get(s.track_id) : undefined);

  // In a listening group the controls drive the shared playback, not the local queue.
  const groupMode = $derived(group.isActive || s.group_mode);
  const entry = $derived(group.currentEntry);
  const groupStatus = $derived(group.playback.status);
  const playing = $derived(
    groupMode ? groupStatus === "playing" || groupStatus === "preparing" : s.playing,
  );

  // Ticks while playing, to move the progress bar between engine events.
  let now = $state(Date.now());
  $effect(() => {
    if (!playing) return;
    const timer = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(timer);
  });

  // The engine knows the real position once the group track is loaded locally;
  // before that, estimate it from the shared state.
  const groupLoaded = $derived(s.group_mode && s.loaded && player.duration > 0);
  const duration = $derived(groupMode && !groupLoaded ? (entry?.duration ?? 0) : player.duration);
  const livePosition = $derived(
    groupMode && !groupLoaded ? group.positionAt(now) : player.positionAt(now),
  );
  const canSeek = $derived(
    groupMode
      ? entry !== undefined &&
          duration > 0 &&
          (groupStatus === "playing" || groupStatus === "paused")
      : s.loaded && duration > 0,
  );
  const position = $derived(
    scrubbing ? scrubValue : Math.min(livePosition, duration || livePosition),
  );

  // Track targeted by the like / "never suggest" buttons.
  const feedbackTrack = $derived(groupMode ? entry : track);
  const feedbackArtist = $derived(groupMode ? entry?.artist : track?.artist);

  const coverSrc = $derived(
    groupMode
      ? entry && `https://i.ytimg.com/vi/${entry.youtube_id}/mqdefault.jpg`
      : s.track_id !== null
        ? convertFileSrc(String(s.track_id), "cover")
        : undefined,
  );

  const repeatLabel = $derived(
    { off: "Répéter : désactivé", all: "Répéter la file", one: "Répéter le titre" }[s.repeat],
  );

  function togglePlay() {
    return groupMode ? group.togglePlay() : player.togglePlay();
  }

  function next() {
    return groupMode ? group.skip() : player.next();
  }

  function previous() {
    return groupMode ? group.seek(0) : player.previous();
  }

  function scrub(secs: number) {
    scrubbing = true;
    scrubValue = secs;
  }

  async function seek(secs: number) {
    await (groupMode ? group.seek(secs) : player.seek(secs));
    scrubbing = false;
  }
</script>

<footer class="player-bar">
  <div class="now-playing">
    {#if groupMode}
      {#if entry}
        <div class="cover" aria-hidden="true">
          {#key coverSrc}
            {#if coverSrc}
              <img
                src={coverSrc}
                alt=""
                onerror={(e) => ((e.currentTarget as HTMLElement).style.display = "none")}
              />
            {/if}
          {/key}
          <Icon name="note" size={24} />
        </div>
        <div class="meta">
          <span class="title" title={entry.title}>{entry.title}</span>
          <span class="artist">{entry.artist ?? "Artiste inconnu"}</span>
        </div>
      {/if}
      <FeedbackButtons track={feedbackTrack} artist={feedbackArtist} />
      <span class="badge" title="Lecture synchronisée avec le groupe d'écoute">Groupe</span>
    {:else if live}
      <div class="cover" aria-hidden="true">
        <img
          src={live.thumbnail}
          alt=""
          onerror={(e) => ((e.currentTarget as HTMLElement).style.display = "none")}
        />
        <Icon name="live" size={24} />
      </div>
      <div class="meta">
        <span class="title" title={live.title}>{live.title}</span>
        <span class="artist">{live.channel ?? "Chaîne inconnue"}</span>
      </div>
      <span class="badge live" title="Direct YouTube">En direct</span>
    {:else if s.path || s.loading}
      <div class="cover" aria-hidden="true">
        {#key coverSrc}
          {#if coverSrc}
            <img
              src={coverSrc}
              alt=""
              onerror={(e) => ((e.currentTarget as HTMLElement).style.display = "none")}
            />
          {/if}
        {/key}
        <Icon name="note" size={24} />
      </div>
      <div class="meta">
        <span class="title" title={s.path ?? undefined}
          >{track?.title ?? (s.path ? fileName(s.path) : "")}</span
        >
        <span class="artist"
          >{s.loading ? "Téléchargement…" : (track?.artist ?? "Artiste inconnu")}</span
        >
      </div>
      <FeedbackButtons track={feedbackTrack} artist={feedbackArtist} />
    {/if}
  </div>

  <div class="center">
    <div class="controls">
      {#if !groupMode}
        <button
          class="icon"
          class:active={s.shuffle}
          onclick={() => player.toggleShuffle()}
          aria-label="Lecture aléatoire"
          aria-pressed={s.shuffle}
          title="Lecture aléatoire"
        >
          <Icon name="shuffle" />
        </button>
        <button
          class="icon"
          onclick={() => player.stop()}
          disabled={!s.loaded}
          aria-label="Stop"
          title="Stop"
        >
          <Icon name="stop" />
        </button>
      {/if}
      <button
        class="icon"
        onclick={previous}
        disabled={groupMode ? !canSeek : s.current === null || live !== null}
        aria-label={groupMode ? "Revenir au début" : "Précédent"}
        title={groupMode ? "Revenir au début" : "Précédent"}
      >
        <Icon name="previous" />
      </button>
      <button
        class="icon play"
        onclick={togglePlay}
        disabled={groupMode
          ? (group.state?.queue.length ?? 0) === 0
          : live === null && player.queue.items.length === 0 && library.tracks.length === 0}
        aria-label={playing ? "Pause" : "Lecture"}
        title={playing ? "Pause" : "Lecture"}
      >
        <Icon name={playing ? "pause" : "play"} />
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
        <Icon name="next" />
      </button>
      {#if !groupMode}
        <button
          class="icon"
          class:active={s.repeat !== "off"}
          onclick={() => player.cycleRepeat()}
          aria-label={repeatLabel}
          title={repeatLabel}
        >
          <Icon name={s.repeat === "one" ? "repeatOne" : "repeat"} />
        </button>
      {/if}
    </div>

    {#if !live}
      <div class="progress">
        <span class="time">{formatTime(position)}</span>
        <Slider
          value={position}
          max={duration}
          step={0.1}
          disabled={!canSeek}
          oninput={scrub}
          onchange={seek}
          label="Position"
        />
        <span class="time">{formatTime(duration)}</span>
      </div>
    {/if}
  </div>

  <div class="volume">
    {#if live}
      <button
        class="icon"
        class:active={lives.videoOpen}
        onclick={() => (lives.videoOpen = !lives.videoOpen)}
        aria-label="Voir la vidéo"
        aria-pressed={lives.videoOpen}
        title="Voir la vidéo"
      >
        <Icon name="expand" />
      </button>
    {/if}
    <button
      class="icon"
      class:active={queueOpen}
      onclick={() => (queueOpen = !queueOpen)}
      aria-label="File d'attente"
      aria-pressed={queueOpen}
      title="File d'attente"
    >
      <Icon name="queue" />
    </button>
    <Icon name="volume" />
    <div class="volume-slider">
      <Slider
        value={player.volume}
        max={1}
        step={0.01}
        oninput={(v) => player.setVolume(v)}
        label="Volume"
      />
    </div>
  </div>
</footer>

<style>
  /* Floating dock, right of the rail (16px inset + 72px rail + 16px gap). */
  .player-bar {
    position: absolute;
    right: 16px;
    bottom: 16px;
    left: 104px;
    z-index: 3;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 2fr) minmax(0, 1fr);
    align-items: center;
    gap: 24px;
    height: var(--dock-height);
    box-sizing: border-box;
    padding: 12px 20px;
    border-radius: 24px;
    background: var(--surface);
    box-shadow: var(--shadow-float);
  }

  .now-playing {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .badge {
    flex: none;
    padding: 0.2rem 0.6rem;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--text);
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .badge.live {
    background: #e5252a;
    color: #fff;
  }

  .cover {
    position: relative;
    display: grid;
    flex: none;
    place-items: center;
    width: 56px;
    height: 56px;
    overflow: hidden;
    border-radius: 12px;
    background: var(--surface-3);
    box-shadow: var(--shadow-cover);
    color: var(--muted);
    font-size: 1.4rem;
  }

  .cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    background: var(--surface-3);
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
    font-size: 0.95rem;
    font-weight: 700;
  }

  .artist {
    color: var(--muted);
    font-size: 0.85rem;
  }

  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--muted);
  }

  .icon:not(:disabled):hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .icon.active,
  .icon.active:hover {
    color: var(--accent);
  }

  .icon.play,
  .icon.play:not(:disabled):hover {
    width: 48px;
    height: 48px;
    background: var(--accent);
    color: var(--on-accent);
  }

  .icon.play:not(:disabled):hover {
    filter: brightness(1.08);
  }

  .progress,
  .volume {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
  }

  .volume {
    justify-content: flex-end;
    gap: 4px;
    color: var(--muted);
  }

  .volume-slider {
    display: flex;
    flex: 0 1 110px;
    margin-left: 6px;
  }

  .time {
    min-width: 3ch;
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
</style>
