# TODO

## Groupes d'écoute

- [ ] Remplacer les codes de 6 caractères par des identifiants uniques (UID) pour rejoindre un groupe.
- [ ] Enregistrer le lien `musicapp://join/…` auprès du système (deep link) pour rejoindre en un clic.
- [ ] Limiter les tentatives de connexion sur le serveur (rate limiting) avant de le mettre sur internet.
- [ ] Déploiement du serveur : HTTPS/WSS, hébergement, sauvegarde de la base.
- [ ] Relancer automatiquement un téléchargement de groupe qui a échoué.
- [ ] En mode groupe, un double-clic dans la Bibliothèque ou une playlist devrait ajouter au groupe (ou être désactivé) au lieu de tenter une lecture locale.
- [ ] Resynchroniser en cours de son si l'écart d'horloge change (aujourd'hui, seul un changement d'état resynchronise).
- [ ] Reprendre la lecture locale après avoir quitté un groupe.
- [ ] Séparer aussi le stockage de la webview entre profils (`MUSIC_APP_PROFILE`).
- [ ] Pouvoir garder dans la bibliothèque un son écouté en groupe.

## Lecture et file d'attente

- [ ] Réordonner la file d'attente (glisser-déposer).
- [ ] Lecture aléatoire et répétition (off / tout / un titre).
- [ ] Bouton pour activer ou désactiver la lecture au hasard quand la file est vide.
- [ ] Sauvegarder la file d'attente entre deux lancements.
- [ ] Retirer de la file un titre supprimé de la bibliothèque.

## Bibliothèque et playlists

- [ ] Afficher les pochettes (intégrées aux fichiers téléchargés).
- [ ] Réordonner les titres d'une playlist.
- [ ] Ajouter plusieurs titres à la fois à une playlist.
- [ ] Ajouter un dossier entier à la bibliothèque.
- [ ] Signaler les fichiers déplacés ou supprimés hors de l'app.
- [ ] Lire les fichiers Opus nommés `.ogg` avec le décodeur Opus.

## Recherche et téléchargements

- [ ] Utiliser la vraie pochette de l'album plutôt qu'une image de la vidéo dans les résultats.
- [ ] Supprimer les fichiers `.part` laissés par un téléchargement annulé.

## Distribution

- [ ] Embarquer libopus dans l'app (feature `bundled` de `symphonia-adapter-libopus`) plutôt que dépendre du paquet système.
- [ ] Rendre configurable le chemin de yt-dlp.
