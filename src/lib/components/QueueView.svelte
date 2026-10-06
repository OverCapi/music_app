<script lang="ts">
  import { player } from '../stores/player.svelte'
  import { formatTime } from '../utils'
  import Cover from './Cover.svelte'
  import Icon from './Icon.svelte'

  let dragFrom = $state<number | null>(null)
  let dragOver = $state<number | null>(null)

  function drop(to: number) {
    if (dragFrom !== null) player.reorder(dragFrom, to)
    dragFrom = dragOver = null
  }
</script>

<div class="queue">
  <h2>En cours de lecture</h2>
  {#if player.current}
    <div class="item now">
      <Cover path={player.current.cover} size={48} />
      <div class="text">
        <span class="title">{player.current.title}</span>
        <span class="sub">{player.current.artist || 'Artiste inconnu'} · {player.current.album}</span>
      </div>
      <span class="duration">{formatTime(player.current.duration)}</span>
    </div>
  {:else}
    <p class="empty">Rien en cours de lecture.</p>
  {/if}

  {#each [{ label: "File d'attente", from: 0, to: player.userCount }, { label: 'À suivre', from: player.userCount, to: player.queue.length }] as section (section.label)}
    {#if section.to > section.from}
      <h2>{section.label}</h2>
      {#each player.queue.slice(section.from, section.to) as track, j (`${section.from + j}-${track.id}`)}
        {@const index = section.from + j}
        <div
          class="item"
          class:over={dragOver === index && dragFrom !== index}
          role="listitem"
          draggable="true"
          ondragstart={(e) => {
            dragFrom = index
            e.dataTransfer?.setData('text/plain', String(index))
          }}
          ondragover={(e) => {
            e.preventDefault()
            dragOver = index
          }}
          ondragend={() => (dragFrom = dragOver = null)}
          ondrop={(e) => {
            e.preventDefault()
            drop(index)
          }}
          ondblclick={() => player.jumpTo(index)}
        >
          <span class="grip"><Icon name="grip" size={16} /></span>
          <Cover path={track.cover} size={40} />
          <div class="text">
            <span class="title">{track.title}</span>
            <span class="sub">{track.artist || 'Artiste inconnu'}</span>
          </div>
          <span class="duration">{formatTime(track.duration)}</span>
          <button class="icon remove" title="Retirer de la file" onclick={() => player.removeFromQueue(index)}>
            <Icon name="close" size={14} />
          </button>
        </div>
      {/each}
    {/if}
  {/each}

  {#if player.current && player.queue.length === 0}
    <p class="empty">La file d'attente est vide.</p>
  {/if}
</div>

<style>
  .queue {
    padding: 0 24px 24px;
    max-width: 900px;
  }
  h2 {
    margin: 24px 0 8px;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 8px;
    border-radius: 6px;
    border-top: 2px solid transparent;
  }
  .item:hover {
    background: var(--surface-2);
  }
  .item.over {
    border-top-color: var(--accent);
  }
  .now .title {
    color: var(--accent);
  }
  .grip {
    color: var(--text-dim);
    cursor: grab;
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title {
    font-weight: 500;
  }
  .sub {
    font-size: 13px;
    color: var(--text-dim);
  }
  .duration {
    font-size: 13px;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .remove {
    visibility: hidden;
  }
  .item:hover .remove {
    visibility: visible;
  }
  .empty {
    color: var(--text-dim);
  }
</style>
