# InterviewScribe

InterviewScribe est une application locale de transcription d'entretiens, en developpement pour Windows, Linux et Android.

Elle vise a transformer une conversation en texte en temps reel ou depuis un enregistrement, distinguer les intervenants et produire deux sorties complementaires:

- une transcription brute, fidele et auditable;
- une transcription nettoyee, sans hesitations inutiles, sans modifier le sens.

## Objectifs du produit

- Transcrire en francais et dans d'autres langues avec Whisper.
- Detecter automatiquement le nombre d'intervenants.
- Associer chaque prise de parole a un intervenant modifiable.
- Activer ou masquer les horodatages.
- Enregistrer une conversation ou importer un fichier audio/video dans tout format courant.
- Exporter en TXT, Markdown, JSON, SRT, VTT, DOCX, DOC et PDF, au choix de l'utilisateur.
- Fonctionner localement par defaut afin de proteger les entretiens.
- Livrer une application Windows, un installateur Windows, un paquet Linux et une application Android.

## Etat du projet

Le prototype permet d’importer un fichier audio sur bureau, le transcrire avec Whisper local, conserver les segments dans SQLite et exporter en TXT, Markdown ou JSON. Large v3 Turbo est fourni avec les paquets. La capture microphone, la diarisation et le nettoyage reversible restent a implementer.

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

Les workflows GitHub Actions preparent les artefacts de version avec le modele integre. Une construction reussie ne remplace pas la validation de l’installation et du fonctionnement sur chaque plateforme cible.

## Developpement

Prerequis cibles: Node.js 22, pnpm, Rust stable et les dependances Tauri 2. Android necessite egalement Android Studio, le SDK et le NDK.

```bash
pnpm install
pnpm models:prepare
pnpm tauri dev
```

La transcription a posteriori est branchee au moteur natif; utiliser l’application Tauri pour acceder a l’import et aux fichiers locaux.

## Confidentialite

L'audio, les empreintes vocales et les transcriptions sont des donnees sensibles. Aucun envoi reseau ne doit etre ajoute par defaut. Toute fonctionnalite distante devra etre facultative, explicite et documentee.

## Modele integre et construction hors ligne

Les paquets incluent Whisper Large v3 Turbo Q5_0 (environ 575 Mo), avec sa
licence. Aucun telechargement de modele n’est demande a l’utilisateur apres
installation. Les entretiens sont transcrits localement.

Pour fabriquer un paquet, recuperer d’abord le modele avec `pnpm models:prepare`
(connexion necessaire sur la machine de construction). `pnpm models:verify`
controle sa taille et son SHA-256; cette verification est automatique avant le
build natif. Ne jamais ajouter le fichier `.bin` a Git. Les workflows de release
preparent le modele avant de construire les installateurs et l’APK.

L’APK necessite egalement de l’espace pour extraire le modele dans le stockage
prive. L’installation et les performances Windows/Android restent a valider sur
leurs plateformes cibles.
