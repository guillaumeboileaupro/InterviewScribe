# Architecture cible

## Choix directeur

Tauri 2 permet de partager l'interface et une partie du coeur Rust entre le bureau et Android. Le moteur de traitement reste decouple afin de pouvoir utiliser des implementations adaptees a chaque plateforme.

## Composants

| Composant | Responsabilite |
| --- | --- |
| Interface React | capture des intentions, suivi temps reel, edition et export |
| Coeur Tauri/Rust | commandes natives, orchestration, fichiers, persistance et securite |
| Capture audio | microphone, normalisation, tampons et sauvegarde incrementale |
| VAD | detection des zones de parole et reduction des calculs inutiles |
| Transcription | inference Whisper locale et horodatage des segments |
| Diarisation | estimation du nombre de locuteurs et attribution des segments |
| Nettoyage | annotations reversibles des hesitations et repetitions |
| Persistance | SQLite pour les metadonnees et fichiers pour l'audio |
| Export | TXT, Markdown, JSON, SRT, VTT, DOCX, PDF et DOC (via conversion locale) |

## Pipeline temps reel

1. Capturer des trames audio courtes.
2. Ecrire immediatement l'audio dans un fichier recuperable.
3. Detecter l'activite vocale.
4. Assembler une fenetre glissante avec chevauchement.
5. Produire des segments provisoires avec Whisper.
6. Stabiliser les segments lorsque le contexte est suffisant.
7. Attribuer provisoirement un locuteur.
8. Persister et rafraichir l'interface sans bloquer la capture.

Le temps reel est une transcription incrementale avec une faible latence, pas une garantie mot par mot instantanee.

## Pipeline a posteriori

1. Decodage et normalisation en PCM mono, via un decodeur couvrant tout format audio et video courant.
2. Detection des zones de parole.
3. Transcription complete.
4. Alignement temporel.
5. Diarisation globale.
6. Reconciliation des segments et locuteurs.
7. Generation de la vue nettoyee.
8. Validation et export.

## Modele de donnees minimal

- `Interview`: titre, langue, dates, mode, chemin audio et etat.
- `Speaker`: identifiant, etiquette, couleur et nom choisi.
- `Segment`: debut, fin, texte brut, locuteur, confiance et etat.
- `Edit`: operation reversible appliquee au texte nettoye.
- `Setting`: horodatage, modele, langue, nettoyage et peripherique.

## Strategie Whisper

- Ne pas coupler l'application a une taille de modele unique.
- Telecharger un modele uniquement apres choix explicite de l'utilisateur.
- Verifier l'integrite du modele.
- Fournir un profil rapide et un profil precis selon les ressources.
- Sur Android, privilegier les modeles quantifies et mesurer memoire, batterie et temperature.
- Inference via `whisper-rs` (bindings whisper.cpp/GGML). Sa compilation croisee Android necessite `cmake` et `clang` installes, ainsi que la variable `CMAKE_TOOLCHAIN_FILE` pointant vers `android.toolchain.cmake` du NDK (sinon CMake echoue avec "Neither the NDK or a standalone toolchain was found"). Utiliser `scripts/tauri-android.sh` (au lieu de `pnpm tauri android ...` directement) qui detecte le NDK installe et charge `scripts/android-toolchain.cmake`, en local comme en CI. Ce fichier mappe la cible Cargo vers l’ABI Android (ARM64, ARMv7, x86 ou x86_64) avant de charger la toolchain NDK; sans ce mapping, le NDK peut compiler en ARMv7 meme pour une cible Rust ARM64.

## Diarisation

Whisper ne distingue pas a lui seul les personnes. La diarisation expose une interface stable (module `diarization`, distinct de `transcription`) et reste remplacable si une meilleure implementation locale apparait. Les recouvrements de voix ou les cas ambigus sont signales comme incertains (`segment.status = 'uncertain'`) plutot que forces vers un seul locuteur.

### Implementation (Phase 3, 7 septembre 2026)

