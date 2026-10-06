# Music

App de musique perso (Tauri 2 + Svelte 5 + TypeScript). Pour l'instant : lecture de fichiers locaux.

## Lancer

```bash
npm install
npm run tauri dev
```

## Structure

- `src-tauri/src/library.rs` : scan des dossiers, lecture des tags et pochettes (lofty)
- `src/lib/stores/library.svelte.ts` : bibliothèque (dossiers, fichiers importés, persistance)
- `src/lib/stores/player.svelte.ts` : moteur de lecture HTML5, file d'attente, shuffle/repeat
- `src/lib/components/` : UI (sidebar, bibliothèque, file, barre de lecture)

## Raccourcis

Espace : lecture/pause · ←/→ : -5 s/+5 s · ⌘←/⌘→ : précédent/suivant · ⌘F : recherche
