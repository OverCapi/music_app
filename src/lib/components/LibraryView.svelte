<script lang="ts">
  import { player } from '../stores/player.svelte'
  import type { Track } from '../types'
  import { formatTime } from '../utils'
  import Cover from './Cover.svelte'
  import Icon from './Icon.svelte'
  import TrackMenu, { type MenuState } from './TrackMenu.svelte'

  type SortKey = 'default' | 'title' | 'artist' | 'album' | 'duration'

  let { tracks }: { tracks: Track[] } = $props()

  let sortKey = $state<SortKey>('default')
  let sortDir = $state<1 | -1>(1)
  let selectedId = $state<string | null>(null)
  let menu = $state<MenuState | null>(null)

  const sorted = $derived.by(() => {
    if (sortKey === 'default') return tracks
    const key = sortKey
    return [...tracks].sort((a, b) => {
      const r =
        key === 'duration'
          ? a.duration - b.duration
          : a[key].localeCompare(b[key], undefined, { sensitivity: 'base' })
      return r * sortDir
    })
  })

  function sortBy(key: SortKey) {
    if (sortKey === key) {
      // Third click returns to the natural artist/album/track order.
      if (sortDir === -1) sortKey = 'default'
      sortDir = sortDir === 1 ? -1 : 1
    } else {
      sortKey = key
      sortDir = 1
    }
  }

  function openMenu(e: MouseEvent, track: Track) {
    e.preventDefault()
    selectedId = track.id
    menu = { x: e.clientX, y: e.clientY, track, context: sorted }
  }
</script>

{#snippet header(key: SortKey, label: string)}
  <button class="th" onclick={() => sortBy(key)}>
    {label}
    {#if sortKey === key}<span class="arrow">{sortDir === 1 ? '▲' : '▼'}</span>{/if}
  </button>
{/snippet}

<div class="table" role="grid">
  <div class="row head">
    <span class="num">#</span>
    {@render header('title', 'Titre')}
    {@render header('album', 'Album')}
    {@render header('duration', 'Durée')}
  </div>

  {#each sorted as track, i (track.id)}
    {@const isCurrent = player.current?.id === track.id}
    <div
      class="row"
      class:current={isCurrent}
      class:selected={selectedId === track.id}
      class:broken={player.unplayable.has(track.id)}
      role="row"
      tabindex="-1"
      onclick={() => (selectedId = track.id)}
      ondblclick={() => player.play(track, sorted)}
      oncontextmenu={(e) => openMenu(e, track)}
      onkeydown={(e) => e.key === 'Enter' && player.play(track, sorted)}
    >
      <span class="num">
        {#if isCurrent && player.isPlaying}
          <span class="eq"><i></i><i></i><i></i></span>
        {:else}
          <span class="idx">{i + 1}</span>
          <button class="icon play" title="Lire" onclick={() => player.play(track, sorted)}><Icon name="play" size={14} /></button>
        {/if}
      </span>
      <span class="title-cell">
        <Cover path={track.cover} size={36} />
        <span class="text">
          <span class="title">{track.title}</span>
          <span class="artist">{track.artist || 'Artiste inconnu'}</span>
        </span>
      </span>
      <span class="album">{track.album}</span>
      <span class="duration">
        {#if player.unplayable.has(track.id)}<span class="badge" title="Format non supporté par le lecteur">non lisible</span>{/if}
        {formatTime(track.duration)}
      </span>
    </div>
  {/each}
</div>

<TrackMenu bind:menu />

<style>
  .table {
    display: flex;
    flex-direction: column;
    padding: 0 16px 16px;
  }
  .row {
    display: grid;
    grid-template-columns: 40px minmax(0, 3fr) minmax(0, 2fr) 110px;
    align-items: center;
    gap: 12px;
    padding: 4px 8px;
    border-radius: 6px;
    user-select: none;
  }
  .row:not(.head):hover {
    background: var(--surface-2);
  }
  .row.selected {
    background: var(--surface-3);
  }
  .row.current .title {
    color: var(--accent);
  }
  .row.broken {
    opacity: 0.5;
  }
  .head {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    border-radius: 0;
    padding-block: 8px;
    margin-bottom: 4px;
    font-size: 12px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .th {
    text-align: left;
    color: inherit;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
  }
  .th:hover {
    color: var(--text);
  }
  .arrow {
    font-size: 9px;
    margin-left: 4px;
  }
  .num {
    display: grid;
    place-items: center;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
    font-size: 13px;
  }
  .play {
    display: none;
  }
  .row:hover .play {
    display: grid;
  }
  .row:hover .idx {
    display: none;
  }
  .title-cell {
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
  .artist,
  .album {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title {
    font-weight: 500;
  }
  .artist,
  .album {
    font-size: 13px;
    color: var(--text-dim);
  }
  .duration {
    text-align: right;
    font-size: 13px;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .head .th:last-child {
    text-align: right;
  }
  .badge {
    font-size: 10px;
    padding: 1px 5px;
    margin-right: 6px;
    border-radius: 4px;
    background: var(--danger);
    color: white;
  }
  .eq {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 14px;
  }
  .eq i {
    width: 3px;
    background: var(--accent);
    animation: eq 0.9s ease-in-out infinite alternate;
  }
  .eq i:nth-child(2) {
    animation-delay: -0.3s;
  }
  .eq i:nth-child(3) {
    animation-delay: -0.6s;
  }
  @keyframes eq {
    from {
      height: 3px;
    }
    to {
      height: 14px;
    }
  }
</style>