- Approche retenue: extraction d'une empreinte vocale par segment Whisper (pas de modele de segmentation separe - les bornes temporelles de Whisper suffisent) puis clustering. Recherche comparative menee avant implementation:
  - `sherpa-onnx` (bindings Rust officiels du projet k2-fsa): pipeline de diarisation complet, mais binding Rust tres recent, telecharge un binaire precompile par defaut, et le support Android de ce binding specifique n'est pas documente. L'ancien crate tiers `sherpa-rs` qui aurait pu servir de repli est archive/deprecie depuis juin 2026.
  - `pyannote-rs` (MIT): retenu pour son `EmbeddingExtractor`, utilisable independamment de son propre modele de segmentation, sans compilation C++/cmake (a la difference de `whisper-rs-sys`). Son propre clustering (`EmbeddingManager`) est en revanche trop simpliste (similarite contre un embedding fixe, jamais mis a jour) - non reutilise.
- Clustering maison (`diarization::Clusterer`, logique pure sans dependance au modele, testee avec des embeddings fabriques a la main): centroides mobiles (moyenne ponderee, pas un point fixe), similarite cosinus, seuil de rattachement a un cluster existant, marge de confiance entre les deux meilleurs candidats pour decider du statut `uncertain`. Un indice utilisateur optionnel ("nombre de personnes attendu") plafonne le nombre de clusters crees sans jamais forcer une fusion silencieuse au-dela.
- Modele: `wespeaker_en_voxceleb_CAM++.onnx` (embeddings de locuteur, ~28 Mo, licence CC-BY-4.0 heritee de VoxCeleb), telecharge depuis les releases GitHub de `k2-fsa/sherpa-onnx`, verifie par somme de controle SHA-256, integre au meme pipeline `pnpm models:prepare` que Whisper (zero appel reseau dans le binaire livre - voir "Modele fourni avec l'application"). Native runtime: `ort` (bindings ONNX Runtime), epingle a `2.0.0-rc.10` car la version `2.0.0-rc.13` resolue par defaut casse la compilation de `pyannote-rs` (regression de bornes `Send`/`Sync` sur `OperatorDomain`/`ErasedOperator`, verifiee par compilation reelle avant de fixer la version).
- Separation d'un locuteur mal fusionne: geree comme une reassignation manuelle segment par segment (`reassign_segment_speaker`), pas comme un re-clustering automatique - plus simple et plus previsible pour l'utilisateur.
- Limite connue: pas de detection audio directe du chevauchement de voix en v1 (demanderait le modele de segmentation de pyannote en plus de l'embedding) - seule la marge de confiance du clustering declenche le statut incertain. Pas encore de jeu de tests multi-locuteurs reel constitue (voir `docs/ROADMAP.md` Phase 3); a couvrir avant de considerer la diarisation pleinement validee. Support Android non valide pour cette dependance (a la difference de `whisper-rs`): a verifier lors d'un prochain passage, sans bloquer la disponibilite desktop.

## Export

- TXT, Markdown, JSON, SRT, VTT, DOCX et PDF sont generes nativement, sans dependance externe.
- DOC (format Word binaire historique) n'a pas de bibliotheque Rust fiable pour l'ecrire directement: il est produit en generant d'abord le DOCX puis en le convertissant localement (par exemple via LibreOffice en ligne de commande) si un convertisseur est installe.
- L'absence du convertisseur ne doit jamais bloquer les autres formats: seul l'export DOC est indisponible, avec un message clair a l'utilisateur.
- Le format d'export est un choix explicite de l'utilisateur, independant du format de capture ou d'import.

### Implementation (Phase 2, 6 septembre 2026)

- DOCX: `docx-rs` 0.4. Le paquetage exige un `std::io::Cursor<Vec<u8>>` (pas un
  `Vec<u8>` seul, qui n'implemente pas `Seek`).
- PDF: `genpdf` 0.2 avec les polices DejaVu Sans (regulier + gras) embarquees
  via `include_bytes!` (`src-tauri/resources/fonts/`, ~1,46 Mo, licence
  permissive Bitstream Vera/DejaVu, commitees directement comme le logo — pas
  comme le modele Whisper). Verifie avec `pdfinfo`/`pdftotext`: pagination
  automatique correcte, accents francais corrects dans le corps du texte.
  `genpdf` n'est plus maintenu depuis 2021 (dependance `printpdf` 0.3.4
  epinglee): risque accepte pour une tache d'ecriture de document hors-ligne
  sans entree non fiable; repli documente si besoin futur: `printpdf` 0.12
  (activement maintenu) et son mode HTML-to-PDF. Limite connue: la metadonnee
  "Titre" du PDF mal-encode les caracteres accentues (bug de `printpdf` 0.3.4
  independant du rendu du corps du texte, qui lui est correct) — volontairement
  omise plutot que de livrer une metadonnee corrompue.
- DOC: pas de bibliotheque Rust pour ecrire le format binaire historique.
  Genere le DOCX puis lance `soffice --headless --convert-to doc` (verifie
  localement: ~2,4s, produit un vrai fichier OLE "MS Word 97", signature
  `D0 CF 11 E0 A1 B1 1A E1`). Detection via la crate `which`; absence de
  `soffice` -> message clair, n'affecte jamais les autres formats. Le frontend
  interroge la disponibilite au chargement de la page pour griser le bouton
  proactivement plutot que d'echouer silencieusement.
- SRT/VTT: aucune dependance, formats texte simples.

## Securite et confidentialite

- Aucun enregistrement ou texte dans les journaux.
- Chiffrement offert pour les projets locaux.
- Permissions microphone demandees au moment utile.
- Aucun trafic reseau pendant une transcription locale, hors telechargement explicite d'un modele.
- Suppression coordonnee de la base, de l'audio, des caches et des exports geres.


## Modele fourni avec l’application

Le modele par defaut est **Whisper Large v3 Turbo multilingue quantifie Q5_0**
(574 041 195 octets, environ 575 Mo). Il remplace Small. Sa revision distante,
sa taille et son empreinte SHA-256 sont epinglees dans
`src-tauri/resources/models/manifest.json`. La licence MIT de Whisper est livree
avec le modele.

Turbo est le variant qu'OpenAI a specifiquement concu pour etre rapide tout en
restant multilingue: d'apres le depot officiel OpenAI, Turbo tourne environ
8x plus vite que Large (809M parametres contre 1550M), la ou les variants
plus petits et plus rapides (Tiny, Base, Small) sont limites a l'anglais.
C'est le compromis vitesse/qualite retenu pour ce projet, confirme lors de
l'echange avec l'utilisateur du 06/09/2026 ("je veux juste une version locale
qui tourne vite et assez bonne") -> decision de garder Large v3 Turbo plutot
que de redescendre vers Small.

Les paquets Windows, Linux et Android incluent le modele dans les ressources
Tauri. L’application ne telecharge aucun modele : le producteur du paquet lance
`pnpm models:prepare` avant la construction. Le hook `beforeBuildCommand`
verifie la taille et le SHA-256 et refuse de fabriquer un paquet incomplet.
Les poids et les fichiers temporaires restent ignores par Git.

Sur bureau, Whisper lit directement la ressource installee. Sur Android, le
plugin filesystem ouvre la ressource APK et le code Rust la copie par flux dans
le stockage prive, verifie son empreinte puis effectue un renommage atomique.
La copie valide est reutilisee; une copie corrompue est reparee depuis l’APK.
Prevoir de la place pour l’APK, la copie extraite et le cache temporaire du
plugin filesystem lorsque la ressource APK est compressee. Aucune connexion ni action
de telechargement n’est necessaire au premier usage. La copie, la verification
et l’inference se font hors du fil de l’interface.

Whisper seul ne fournit pas de diarisation : la separation des locuteurs est
geree par un second modele bundle de la meme maniere (`wespeaker_en_voxceleb_CAM++.onnx`,
voir "Diarisation" ci-dessus). Les performances et la memoire sur telephone restent a
mesurer sur appareil cible; la presence des modeles ne valide pas le support Android
pour la diarisation (a la difference de Whisper, deja compile et teste sur Android).

Verification locale facultative avec l’echantillon public `jfk.wav` de
whisper.cpp (ne pas ajouter l’audio ni les poids au depot) :

```bash
pnpm models:prepare
INTERVIEWSCRIBE_TEST_MODEL="$PWD/src-tauri/resources/models/ggml-large-v3-turbo-q5_0.bin" \
INTERVIEWSCRIBE_TEST_WAV=/chemin/jfk.wav \
cargo test --manifest-path src-tauri/Cargo.toml --lib whisper_smoke -- --ignored
```

En developpement, preparer le modele puis lancer `pnpm tauri dev`. Le serveur
Vite seul ne fournit pas les commandes natives. En debug, si les ressources ne
sont pas encore copiees par Tauri, le moteur cherche le modele dans le dossier
source `src-tauri/resources/models`. Ce repli est absent des versions release.

Sources : [modeles whisper.cpp](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md),
[ressources Tauri](https://v2.tauri.app/develop/resources/),
[depot officiel OpenAI Whisper](https://github.com/openai/whisper) (tailles de
modeles, vitesses relatives et licence MIT d'origine).

### Validation du modele integre (6 septembre 2026)

- Test de parole public JFK reussi avec Large v3 Turbo Q5_0 sur CPU Linux.
- 30 tests Rust, 8 tests d’interface et 3 cas de verification de packaging reussis.
- Paquet Debian debug construit; modele et licence presents, SHA-256 du contenu verifie.
- Binaire du paquet Linux demarre sous Xvfb avec un profil temporaire (arret apres 12 secondes sans erreur).
- APK debug construit pour ARM64; modele et licence presents, SHA-256 du contenu verifie.
- Installation neuve, extraction et inference sur appareil Android, ainsi que validation Windows, restent a effectuer.

Ces artefacts debug sont des paquets de validation, pas une publication signee.

### Validation accents et bruit (6 septembre 2026)

Passage manuel avec Large v3 Turbo Q5_0 sur CPU Linux, sur des enregistrements
publics (domaine public LibriVox, jamais ajoutes au depot, cf. AGENTS.md) via
l'outil `whisper_qa_sample` (`src-tauri/src/transcription/whisper_cpp.rs`,
`cargo test -- --ignored --nocapture whisper_qa_sample`):

- **Francais quebecois** (Filiatreault, *Contes, anecdotes et recits
  canadiens*, lu par une locutrice quebecoise): transcription quasi parfaite,
  confiance >0.9998 sur l'ensemble du passage.
- **Francais standard/europeen** (Chateaubriand, *Voyage en Italie*, texte
  litteraire du XIXe siecle avec citations latines): transcription quasi
  parfaite sur le francais; seules les citations en latin sont degradees
  (attendu, ce n'est pas la langue cible).
- **Bruit synthetique** (bruit blanc mixe numeriquement au signal propre, a un
  rapport signal/bruit controle, plutot que de chercher des enregistrements
  bruites reels sans texte de reference): a 10 dB SNR (bruit de fond notable),
  aucune degradation mesurable. A 0 dB SNR (bruit aussi fort que la voix),
  degradation progressive et localisee en fin de passage plutot
  qu'un echec brutal — la premiere moitie du texte reste quasi intacte.
- Une etude independante du CNRS sur la transcription d'entretiens avec
  Whisper (source ci-dessous) mesure des taux d'erreur de 1,4 a 3,5% avec
  `large-v2` contre 5,8 a 8,1% avec `small` et 10,5 a 14,3% avec `base` sur des
  entretiens reels (studio et micro-trottoir bruite). Ceci confirme
  empiriquement le choix de conserver un modele de categorie Large (Turbo)
  plutot que de redescendre vers un modele plus petit pour la vitesse.

**Limite connue de Whisper, a garder en tete pour l'invariant "texte brut
jamais reecrit" (AGENTS.md):** Whisper n'est pas un transcripteur mot-a-mot.
D'apres la meme etude CNRS, il omet couramment les hesitations, les relances
rapides et interruptions, et certains mots de liaison; il confond parfois des
noms propres. On observe aussi, en fin de passage sur silence ou bruit fort,
une hallucination connue du modele (ici "Sous-titrage Societe Radio-Canada",
absente de l'enregistrement d'origine). La transcription "brute" stockee dans
`segment.raw_text` est donc le brut *de Whisper*, pas un verbatim absolu de
l'audio — a documenter clairement pour l'utilisateur plutot que de laisser
croire a une fidelite parfaite.

Non teste dans cette passe: accents africains/belges/suisses (aucune source
publique fiable identifiee rapidement) et bruit non-blanc (brouhaha de
conversation, musique).

Sources : [CNRS CSS — Whisper pour retranscrire des entretiens](https://www.css.cnrs.fr/whisper-pour-retranscrire-des-entretiens/),
[LibriVox](https://librivox.org/) via [Internet Archive](https://archive.org/details/librivoxaudio) (enregistrements du domaine public utilises pour ce test, non conserves).
