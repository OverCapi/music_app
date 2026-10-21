<script lang="ts">
  /** Floating dock between the two panels (max 1000 px), shrinks with them.
   * Left: spinning disc cover + title (+ DIRECT / GROUPE badges). Center: shuffle · prev ·
   * play · next · repeat, then a 4 px progress bar (hidden for lives). Right: mini equalizer ·
   * like · volume (widens on hover, draggable) · fullscreen.
   * Group / live logic is the same as the current PlayerBar — only the presentation changes. */
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { player } from "#lib/stores/player.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { lives } from "#lib/stores/lives.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { feedback } from "#lib/stores/feedback.svelte.ts";
  import { ui } from "#lib/stores/ui.svelte.ts";
  import { fileName, formatTime } from "#lib/format.ts";
  import { liquid } from "#lib/motion.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import Visualizer from "#lib/components/ui/Visualizer.svelte";

  const s = $derived(player.state);
  const live = $derived(s.group_mode ? null : s.live);
  const track = $derived(s.track_id !== null ? library.byId.get(s.track_id) : undefined);
  const groupMode = $derived(group.isActive || s.group_mode);
  const entry = $derived(group.currentEntry);
  const playing = $derived(
    groupMode ? ["playing", "preparing"].includes(group.playback.status) : s.playing,
  );

  let now = $state(Date.now());
  $effect(() => {
    if (!playing) return;
    const t = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(t);
  });

  const duration = $derived(
    groupMode && !(s.group_mode && s.loaded) ? (entry?.duration ?? 0) : player.duration,
  );
  const position = $derived(
    Math.min(
      groupMode && !(s.group_mode && s.loaded) ? group.positionAt(now) : player.positionAt(now),
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
      ? entry && `https://i.ytimg.com/vi/${entry.youtube_id}/mqdefault.jpg`
      : live
        ? live.thumbnail
        : s.track_id !== null
          ? convertFileSrc(String(s.track_id), "cover")
          : undefined,
  );
  const ytId = $derived(groupMode ? entry?.youtube_id : track?.youtube_id);
  const liked = $derived(!!ytId && feedback.trackKind(ytId) === "like");

  let volDragging = $state(false);
  function volume(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    const set = (x: number) => {
      const r = el.getBoundingClientRect();
      player.setVolume(Math.max(0, Math.min(1, (x - r.left) / r.width)));
    };
    volDragging = true;
    set(e.clientX);
    const move = (ev: PointerEvent) => set(ev.clientX);
    const up = () => (
      removeEventListener("pointermove", move),
      removeEventListener("pointerup", up),
      (volDragging = false)
    );
    addEventListener("pointermove", move);
    addEventListener("pointerup", up);
  }

  function seek(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const secs = ((e.clientX - r.left) / r.width) * duration;
    return groupMode ? group.seek(secs) : player.seek(secs);
  }
</script>

<div class="wrap" style:left="{ui.left}px" style:right="{ui.right}px">
  <footer class="dock glass" class:compact={ui.compact}>
    <button class="now" onclick={() => (live ? lives.expandVideo() : (ui.fullscreen = true))}>
      <span class="disc" class:spinning={playing}>
        {#key cover}{#if cover}<img class="swap" src={cover} alt="" />{/if}{/key}
        <span class="hole"></span>
      </span>
      <span class="meta">
        <span class="line">
          <span class="title">{title}</span>
          {#if live}<span class="badge-live">DIRECT</span>{/if}
          {#if groupMode}<span class="badge-group">GROUPE</span>{/if}
        </span>
        <span class="artist">{artist ?? "Artiste inconnu"}</span>
      </span>
    </button>

    <div class="center">
      <div class="controls">
        {#if !groupMode}
          <button
            class="halo-btn sm"
            class:active={s.shuffle}
            use:liquid
            onclick={() => player.toggleShuffle()}
            aria-label="Lecture aléatoire"><Icon name="shuffle" size={16} /></button
          >
        {/if}
        <button
          class="halo-btn"
          use:liquid
          onclick={() => (groupMode ? group.seek(0) : player.previous())}
          aria-label="Précédent"><Icon name="previous" size={18} /></button
        >
        <button
          class="play-btn"
          style="width:42px;height:42px"
          use:liquid
          onclick={() => (groupMode ? group.togglePlay() : player.togglePlay())}
          aria-label={playing ? "Pause" : "Lecture"}
        >
          <span class="swap-ico" class:on={playing}><Icon name="pause" size={18} /></span>
          <span class="swap-ico" class:on={!playing}><Icon name="play" size={18} /></span>
        </button>
        <button
          class="halo-btn"
          use:liquid
          onclick={() => (groupMode ? group.skip() : player.next())}
          aria-label="Suivant"><Icon name="next" size={18} /></button
        >
        {#if !groupMode}
          <button
            class="halo-btn sm"
            class:active={s.repeat !== "off"}
            use:liquid
            onclick={() => player.cycleRepeat()}
            aria-label="Répéter"
            ><Icon name={s.repeat === "one" ? "repeatOne" : "repeat"} size={16} /></button
          >
        {/if}
        {#if ui.compact}
          <button
            class="halo-btn"
            use:liquid
            onclick={() => {
              ui.sideOpen = false;
              ui.queueOpen = true;
            }}
            aria-label="File d'attente"><Icon name="queue" size={18} /></button
          >
        {/if}
      </div>
      <div class="progress mono" class:hidden={!!live}>
        <span class="time r">{formatTime(position)}</span>
        <button class="track" onclick={seek} aria-label="Position">
          <span class="rail"
            ><span class="fill" style:width="{duration ? (position / duration) * 100 : 0}%"
              ><span class="knob"></span></span
            ></span
          >
        </button>
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
        <button
          class="halo-btn sm"
          class:active={liked}
          use:liquid
          onclick={() => feedback.toggleLike({ youtube_id: ytId, title: title ?? "" })}
          aria-label="J'aime"
        >
          <Icon name={liked ? "heartFilled" : "heart"} size={18} />
        </button>
      {/if}
      {#if ui.showVolume}
        <Icon name="volume" size={16} />
        <button class="vol" onpointerdown={volume} aria-label="Volume">
          <span class="rail"
            ><span
              class="fill vfill"
              class:dragging={volDragging}
              style:width="{player.volume * 100}%"
            ></span></span
          >
        </button>
      {/if}
      <button
        class="halo-btn sm"
        use:liquid
        onclick={() => (live ? lives.expandVideo() : (ui.fullscreen = true))}
        aria-label="Plein écran"><Icon name="expand" size={16} /></button
      >
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
  .disc {
    position: relative;
    flex: none;
    width: 52px;
    height: 52px;
    overflow: hidden;
    border-radius: 50%;
    background: var(--line);
    box-shadow: 0 8px 20px -6px var(--shadow);
    animation: spin 9s linear infinite paused;
  }
  .disc.spinning {
    animation-play-state: running;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .disc img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .hole {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    border-radius: 50%;
    background: var(--bg);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .swap {
    animation: swapin 0.45s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  @keyframes swapin {
    from {
      opacity: 0.3;
      transform: scale(1.1);
      filter: blur(10px);
    }
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
  .badge-live {
    flex: none;
    font-size: 9px;
    padding: 2px 7px;
  }
  .badge-group {
    flex: none;
    padding: 2px 7px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--glass2);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.06em;
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
  .halo-btn.sm {
    width: 34px;
    height: 34px;
  }
  .swap-ico {
    grid-area: 1 / 1;
    display: grid;
    opacity: 0;
    transform: scale(0.5) rotate(-30deg);
    transition:
      opacity 0.25s,
      transform 0.35s var(--ease-spring);
  }
  .swap-ico.on {
    opacity: 1;
    transform: none;
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
  .track {
    flex: 1;
    height: 14px;
    padding: 0;
    background: none;
  }
  .rail {
    position: relative;
    display: block;
    height: 4px;
    border-radius: 4px;
    background: var(--line);
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 6px;
    background: var(--accent);
    box-shadow: 0 0 14px var(--accent);
    transition: width 0.3s linear;
  }
  .knob {
    position: absolute;
    right: -6px;
    top: 50%;
    width: 12px;
    height: 12px;
    margin-top: -6px;
    border-radius: 50%;
    background: var(--fg);
    box-shadow: 0 2px 8px var(--shadow);
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
  .vol {
    width: 84px;
    height: 16px;
    padding: 6px 0;
    box-sizing: border-box;
    background: none;
    touch-action: none;
    transition:
      width 0.35s var(--ease-out),
      padding 0.3s var(--ease-out);
  }
  .vol:hover {
    width: 108px;
    padding: 4px 0;
  }
  .vol .rail {
    height: 100%;
  }
  .vfill {
    background: var(--fg);
    box-shadow: none;
    transition: width 0.35s var(--ease-out);
  }
  .vfill.dragging {
    transition: width 0.07s linear;
  }
</style>
