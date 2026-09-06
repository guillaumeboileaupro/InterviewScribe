# Planification

## Phase 0 - Fondation

- [x] Definir la vision et les limites du produit.
- [x] Choisir Tauri 2, React, TypeScript et Rust.
- [x] Ajouter les contextes Claude/Codex et les skills du projet.
- [x] Definir la strategie de livraison Windows, Linux et Android.
- [x] Initialiser une application Tauri 2 executable sur bureau et Android.
- [x] Mettre en place lint, formatage, tests et integration continue.

## Phase 1 - Prototype a posteriori

- [x] Importer tout format audio courant (WAV, MP3, M4A, FLAC, OGG, AAC, etc.).
- [x] Integrer un modele Whisper local.
- [x] Fournir Large v3 Turbo Q5_0 dans les ressources du paquet, avec verification SHA-256 avant construction.
- [ ] Valider une installation neuve hors connexion sur chaque plateforme cible.
- [x] Afficher les segments et horodatages.
- [x] Sauvegarder un projet dans SQLite.
- [x] Exporter en TXT, Markdown et JSON.
- [ ] Tester plusieurs accents francais et niveaux de bruit.

Critere de sortie: un entretien mono-locuteur peut etre importe, transcrit, corrige, sauvegarde et exporte sans connexion. Valide sur desktop (build, lancement et transcription reelle verifies). Sur Android, `whisper-rs` compile et linke (voir Strategie Whisper dans `docs/ARCHITECTURE.md`), mais le picker de fichier renvoie une URI `content://` que l'import ne sait pas encore lire directement (copie fichier actuelle suppose un chemin disque reel) — a couvrir en Phase 6.

## Phase 2 - Edition et nettoyage

- [ ] Ajouter l'editeur de segments.
- [ ] Conserver le texte brut immuable.
- [ ] Implementer les suppressions reversibles d'hesitations.
- [ ] Visualiser et annuler chaque modification.
- [ ] Ajouter SRT, VTT, DOCX et PDF.
- [ ] Ajouter DOC via conversion locale optionnelle (sans bloquer les autres formats si l'outil est absent).

Critere de sortie: aucune modification automatique ne peut detruire le texte source ou changer silencieusement le sens.

## Phase 3 - Multi-locuteurs

- [ ] Integrer la diarisation locale.
- [ ] Estimer le nombre de locuteurs avec possibilite de correction.
- [ ] Renommer, fusionner et separer les locuteurs.
- [ ] Gerer explicitement les chevauchements et incertitudes.
- [ ] Constituer un jeu de tests multi-locuteurs non prive.

Critere de sortie: une discussion de deux a cinq personnes peut etre corrigee rapidement et exportee avec des etiquettes stables.

## Phase 4 - Temps reel

- [ ] Capturer le microphone avec sauvegarde incrementale.
- [ ] Ajouter VAD, fenetres glissantes et segments provisoires.
- [ ] Stabiliser le texte sans sauts visuels excessifs.
- [ ] Supporter pause, reprise, changement de peripherique et recuperation.
- [ ] Mesurer latence et consommation.

Critere de sortie: une session d'une heure reste stable et recuperable, avec une latence cible mesuree et documentee.

## Phase 5 - Livraison bureau

- [ ] Generer l'executable Windows.
- [ ] Generer l'installateur NSIS Windows.
- [ ] Generer le paquet Debian `.deb`.
- [ ] Tester installation, mise a niveau et desinstallation.
- [ ] Publier les sommes de controle des artefacts.

## Phase 6 - Android

- [ ] Lire les fichiers importes via une URI `content://` (pas seulement un chemin disque).
- [ ] Adapter capture, stockage, permissions et cycle de vie.
- [ ] Optimiser un modele quantifie pour telephone.
- [ ] Tester interruption, verrouillage d'ecran, batterie et temperature.
- [ ] Generer et signer l'APK.
- [ ] Documenter les limites selon la memoire du telephone.

## Definition de termine

Une tache n'est terminee que si elle comporte des tests pertinents, une documentation a jour, aucune donnee sensible dans les traces et une validation sur la plateforme concernee.

