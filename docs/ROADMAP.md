# Planification

## Phase 0 - Fondation

- [X] Definir la vision et les limites du produit.
- [X] Choisir Tauri 2, React, TypeScript et Rust.
- [X] Ajouter les contextes Claude/Codex et les skills du projet.
- [X] Definir la strategie de livraison Windows, Linux et Android.
- [X] Initialiser une application Tauri 2 executable sur bureau et Android.
- [X] Mettre en place lint, formatage, tests et integration continue.

## Phase 1 - Prototype a posteriori

- [X] Importer tout format audio courant (WAV, MP3, M4A, FLAC, OGG, AAC, etc.).
- [X] Integrer un modele Whisper local.
- [X] Fournir Large v3 Turbo Q5_0 dans les ressources du paquet, avec verification SHA-256 avant construction.
- [X] Valider une installation neuve hors connexion sur chaque plateforme cible.
- [X] Afficher les segments et horodatages.
- [X] Sauvegarder un projet dans SQLite.
- [X] Exporter en TXT, Markdown et JSON.
- [X] Tester plusieurs accents francais et niveaux de bruit.

Critere de sortie: un entretien mono-locuteur peut etre importe, transcrit, corrige, sauvegarde et exporte sans connexion. Valide sur desktop (build, lancement et transcription reelle verifies). Sur Android, `whisper-rs` compile et linke (voir Strategie Whisper dans `docs/ARCHITECTURE.md`), mais le picker de fichier renvoie une URI `content://` que l'import ne sait pas encore lire directement (copie fichier actuelle suppose un chemin disque reel) — a couvrir en Phase 6.

## Phase 2 - Edition et nettoyage

- [X] Ajouter l'editeur de segments.
- [X] Conserver le texte brut immuable (`segment.raw_text` n'est jamais reecrit; le texte affiche/exporte "nettoye" est toujours derive via la table `edit`. Reste a documenter dans l'UI que ce "brut" est celui produit par Whisper, pas un verbatim absolu: Whisper omet parfois hesitations/interruptions et peut halluciner une phrase en fin de silence — voir "Validation accents et bruit" dans `docs/ARCHITECTURE.md`).
- [X] Implementer les suppressions reversibles d'hesitations (hesitations lexicales, repetitions immediates, pauses sans contenu — jamais la formulation elle-meme).
- [X] Visualiser et annuler chaque modification (diff mot-a-mot affiche, annulation LIFO par segment).
- [X] Ajouter SRT, VTT, DOCX et PDF.
- [X] Ajouter DOC via conversion locale optionnelle (sans bloquer les autres formats si l'outil est absent).

Critere de sortie: aucune modification automatique ne peut detruire le texte source ou changer silencieusement le sens. Valide: 61 tests Rust (nettoyage, edition/annulation, chaque format d'export) et 12 tests d'interface passent; build desktop complet regenere avec succes apres l'ajout de `regex`, `docx-rs`, `genpdf` et `which`.

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
