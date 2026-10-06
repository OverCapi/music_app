<script lang="ts">
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { onMount } from 'svelte'
  import EmptyState from './lib/components/EmptyState.svelte'
  import LibraryView from './lib/components/LibraryView.svelte'
  import PlayerBar from './lib/components/PlayerBar.svelte'
  import QueueView from './lib/components/QueueView.svelte'
  import SearchBar from './lib/components/SearchBar.svelte'
  import Sidebar from './lib/components/Sidebar.svelte'
  import { library } from './lib/stores/library.svelte'
  import { player } from './lib/stores/player.svelte'
  import type { View } from './lib/types'
  import { normalize } from './lib/utils'

  let view = $state<View>('library')
  let query = $state('')
  let searchInput = $state<HTMLInputElement>()
  let dropping = $state(false)

  // Precompute a normalized haystack per track so filtering stays cheap while typing.
  const haystacks = $derived(
    library.tracks.map((t) => ({ track: t, text: normalize(`${t.title} ${t.artist} ${t.album}`) })),
  )
  const filtered = $derived.by(() => {
    const terms = normalize(query).split(/\s+/).filter(Boolean)
    if (terms.length === 0) return library.tracks
    return haystacks.filter((h) => terms.every((term) => h.text.includes(term))).map((h) => h.track)
  })

  onMount(() => {
    player.init()
    library.init()

    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      const { type } = event.payload
      if (type === 'enter' || type === 'over') dropping = true
      else if (type === 'leave') dropping = false
      else if (type === 'drop') {
        dropping = false
        if (event.payload.paths.length) library.addPaths(event.payload.paths)
      }
    })
    return () => {
      unlisten.then((fn) => fn())
    }
  })

  function onKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement
    const typing = target.tagName === 'INPUT' && (target as HTMLInputElement).type !== 'range'
    const mod = e.metaKey || e.ctrlKey

    if (mod && e.key.toLowerCase() === 'f') {
      e.preventDefault()
      view = 'library'
      searchInput?.focus()
      return
    }
    if (typing) return

    switch (e.key) {
      case ' ':
        e.preventDefault()
        player.toggle()
        break
      case 'ArrowRight':
        e.preventDefault()
        if (mod) player.next()
        else player.seekBy(5)
        break
      case 'ArrowLeft':
        e.preventDefault()
        if (mod) player.prev()
        else player.seekBy(-5)
        break
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app">
  <Sidebar bind:view />

  <main>
    {#if view === 'library'}
      <header>
        <h2>Bibliothèque</h2>
        <SearchBar bind:query bind:input={searchInput} />
      </header>
      <div class="scroll">
        {#if library.error}
          <p class="message error">Erreur lors du scan : {library.error}</p>
        {/if}
        {#if library.tracks.length === 0}
          {#if library.scanning}
            <p class="message">Analyse de la bibliothèque…</p>
          {:else}
            <EmptyState />
          {/if}
        {:else if filtered.length === 0}
          <p class="message">Aucun résultat pour « {query} ».</p>
        {:else}
          <LibraryView tracks={filtered} />
        {/if}
      </div>
    {:else}
      <header><h2>File d'attente</h2></header>
      <div class="scroll"><QueueView /></div>
    {/if}
  </main>

  <PlayerBar />

  {#if dropping}
    <div class="dropzone">Dépose tes fichiers ou dossiers pour les ajouter</div>
  {/if}
</div>

<style>
  .app {
    height: 100vh;
    display: grid;
    grid-template-columns: 240px 1fr;
    grid-template-rows: minmax(0, 1fr) auto;
  }
  main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 20px 24px 12px;
  }
  h2 {
    margin: 0;
    font-size: 24px;
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
  }
  .message {
    padding: 0 24px;
    color: var(--text-dim);
  }
  .error {
    color: var(--danger);
  }
  .dropzone {
    position: fixed;
    inset: 12px;
    z-index: 200;
    display: grid;
    place-items: center;
    border: 2px dashed var(--accent);
    border-radius: 16px;
    background: rgb(0 0 0 / 0.6);
    font-size: 18px;
    pointer-events: none;
  }
</style>
