<script lang="ts">
  /** Floating dock between the two panels (max 1000 px), shrinks with them.
   * Left: spinning disc cover + title (+ DIRECT / GROUPE badges). Center: shuffle · prev ·
   * play · next · repeat, then a 4 px progress bar (hidden for lives). Right: mini equalizer ·
   * like · volume (widens on hover, draggable) · fullscreen.
   * Group / live logic is the same as the current PlayerBar — only the presentation changes. */
  import { player } from "#lib/stores/player.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { lives } from "#lib/stores/lives.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { feedback } from "#lib/stores/feedback.svelte.ts";
  import { ui } from "#lib/stores/ui.svelte.ts";
  import { fileName, formatTime } from "#lib/format.ts";
  import { ticker } from "#lib/clock.svelte.ts";
  import { localCover, youtubeCover } from "#lib/covers.ts";
  import Badge from "#lib/components/ui/Badge.svelte";
  import Disc from "#lib/components/ui/Disc.svelte";
  import Icon from "#lib/components/ui/Icon.svelte";
  import IconButton from "#lib/components/ui/IconButton.svelte";
  import PlayButton from "#lib/components/ui/PlayButton.svelte";
  import RailBar from "#lib/components/ui/RailBar.svelte";
  import Visualizer from "#lib/components/ui/Visualizer.svelte";

  const s = $derived(player.state);
  const live = $derived(s.group_mode ? null : s.live);
  const track = $derived(s.track_id !== null ? library.byId.get(s.track_id) : undefined);
  const groupMode = $derived(group.isActive || s.group_mode);
  const entry = $derived(group.currentEntry);
  const playing = $derived(
    groupMode ? ["playing", "preparing"].includes(group.playback.status) : s.playing,
  );

  const clock = ticker(() => playing);

  const duration = $derived(
    groupMode && !(s.group_mode && s.loaded) ? (entry?.duration ?? 0) : player.duration,
  );
  const position = $derived(
    Math.min(
      groupMode && !(s.group_mode && s.loaded)
        ? group.positionAt(clock.now)
        : player.positionAt(clock.now),
      duration || Infinity,
    ),
  );
  const title = $derived(
    groupMode
      ? entry?.title
      : live
        ? live.title
        : (track?.title ?? (s.path ? fileName(s.path) : "")),
  );
  const artist = $derived(
    groupMode ? entry?.artist : live ? live.channel : s.loading ? "Téléchargement…" : track?.artist,
  );
  const cover = $derived(
    groupMode
      ? entry && youtubeCover(entry.youtube_id)
      : live
        ? live.thumbnail
        : s.track_id !== null
          ? localCover(s.track_id)
          : undefined,
  );
  const ytId = $derived(groupMode ? entry?.youtube_id : track?.youtube_id);
  const liked = $derived(!!ytId && feedback.trackKind(ytId) === "like");

  function seek(fraction: number) {
    const secs = fraction * duration;
    return groupMode ? group.seek(secs) : player.seek(secs);
  }
</script>

