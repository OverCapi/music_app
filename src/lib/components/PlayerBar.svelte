<script lang="ts">
  import { player } from '../stores/player.svelte'
  import { formatTime } from '../utils'
  import Cover from './Cover.svelte'
  import Icon from './Icon.svelte'

  // While dragging the seek bar we show the preview position and only seek on release.
  let scrubbing = $state<number | null>(null)
  const shownPosition = $derived(scrubbing ?? player.position)
  const progress = $derived(player.duration ? (shownPosition / player.duration) * 100 : 0)
  const volumePct = $derived((player.muted ? 0 : player.volume) * 100)
</script>

<footer>
  <div class="now">
    {#if player.current}
      <Cover path={player.current.cover} size={56} />
      <div class="text">
        <span class="title">{player.current.title}</span>
        <span class="artist">{player.current.artist || 'Artiste inconnu'}</span>
      </div>
    {/if}
  </div>

  <div class="center">
    <div class="controls">
      <button class="icon toggle" class:on={player.shuffle} title="Lecture aléatoire" onclick={() => player.toggleShuffle()}>
        <Icon name="shuffle" />
      </button>
      <button class="icon" title="Précédent" onclick={() => player.prev()} disabled={!player.current}>
        <Icon name="prev" size={20} />
      </button>
      <button class="play" title={player.isPlaying ? 'Pause' : 'Lecture'} onclick={() => player.toggle()} disabled={!player.current}>
        <Icon name={player.isPlaying ? 'pause' : 'play'} size={20} />
      </button>
      <button class="icon" title="Suivant" onclick={() => player.next()} disabled={!player.current}>
        <Icon name="next" size={20} />
      </button>
      <button
        class="icon toggle"
        class:on={player.repeat !== 'off'}
        title={{ off: 'Répéter : non', all: 'Répéter : tout', one: 'Répéter : ce titre' }[player.repeat]}
        onclick={() => player.cycleRepeat()}
      >
        <Icon name="repeat" />
        {#if player.repeat === 'one'}<span class="one">1</span>{/if}
      </button>
    </div>

    <div class="progress">
      <span class="time">{formatTime(shownPosition)}</span>
      <input
        type="range"
        min="0"
        max={player.duration || 0}
        step="0.1"
        value={shownPosition}
        style:--pct="{progress}%"
        disabled={!player.current}
        aria-label="Position"
        oninput={(e) => (scrubbing = e.currentTarget.valueAsNumber)}
        onchange={(e) => {
          player.seek(e.currentTarget.valueAsNumber)
          scrubbing = null
        }}
      />
      <span class="time">{formatTime(player.duration)}</span>
    </div>
  </div>

  <div class="volume">
    <button class="icon" title={player.muted ? 'Réactiver le son' : 'Couper le son'} onclick={() => player.toggleMute()}>
      <Icon name={player.muted || player.volume === 0 ? 'mute' : 'volume'} />
    </button>
    <input
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={player.muted ? 0 : player.volume}
      style:--pct="{volumePct}%"
      aria-label="Volume"
      oninput={(e) => player.setVolume(e.currentTarget.valueAsNumber)}
    />
  </div>
</footer>

<style>
  footer {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: minmax(180px, 1fr) minmax(320px, 2fr) minmax(140px, 1fr);
    align-items: center;
    gap: 16px;
    padding: 12px 20px;
    background: var(--surface-1);
    border-top: 1px solid var(--border);
  }
  .now {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .text {
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
    font-weight: 600;
  }
  .artist {
    font-size: 13px;
    color: var(--text-dim);
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
    gap: 14px;
  }
  .play {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: var(--text);
    color: var(--bg);
    transition: transform 0.1s;
  }
  .play:hover:not(:disabled) {
    transform: scale(1.06);
  }
  .toggle {
    position: relative;
  }
  .toggle.on {
    color: var(--accent);
  }
  .one {
    position: absolute;
    top: -4px;
    right: -6px;
    font-size: 9px;
    font-weight: 700;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
  }
  .time {
    min-width: 40px;
    font-size: 12px;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }
  .progress input {
    flex: 1;
  }
  .volume {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .volume input {
    width: 110px;
  }
</style>
