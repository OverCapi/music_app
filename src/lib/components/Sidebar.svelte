<script lang="ts">
  import { library } from '../stores/library.svelte'
  import { player } from '../stores/player.svelte'
  import type { View } from '../types'
  import Icon from './Icon.svelte'

  let { view = $bindable() }: { view: View } = $props()

  function folderName(path: string) {
    return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path
  }
</script>

<aside>
  <h1><Icon name="note" size={20} /> Music</h1>

  <nav>
    <button class:active={view === 'library'} onclick={() => (view = 'library')}>
      <Icon name="library" /> Bibliothèque
      <span class="count">{library.tracks.length}</span>
    </button>
    <button class:active={view === 'queue'} onclick={() => (view = 'queue')}>
      <Icon name="queue" /> File d'attente
      {#if player.queue.length}<span class="count">{player.queue.length}</span>{/if}
    </button>
  </nav>

  <section>
    <header>
      <span>Dossiers</span>
      <button class="icon" title="Rescanner" onclick={() => library.rescan()} disabled={library.scanning}>
        <span class:spin={library.scanning}><Icon name="refresh" size={14} /></span>
      </button>
    </header>
    <ul>
      {#each library.folders as folder (folder)}
        <li title={folder}>
          <Icon name="folder" size={14} />
          <span class="name">{folderName(folder)}</span>
          <button class="icon remove" title="Retirer" onclick={() => library.removeFolder(folder)}>
            <Icon name="close" size={12} />
          </button>
        </li>
      {/each}
      {#if library.files.length}
        <li class="muted"><Icon name="file" size={14} /> {library.files.length} fichier(s) importé(s)</li>
      {/if}
    </ul>
  </section>

  <div class="actions">
    <button class="secondary" onclick={() => library.addFolder()}><Icon name="folder" /> Ajouter un dossier</button>
    <button class="secondary" onclick={() => library.importFiles()}><Icon name="file" /> Importer des fichiers</button>
  </div>
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 20px 12px;
    background: var(--surface-1);
    border-right: 1px solid var(--border);
    min-height: 0;
  }
  h1 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 8px;
    font-size: 18px;
    color: var(--accent);
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    color: var(--text-dim);
    font-weight: 500;
    text-align: left;
  }
  nav button:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  nav button.active {
    color: var(--text);
    background: var(--surface-3);
  }
  .count {
    margin-left: auto;
    font-size: 12px;
    color: var(--text-dim);
  }
  section {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  section header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 10px 6px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    font-size: 13px;
    color: var(--text-dim);
    border-radius: 6px;
  }
  li:hover {
    background: var(--surface-2);
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .remove {
    visibility: hidden;
  }
  li:hover .remove {
    visibility: visible;
  }
  .actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .spin {
    display: inline-flex;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
