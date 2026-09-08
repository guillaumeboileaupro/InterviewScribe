# Plan d'implementation des tests

## Regle d'execution

Ce document est la liste de travail faisant autorite pour terminer la strategie
de test. Les taches sont executees dans l'ordre. Une case n'est cochee que si le
code, le test et la preuve correspondante existent. Les validations materielles
restent marquees comme telles jusqu'a leur execution reelle.

Etats utilises :

- `[x]` termine et verifie ;
- `[ ]` a faire ;
- `EN COURS` identifie l'unique etape active ;
- `MATERIEL` exige une machine, un peripherique ou une intervention humaine.

## 0. Socle de test — termine

- [x] Creer la matrice de tracabilite dans `docs/TEST_STRATEGY.md`.
- [x] Ajouter la commande globale `pnpm check`.
- [x] Verifier l'immutabilite de `raw_text` a travers nettoyage, edition,
  fusion, export et annulation.
- [x] Verifier la persistance SQLite des segments apres reouverture.
- [x] Verifier la suppression coordonnee SQLite/audio, le refus des chemins
  externes et le rollback sur erreur.
- [x] Tester la confirmation UI, le focus sur l'action sure, Echap, succes et
  erreur de suppression.
- [x] Activer une CSP locale et ajouter les gardes statiques de confidentialite.
- [x] Ajouter Axe et couvrir bibliotheque, dialogue, enregistrement et entretien
  long en erreur.
- [x] Verifier l'option d'horodatage transmise a l'export sans perte du texte.
- [x] Creer `docs/E2E_TEST_CAMPAIGN.md` pour les validations manuelles reelles.

Preuves principales : commits `e2deb74`, `e8c06c5`, `214577d`, `38f998d`.

## 1. Recuperation apres interruption — EN COURS

Objectif : rendre un enregistrement interrompu recuperable sans modifier ni
supprimer les segments bruts deja stabilises et sans creer de doublons.

- [x] 1.1 Definir et tester l'etat interrompu derive, distinct d'une capture
  encore active, sans reecrire le statut persiste.
- [x] 1.2 Exposer uniquement les sessions `realtime + transcribing` sans capture
  active comme candidates recuperables apres redemarrage.
- [x] 1.3 Exposer une commande de recuperation qui valide d'abord le WAV partiel.
- [x] 1.4 Conserver tous les segments existants et calculer la derniere frontiere
  audio stabilisee.
- [x] 1.5 Transcrire uniquement la partie non stabilisee, avec timestamps
  recalages et garde anti-doublon.
  - [x] Decouper le PCM exactement apres la derniere frontiere stable.
  - [x] Recaler les timestamps et filtrer les doublons de frontiere.
  - [x] Cabler ces regles a la commande Whisper de recuperation.
- [ ] 1.6 Reconstituer une attribution de locuteur prudente : attribution
  incertaine plutot que fusion forcee si l'identite ne peut pas etre reliee.
- [ ] 1.7 Ajouter l'action UI `Recuperer`, ses etats occupes/erreur et une option
  `Conserver en l'etat`.
- [ ] 1.8 Tester : aucun segment, segments partiels, WAV absent/corrompu, silence
  final, interruption repetee et export apres recuperation.
- [ ] 1.9 `MATERIEL` Executer fermeture forcee pendant une vraie capture, relancer
  l'application et verifier audio, segments et export.

Critere de sortie : aucune perte de donnees, aucun `raw_text` modifie, aucun
doublon aux frontieres et erreur locale explicite si le WAV est inutilisable.

## 2. E2E desktop automatise

- [ ] 2.1 Choisir et documenter le pilote compatible Tauri 2/WebDriver.
- [ ] 2.2 Ajouter un profil de donnees temporaire et des fixtures synthetiques.
- [ ] 2.3 Automatiser lancement, bibliotheque et navigation sous Linux.
- [ ] 2.4 Automatiser import, edition, nettoyage, locuteurs, export et suppression.
- [ ] 2.5 Ajouter le job E2E Linux a la CI avec artefacts de diagnostic sur echec.
- [ ] 2.6 Porter la meme suite sur `windows-latest`.
- [ ] 2.7 Verifier qu'aucun audio/transcript n'apparait dans les artefacts CI.

Critere de sortie : la meme suite pilote des applications Tauri construites sur
Linux et Windows, et non un frontend simule seul.

## 3. Corpus audio public et mesures

