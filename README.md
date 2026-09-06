# InterviewScribe

InterviewScribe est une application locale de transcription d'entretiens, disponible sur Windows, Linux et Android.

Elle transforme une conversation en texte en temps reel ou depuis un enregistrement, distingue les intervenants et produit deux sorties complementaires:

- une transcription brute, fidele et auditable;
- une transcription nettoyee, sans hesitations inutiles, sans modifier le sens.

## Objectifs du produit

- Transcrire en francais et dans d'autres langues avec Whisper.
- Detecter automatiquement le nombre d'intervenants.
- Associer chaque prise de parole a un intervenant modifiable.
- Activer ou masquer les horodatages.
- Enregistrer une conversation ou importer un fichier audio/video.
- Exporter en TXT, Markdown, JSON, SRT et VTT.
- Fonctionner localement par defaut afin de proteger les entretiens.
- Livrer une application Windows, un installateur Windows, un paquet Linux et une application Android.

## Etat du projet

Le depot contient actuellement le cadrage fonctionnel, l'architecture cible, les consignes Claude/Codex, les skills du projet et le squelette de l'interface. L'integration du moteur audio et de Whisper constitue la premiere phase de developpement.

Consultez:

- [Vision produit](docs/PRODUCT.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Planification](docs/ROADMAP.md)
- [Principes UI/UX](docs/UI_UX.md)

## Architecture cible

- Tauri 2 et Rust pour le coeur multiplateforme.
- React et TypeScript pour l'interface.
- Whisper/whisper.cpp pour la transcription locale.
- Un module de diarisation distinct pour les locuteurs.
- SQLite pour les projets, segments, intervenants et reglages.

## Formats de livraison

| Plateforme | Format |
| --- | --- |
| Windows | executable `.exe` et installateur NSIS `.exe` |
| Ubuntu/Debian | paquet `.deb` |
| Android | paquet `.apk` |

Les artefacts seront generes par GitHub Actions a chaque version publiee. Ils ne sont pas encore disponibles tant que le moteur natif n'est pas integre.

## Developpement

Prerequis cibles: Node.js 22, pnpm, Rust stable et les dependances Tauri 2. Android necessite egalement Android Studio, le SDK et le NDK.

```bash
pnpm install
pnpm dev
```

Le squelette ne pretend pas encore fournir une transcription fonctionnelle. Les commandes natives sont ajoutees progressivement selon la feuille de route.

## Confidentialite

L'audio, les empreintes vocales et les transcriptions sont des donnees sensibles. Aucun envoi reseau ne doit etre ajoute par defaut. Toute fonctionnalite distante devra etre facultative, explicite et documentee.
