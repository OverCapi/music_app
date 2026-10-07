# Music App

Lecteur de musique de bureau inspiré de Spotify, centré sur les **fichiers locaux**. Les morceaux sont toujours lus depuis la machine de l'utilisateur, y compris dans les groupes d'écoute.

Tauri 2 + SvelteKit (Svelte 5) pour l'interface, Rust pour le backend (lecture audio, bibliothèque, téléchargements), et un petit serveur Rust pour les groupes d'écoute.

## Fonctionnalités

- **Lecture** de fichiers `.mp3` et `.opus` : lecture/pause, stop, barre de progression, volume.
- **File d'attente** : suivant / précédent, enchaînement automatique, ajout de titres. Quand la file est vide, un titre au hasard de la bibliothèque est joué.
- **Bibliothèque** persistante, avec titre, artiste, album et durée lus dans les tags, et une recherche.
- **Playlists** : création, renommage, suppression, ajout et retrait de titres, lecture.
- **Recherche YouTube Music / YouTube** via yt-dlp, avec « Plus de résultats ».
- **Téléchargement** en Opus, avec tags et pochette, dans `~/Music/Music App`. Le titre est ajouté automatiquement à la bibliothèque. Le supprimer de la bibliothèque met le fichier à la corbeille.
- **Groupes d'écoute** : comptes sur un serveur et groupe rejoignable par un code (ou un lien `musicapp://join/CODE`).
  - Chaque membre télécharge les sons localement, et la lecture démarre en même temps chez tout le monde.
  - Tous les membres peuvent ajouter, mettre en pause, avancer dans le son ou passer au suivant.

## Prérequis

- [Rust](https://rustup.rs/), Node.js et [pnpm](https://pnpm.io/)
- Dépendances Tauri pour Linux (`webkit2gtk-4.1`, etc.) et `alsa-lib`
- `libopus` (paquet `opus`), utilisé pour lire les `.opus`
- [yt-dlp](https://github.com/yt-dlp/yt-dlp), dans le `PATH` ou dans `~/Applications/yt-dlp/yt-dlp`
- `ffmpeg`, pour l'extraction audio et l'intégration des tags et de la pochette
- `node`, recommandé : yt-dlp en a besoin comme moteur JavaScript pour télécharger depuis YouTube

## Lancer l'app

```sh
pnpm install
pnpm tauri dev      # ou ./run.sh
```

## Groupes d'écoute

Lancer le serveur :

```sh
cd server
cargo run           # écoute sur 127.0.0.1:8787
```

Variables d'environnement du serveur :

| Variable | Défaut | Rôle |
| --- | --- | --- |
| `MUSIC_SERVER_ADDR` | `127.0.0.1:8787` | adresse d'écoute |
| `MUSIC_SERVER_DB` | `./server.db` | base SQLite des comptes |
| `MUSIC_SERVER_READY_TIMEOUT_MS` | `30000` | attente maximale des membres avant de lancer un son |

Dans l'app, ouvrir « Groupe d'écoute », se connecter (l'adresse du serveur est modifiable), puis créer un groupe ou en rejoindre un en collant le code ou le lien.

Pour tester à deux sur une même machine, lancer une seconde instance avec un profil séparé. Elle utilise le serveur Vite de la première instance, qui doit donc rester ouverte :

```sh
MUSIC_APP_PROFILE=b src-tauri/target/debug/music_app
```

## Développement

```sh
pnpm check                                   # vérification TypeScript / Svelte
cd src-tauri && cargo clippy && cargo test   # backend de l'app
cd server && cargo test                      # serveur (tests unitaires + intégration)
cd protocol && cargo test                    # protocole partagé
```

Organisation du dépôt :

- `src/` : interface SvelteKit (stores dans `src/lib/*.svelte.ts`, composants dans `src/lib/components/`)
- `src-tauri/` : backend Rust de l'app
- `server/` : serveur des groupes d'écoute
- `protocol/` : messages partagés entre l'app et le serveur

Les évolutions prévues sont listées dans [TODO.md](TODO.md).