- [ ] 3.1 Definir le manifeste : URL, licence, SHA-256, langue, locuteurs et cas.
- [ ] 3.2 Selectionner des extraits redistribuables mono-locuteur et 2 a 5 voix.
- [ ] 3.3 Ajouter silence, bruit, musique et chevauchement, sans donnee privee.
- [ ] 3.4 Implementer normalisation texte et calcul WER/CER.
- [ ] 3.5 Implementer DER, confusion de locuteur et couverture `uncertain`.
- [ ] 3.6 Mesurer doublons et derive des horodatages aux frontieres.
- [ ] 3.7 Fixer des seuils initiaux documentes, puis faire echouer les regressions.
- [ ] 3.8 Executer la qualification lourde manuellement ou la nuit, jamais sur
  chaque commit.

Critere de sortie : chaque mesure est reproductible depuis un manifeste epingle
et ne depend d'aucune donnee sensible.

## 4. Audio et erreurs d'integration

- [ ] 4.1 Ajouter de petites fixtures WAV, MP3, M4A, FLAC, OGG et AAC autorisees.
- [ ] 4.2 Tester mono, stereo, frequences d'echantillonnage et durees extremes.
- [ ] 4.3 Tester fichiers tronques, vides, extension trompeuse et permission refusee.
- [ ] 4.4 Simuler disque plein/ecriture et verifier la propagation sans perte.
- [ ] 4.5 Tester erreur d'un segment sans perte du reste de l'entretien.

## 5. Accessibilite et rendu reel

- [ ] 5.1 Etendre Axe aux etats chargement, pause, termine et reglages.
- [ ] 5.2 Tester l'ordre de tabulation et le retour du focus apres dialogue.
- [ ] 5.3 Tester les raccourcis clavier documentes.
- [ ] 5.4 `MATERIEL` Auditer contraste et lisibilite en clair/sombre/systeme.
- [ ] 5.5 `MATERIEL` Verifier les largeurs 360 px, tablette et bureau avec long texte.
- [ ] 5.6 `MATERIEL` Tester lecteur d'ecran sur Windows et Android.

## 6. Performance et endurance

- [ ] 6.1 Ajouter un collecteur sans contenu sensible : latence, CPU, memoire,
  batterie, temperature et volume disque.
- [ ] 6.2 Definir seuils p50/p95 et budget memoire par plateforme.
- [ ] 6.3 Executer une session synthetique longue sans microphone.
- [ ] 6.4 `MATERIEL` Executer une session parlee d'une heure sous Linux et Windows.
- [ ] 6.5 `MATERIEL` Tester changement et disparition du microphone.
- [ ] 6.6 `MATERIEL` Mesurer batterie/temperature et cycle de vie Android.

## 7. Packaging, mise a niveau et premier lancement

- [ ] 7.1 Construire une fixture de base/projet au schema N-1.
- [ ] 7.2 Installer N-1 puis N sous Linux et verifier migration et donnees.
- [ ] 7.3 Installer N-1 puis N sous Windows et verifier migration et donnees.
- [ ] 7.4 Lancer chaque artefact installe et verifier le modele hors connexion.
- [ ] 7.5 Verifier desinstallation sans effacer les donnees utilisateur hors scope.
- [ ] 7.6 Tester l'APK sur emulateur arm64-v8a.
- [ ] 7.7 `MATERIEL` Tester installation, import `content://` et transcription sur
  un appareil Android physique.

## 8. Organisation CI finale

- [ ] 8.1 Conserver `pnpm check` obligatoire sur chaque pull request.
- [ ] 8.2 Ajouter E2E desktop court sur pull request.
- [ ] 8.3 Ajouter workflow nocturne pour corpus et performance.
- [ ] 8.4 Ajouter workflow manuel pour tests modele/materiel avec compte rendu.
- [ ] 8.5 Publier rapports WER/CER/DER, performance et packaging sans donnees brutes.
- [ ] 8.6 Bloquer une release si une preuve obligatoire manque, sans bloquer sur
  une validation explicitement classee `MATERIEL` hors environnement.

## Journal d'avancement

| Date | Etape | Resultat |
| --- | --- | --- |
| 2026-09-08 | 0 | Socle, confidentialite, suppression, Axe et campagne manuelle termines. |
| 2026-09-08 | 1.1-1.2 | Etat derive et commande `list_recovery_candidates` implementes et testes. |
| 2026-09-08 | 1.3-1.4 | Inspection du WAV prive, rejet des fichiers invalides et frontiere stable implementes et testes. |
| 2026-09-08 | 1.5 (partiel) | Decoupe PCM, recalage temporel et garde anti-doublon implementes comme regles pures testees. |
| 2026-09-08 | 1.5 | Commande Whisper de reprise cablee au suffixe audio et persistance additive implementee. |
