# Music App

Lecteur de musique de bureau inspiré de Spotify, centré sur les **fichiers locaux**. Les morceaux sont toujours lus depuis la machine de l'utilisateur, y compris dans les groupes d'écoute.

Tauri 2 + SvelteKit 3 (Svelte 5) pour l'interface, Rust pour le backend (lecture audio, bibliothèque, téléchargements), et un petit serveur Rust pour les groupes d'écoute.

## Fonctionnalités

- **Lecture** de fichiers `.mp3` et `.opus` : lecture/pause, stop, barre de progression, volume.
- **File d'attente** : suivant / précédent, enchaînement automatique, ajout de titres. Quand la file est vide, un titre au hasard de la bibliothèque est joué.
- **Bibliothèque** persistante (SQLite ; les anciens fichiers `library.json` / `playlists.json` / `account.json` sont importés automatiquement au premier lancement puis renommés en `.bak`), avec titre, artiste, album et durée lus dans les tags, et une recherche.
- **Playlists** : création, renommage, suppression, ajout et retrait de titres, lecture.
- **Accueil** : mixes générés à partir de vos goûts (Que du connu / Que du nouveau / Mélangé), découvertes de la semaine et écoutes récentes.
- **Lives YouTube** : recherche par mot-clé, par chaîne ou par lien, chaînes favorites. Le son passe par le moteur audio ; la vidéo peut être agrandie en plein écran dans l'app (sans le son, avec un décalage de quelques secondes).
- **Synchronisation** des titres likés et des playlists YouTube Music vers la bibliothèque (Spotify : prévu), dans les paramètres. Les titres sont téléchargés à la première lecture.
- **Thèmes de couleurs** (Nuit, Aube, Braise, Océan, Rose) dans les paramètres.
- **Recherche YouTube Music / YouTube** via yt-dlp, avec « Plus de résultats ».
- **Téléchargement** en Opus, avec tags et pochette, dans `~/Music/Music App`. Le titre est ajouté automatiquement à la bibliothèque. Le supprimer de la bibliothèque met le fichier à la corbeille.
- **Groupes d'écoute** : comptes sur un serveur et groupe rejoignable par un code (ou un lien `musicapp://join/CODE`).
  - Chaque membre télécharge les sons localement, et la lecture démarre en même temps chez tout le monde.
  - Tous les membres peuvent ajouter, mettre en pause, avancer dans le son ou passer au suivant.

## Installation

Le script installe les dépendances (`yt-dlp`, `ffmpeg`, `node`) et la dernière [release](https://github.com/OverCapi/music_app/releases) : rien à compiler. Par défaut il installe le **client** ; `--server` installe uniquement le serveur de groupes (Docker), `--both` les deux.

macOS / Linux :

```sh
curl -fsSL https://raw.githubusercontent.com/OverCapi/music_app/master/install.sh | bash                   # client
curl -fsSL https://raw.githubusercontent.com/OverCapi/music_app/master/install.sh | bash -s -- --server    # serveur
```

Windows (PowerShell) :

```powershell
irm https://raw.githubusercontent.com/OverCapi/music_app/master/install.ps1 -OutFile install.ps1
.\install.ps1                   # client (-Mode server | both)
```

Depuis un clone, `./install.sh --from-source` compile l'app (Rust, Node, pnpm, dépendances Tauri installés au besoin) au lieu de télécharger la release. `--version vX.Y.Z` choisit une autre release.

L'app n'est pas signée : sur macOS le script retire la quarantaine, sur Windows SmartScreen peut demander « Exécuter quand même ».

### Développement : prérequis manuels

- [Rust](https://rustup.rs/) (+ `cmake`, qui compile libopus), Node.js et [pnpm](https://pnpm.io/)
- Dépendances Tauri pour Linux (`webkit2gtk-4.1`, etc.) et `alsa-lib`
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) et `ffmpeg` dans le `PATH` (ou `~/.local/bin`, `/opt/homebrew/bin`, `~/Applications/yt-dlp`)
- `node`, recommandé : yt-dlp en a besoin comme moteur JavaScript pour télécharger depuis YouTube

## Lancer l'app

```sh
cd app                          # les commandes pnpm se lancent depuis app/
pnpm install
pnpm tauri dev
```

## Groupes d'écoute

Lancer le serveur :

```sh
cargo run -p music-group-server   # écoute sur 127.0.0.1:8787
```

Ou sur un serveur, avec Docker (écoute sur `0.0.0.0:8787`, base SQLite dans le volume `server-data`) :

```sh
docker compose up -d
```

Le serveur est indépendant de l'app : les clients saisissent son adresse dans « Groupe d'écoute ». Pour l'exposer sur Internet, le placer derrière un reverse proxy HTTPS (Caddy, nginx…).

Variables d'environnement du serveur :

| Variable | Défaut | Rôle |
| --- | --- | --- |
| `MUSIC_SERVER_ADDR` | `127.0.0.1:8787` | adresse d'écoute |
| `MUSIC_SERVER_DB` | `./server.db` | base SQLite des comptes |
| `MUSIC_SERVER_READY_TIMEOUT_MS` | `30000` | attente maximale des membres avant de lancer un son |

Dans l'app, ouvrir « Groupe d'écoute », se connecter (l'adresse du serveur est modifiable), puis créer un groupe ou en rejoindre un en collant le code ou le lien.

Pour tester à deux sur une même machine, lancer une seconde instance avec un profil séparé. Elle utilise le serveur Vite de la première instance, qui doit donc rester ouverte :

```sh
MUSIC_APP_PROFILE=b target/debug/music_app
```

## Développement

```sh
(cd app && pnpm check && pnpm lint && pnpm build)    # interface
cargo clippy --workspace --all-targets -- -D warnings      # tout le Rust
cargo test --workspace                                     # tests (régénère app/src/lib/bindings.ts)
```

Organisation du dépôt (un workspace Cargo) :

- `app/src/` : interface SvelteKit (routes dans `app/src/routes/`, stores dans `app/src/lib/stores/`, composants dans `app/src/lib/components/`, types et commandes générés dans `app/src/lib/bindings.ts`)
- `app/src-tauri/` : backend Rust de l'app, un module par fonctionnalité
- `crates/server/` : serveur des groupes d'écoute
- `crates/protocol/` : messages partagés entre l'app et le serveur

Voir [CLAUDE.md](CLAUDE.md) pour l'architecture détaillée et la marche à suivre pour ajouter une fonctionnalité.

Les évolutions prévues sont listées dans [TODO.md](docs/TODO.md).
