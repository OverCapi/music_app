<script lang="ts">
  import { library } from '../stores/library.svelte'
  import Icon from './Icon.svelte'
</script>

<div class="empty">
  <Icon name="note" size={56} />
  <h2>Ta bibliothèque est vide</h2>
  <p>Ajoute un dossier de musique, importe des fichiers, ou glisse-les directement dans la fenêtre.</p>
  <div class="actions">
    <button class="primary" onclick={() => library.addFolder()}><Icon name="folder" /> Ajouter un dossier</button>
    <button class="secondary" onclick={() => library.importFiles()}><Icon name="file" /> Importer des fichiers</button>
  </div>
</div>

<style>
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
    color: var(--text-dim);
  }
  h2 {
    margin: 12px 0 0;
    color: var(--text);
  }
  p {
    max-width: 380px;
    margin: 0 0 12px;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
</style>
