# InterviewScribe

InterviewScribe est une application locale de transcription d'entretiens pour Windows, Linux et Android. Elle transforme une conversation en texte (enregistrement microphone ou fichier importe), distingue les intervenants, et produit une transcription brute fidele ainsi qu'une version nettoyee (hesitations retirees, sens jamais modifie).

Tout tourne en local sur la machine de l'utilisateur. Aucune connexion reseau, aucun compte, aucun envoi d'audio ou de transcription vers un service distant.

## Telecharger

Version publique actuelle: [InterviewScribe v0.1.0](https://github.com/guillaumeboileaupro/InterviewScribe/releases/tag/v0.1.0).

| Plateforme | Fichier a telecharger | Installation |
| --- | --- | --- |
| Windows | [`InterviewScribe_0.1.0_x64-setup.exe`](https://github.com/guillaumeboileaupro/InterviewScribe/releases/download/v0.1.0/InterviewScribe_0.1.0_x64-setup.exe) | Lancer l'installateur et suivre les etapes. |
| Linux (Debian/Ubuntu) | [`InterviewScribe_0.1.0_amd64.deb`](https://github.com/guillaumeboileaupro/InterviewScribe/releases/download/v0.1.0/InterviewScribe_0.1.0_amd64.deb) | `sudo dpkg -i InterviewScribe_0.1.0_amd64.deb` (ou double-clic dans le gestionnaire de paquets). |
| Android | [`InterviewScribe_0.1.0_arm64-v8a.apk`](https://github.com/guillaumeboileaupro/InterviewScribe/releases/download/v0.1.0/InterviewScribe_0.1.0_arm64-v8a.apk) | Telecharger sur l'appareil, autoriser l'installation depuis une source inconnue si demande, puis ouvrir pour installer. |

Aucun git, aucune compilation, aucune connexion reseau n'est necessaire pour installer ou utiliser l'application: le fichier telecharge est autosuffisant, avec les modeles deja integres a l'interieur.

L'installateur Windows n'est pas encore signe numeriquement. Windows SmartScreen peut afficher un avertissement au premier lancement, et le navigateur peut aussi signaler le fichier comme peu telecharge (avertissement lie a la nouveaute du fichier, pas a un probleme reel): utiliser « Conserver quand meme » dans la barre de telechargement. Verifier que le fichier provient bien de la page officielle de la release ci-dessus avant de l'executer. Le `.exe` est construit et son installation/desinstallation verifiees automatiquement par la CI, mais n'a pas encore ete teste manuellement sur une machine Windows.

L'APK Android n'est pas distribue via le Play Store: il est signe avec une cle de release dediee au projet, mais Android affichera un avertissement car l'application ne provient pas d'un magasin reconnu. Autoriser l'installation depuis le navigateur ou le gestionnaire de fichiers (« Installer des applications inconnues ») pour ce fichier uniquement, apres avoir verifie qu'il provient bien de la page officielle de la release. Compatible Android 8.0 (API 26) et plus, architecture ARM64 (arm64-v8a) uniquement — **aucun test sur appareil ou emulateur physique n'a ete effectue**, uniquement verifie par compilation et signature reelles en CI (voir [Architecture](docs/ARCHITECTURE.md)).

Chaque fichier est accompagne d'une somme de controle `.sha256`. Pour la verifier avant installation:

```bash
sha256sum -c InterviewScribe_0.1.0_amd64.deb.sha256
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
- [Strategie de test](docs/TEST_STRATEGY.md)
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
