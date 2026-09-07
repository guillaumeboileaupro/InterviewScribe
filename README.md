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

Sur bureau (Windows, Linux): import audio ou capture microphone en direct, transcription Whisper locale, diarisation locale multi-locuteurs (renommage/fusion/reassignation), nettoyage reversible des hesitations, export TXT/Markdown/JSON/SRT/VTT/DOCX/PDF/DOC, et une chaine de publication reelle (executable et installateur NSIS Windows, paquet Debian) verifiee par installation et desinstallation automatisees sur des runners reels.

Sur Android: une APK signee (arm64-v8a) est produite et sa signature verifiee automatiquement a chaque publication, avec l'import de fichier via le selecteur systeme et la transcription Whisper locale corriges pour cette plateforme. La diarisation y est desactivee (limite technique documentee dans [Architecture](docs/ARCHITECTURE.md)) et **aucun test sur appareil ou emulateur physique n'a ete effectue** (uniquement verifie par compilation et signature reelles en CI) - a faire avant de considerer Android pleinement valide.

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

Les workflows GitHub Actions preparent les artefacts de version avec les modeles integres. Pour Windows et Linux, chaque publication installe et desinstalle reellement l'artefact produit (pas seulement une construction reussie) avant de le publier. L'APK Android est produite et signee, mais sa validation sur appareil physique reste a faire.

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

## Modeles integres et construction hors ligne

**Ce que recoit l'utilisateur final: uniquement un binaire installable** (l'executable
Windows, le paquet `.deb` ou l'APK), avec les modeles deja integres a l'interieur.
Aucun git, aucune compilation, aucune connexion reseau n'est necessaire pour
installer ou utiliser l'application - tout tourne en local sur sa machine, jamais
un service distant. Les paquets incluent Whisper Large v3 Turbo Q5_0 (environ
575 Mo) pour la transcription, et sur bureau uniquement, un second modele
d'empreintes vocales (WeSpeaker CAM++, environ 28 Mo) pour la diarisation -
chacun avec sa licence.

Ce qui suit ne concerne que la fabrication du paquet par le developpeur, jamais
l'utilisateur final: pour construire un paquet, recuperer d'abord les modeles
avec `pnpm models:prepare` (connexion necessaire uniquement sur la machine de
construction, jamais sur celle de l'utilisateur). `pnpm models:verify` controle
leur taille et leur SHA-256; cette verification est automatique avant le build
natif. Ne jamais ajouter ces fichiers a Git. Les workflows de release preparent
les modeles avant de construire les installateurs et l'APK.

L'APK necessite egalement de l'espace pour extraire le modele Whisper dans le
stockage prive (la diarisation n'est pas embarquee sur Android, voir
[Architecture](docs/ARCHITECTURE.md)). L'installation Windows et Linux est
verifiee automatiquement (installation, desinstallation, sommes de controle) a
chaque publication; la validation Android sur appareil physique reel reste a
faire.