<div class="wrap" style:left="{ui.left}px" style:right="{ui.right}px">
  <footer class="dock glass" class:compact={ui.compact}>
    <button class="now" onclick={() => (live ? lives.expandVideo() : (ui.fullscreen = true))}>
      <Disc src={cover} spinning={playing} />
      <span class="meta">
        <span class="line">
          <span class="title">{title}</span>
          {#if live}<Badge size="sm">DIRECT</Badge>{/if}
          {#if groupMode}<Badge variant="group" size="sm">GROUPE</Badge>{/if}
        </span>
        <span class="artist">{artist ?? "Artiste inconnu"}</span>
      </span>
    </button>

    <div class="center">
      <div class="controls">
        {#if !groupMode}
          <IconButton
            class="sm"
            icon="shuffle"
            label="Lecture aléatoire"
            size={34}
            iconSize={16}
            active={s.shuffle}
            onclick={() => player.toggleShuffle()}
          />
        {/if}
        <IconButton
          icon="previous"
          label="Précédent"
          onclick={() => (groupMode ? group.seek(0) : player.previous())}
        />
        <PlayButton
          size={42}
          label={playing ? "Pause" : "Lecture"}
          {playing}
          onclick={() => (groupMode ? group.togglePlay() : player.togglePlay())}
        />
        <IconButton
          icon="next"
          label="Suivant"
          onclick={() => (groupMode ? group.skip() : player.next())}
        />
        {#if !groupMode}
          <IconButton
            class="sm"
            icon={s.repeat === "one" ? "repeatOne" : "repeat"}
            label="Répéter"
            size={34}
            iconSize={16}
            active={s.repeat !== "off"}
            onclick={() => player.cycleRepeat()}
          />
        {/if}
        {#if ui.compact}
          <IconButton
            icon="queue"
            label="File d'attente"
            onclick={() => {
              ui.sideOpen = false;
              ui.queueOpen = true;
            }}
          />
        {/if}
      </div>
      <div class="progress mono" class:hidden={!!live}>
        <span class="time r">{formatTime(position)}</span>
        <RailBar value={duration ? position / duration : 0} label="Position" knob onvalue={seek} />
        <span class="time">{formatTime(duration)}</span>
      </div>
    </div>

    <div class="end">
      <span class="eq"
        ><Visualizer
          {playing}
          levels={groupMode ? undefined : player.levels}
          bars={4}
          gap={3}
          color="var(--accent)"
        /></span
      >
      {#if ytId}
        <IconButton
          class="sm"
          icon={liked ? "heartFilled" : "heart"}
          label="J'aime"
          size={34}
          active={liked}
          onclick={() => feedback.toggleLike({ youtube_id: ytId, title: title ?? "" })}
        />
      {/if}
      {#if ui.showVolume}
        <Icon name="volume" size={16} />
        <RailBar
          variant="volume"
          dock
          value={player.volume}
          label="Volume"
          onvalue={(f) => player.setVolume(f)}
        />
      {/if}
      <IconButton
        class="sm"
        icon="expand"
        label="Plein écran"
        size={34}
        iconSize={16}
        onclick={() => (live ? lives.expandVideo() : (ui.fullscreen = true))}
      />
    </div>
  </footer>
</div>

<style>
  .wrap {
    position: absolute;
    bottom: 16px;
    box-sizing: border-box;
    width: auto;
    z-index: 10;
    display: flex;
    justify-content: center;
    padding: 0 16px;
    pointer-events: none;
    transition:
      left 0.55s var(--ease-out),
      right 0.55s var(--ease-out);
  }
  .dock {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr) minmax(0, 1fr);
    align-items: center;
    gap: 20px;
    width: 100%;
    max-width: 1000px;
    padding: 10px 16px 10px 10px;
    box-sizing: border-box;
    border-radius: calc(var(--rs) + 12px);
    pointer-events: auto;
  }
  .dock.compact {
    position: relative;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    padding: 8px 12px 12px 8px;
  }
  .dock.compact .end,
  .dock.compact .time,
  .dock.compact :global(.halo-btn.sm) {
    display: none;
  }
  .dock.compact .progress {
    position: absolute;
    right: 16px;
    bottom: 3px;
    left: 16px;
    max-height: 14px;
  }
  .dock.compact .center {
    gap: 0;
  }
  .dock.compact .controls {
    gap: 2px;
  }
  .now {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 0;
    background: none;
    text-align: left;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .title,
  .artist {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title {
    font-size: 14px;
    font-weight: 600;
  }
  .artist {
    color: var(--mute);
    font-size: 12px;
  }
  .center {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: 10px;
    max-height: 20px;
    overflow: hidden;
    color: var(--mute);
    font-size: 11px;
    transition:
      max-height 0.45s var(--ease-out),
      opacity 0.3s;
  }
  .progress.hidden {
    max-height: 0;
    opacity: 0;
  }
  .time {
    width: 34px;
  }
  .time.r {
    text-align: right;
  }
  .end {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
  }
  .eq {
    width: 21px;
    height: 18px;
    margin-right: 4px;
  }
</style>
