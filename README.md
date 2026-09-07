# InterviewScribe

InterviewScribe est une application locale de transcription d'entretiens pour Windows, Linux et Android. Elle transforme une conversation en texte (enregistrement microphone ou fichier importe), distingue les intervenants, et produit une transcription brute fidele ainsi qu'une version nettoyee (hesitations retirees, sens jamais modifie).

Tout tourne en local sur la machine de l'utilisateur. Aucune connexion reseau, aucun compte, aucun envoi d'audio ou de transcription vers un service distant.

## Telechargement et installation

Les paquets se trouvent sur la page [Releases](https://github.com/guillaumeboileaupro/InterviewScribe/releases/latest). Chaque version fournit, avec les modeles deja integres a l'interieur:

| Plateforme | Fichier a telecharger | Installation |
| --- | --- | --- |
| Windows | `InterviewScribe_x.y.z_x64-setup.exe` | Lancer l'installateur et suivre les etapes. |
| Linux (Debian/Ubuntu) | `InterviewScribe_x.y.z_amd64.deb` | `sudo dpkg -i InterviewScribe_x.y.z_amd64.deb` (ou double-clic dans le gestionnaire de paquets). |
| Android | `InterviewScribe_x.y.z_arm64-v8a.apk` | Telecharger l'APK sur l'appareil, autoriser l'installation depuis une source inconnue si demande, puis l'ouvrir pour installer. |

Aucun git, aucune compilation, aucune connexion reseau n'est necessaire pour installer ou utiliser l'application: le fichier telecharge est autosuffisant.

Chaque fichier est accompagne d'une somme de controle `.sha256`. Pour la verifier avant installation:

```bash
sha256sum -c InterviewScribe_x.y.z_amd64.deb.sha256
```

(remplacer par le nom du fichier correspondant a votre plateforme).

## Utilisation

1. Ouvrir InterviewScribe.
2. Creer un entretien: importer un fichier audio/video existant, ou lancer un enregistrement microphone (Windows/Linux).
3. Attendre la transcription locale (Whisper). Sur bureau, les intervenants sont automatiquement detectes et separes; ils restent renommables, fusionnables et reassignables a tout moment.
4. Relire et corriger si besoin: le texte brut original reste toujours consultable et n'est jamais modifie; les corrections sont des couches separees et annulables.
5. Exporter au format souhaite: TXT, Markdown, JSON, SRT, VTT, DOCX, PDF ou DOC.

## Confidentialite

L'audio, les empreintes vocales et les transcriptions sont des donnees sensibles. Rien n'est envoye sur le reseau par defaut, et aucune fonctionnalite distante ne sera ajoutee sans etre facultative, explicite et documentee.

## Documentation

- [Vision produit](docs/PRODUCT.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Planification](docs/ROADMAP.md)
- [Principes UI/UX](docs/UI_UX.md)

## Developpement (contributeurs)

Cette section ne concerne pas l'utilisateur final, seulement la fabrication du paquet.

Prerequis: Node.js 22, pnpm, Rust stable et les dependances Tauri 2 (Android necessite en plus Android Studio, le SDK et le NDK).

```bash
pnpm install
pnpm models:prepare   # telecharge les modeles (connexion necessaire uniquement ici)
pnpm tauri dev
```

`pnpm models:prepare` recupere Whisper Large v3 Turbo Q5_0 et, pour le bureau, le modele d'empreintes vocales WeSpeaker CAM++ (diarisation). `pnpm models:verify` controle leur taille et leur SHA-256; c'est fait automatiquement avant tout build natif. Ces fichiers ne doivent jamais etre ajoutes a Git. Les workflows GitHub Actions preparent les modeles avant de construire les installateurs et l'APK, et verifient reellement l'installation/desinstallation (Windows, Linux) avant publication.
