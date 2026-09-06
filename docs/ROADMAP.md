# Planification

## Phase 0 - Fondation

- [x] Definir la vision et les limites du produit.
- [x] Choisir Tauri 2, React, TypeScript et Rust.
- [x] Ajouter les contextes Claude/Codex et les skills du projet.
- [x] Definir la strategie de livraison Windows, Linux et Android.
- [ ] Initialiser une application Tauri 2 executable sur bureau et Android.
- [ ] Mettre en place lint, formatage, tests et integration continue.

## Phase 1 - Prototype a posteriori

- [ ] Importer WAV et MP3.
- [ ] Integrer un modele Whisper local.
- [ ] Afficher les segments et horodatages.
- [ ] Sauvegarder un projet dans SQLite.
- [ ] Exporter en TXT, Markdown et JSON.
- [ ] Tester plusieurs accents francais et niveaux de bruit.

Critere de sortie: un entretien mono-locuteur peut etre importe, transcrit, corrige, sauvegarde et exporte sans connexion.

## Phase 2 - Edition et nettoyage

- [ ] Ajouter l'editeur de segments.
- [ ] Conserver le texte brut immuable.
- [ ] Implementer les suppressions reversibles d'hesitations.
- [ ] Visualiser et annuler chaque modification.
- [ ] Ajouter SRT et VTT.

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

- [ ] Adapter capture, stockage, permissions et cycle de vie.
- [ ] Optimiser un modele quantifie pour telephone.
- [ ] Tester interruption, verrouillage d'ecran, batterie et temperature.
- [ ] Generer et signer l'APK.
- [ ] Documenter les limites selon la memoire du telephone.

## Definition de termine

Une tache n'est terminee que si elle comporte des tests pertinents, une documentation a jour, aucune donnee sensible dans les traces et une validation sur la plateforme concernee.

