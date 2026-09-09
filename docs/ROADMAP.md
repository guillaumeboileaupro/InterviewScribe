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

- [x] Integrer la diarisation locale (extraction d'empreintes vocales + clustering maison a centroides mobiles, voir "Diarisation" dans `docs/ARCHITECTURE.md`).
- [x] Estimer le nombre de locuteurs avec possibilite de correction (clustering automatique, indice optionnel "nombre de personnes" au lancement, ajout manuel d'un locuteur oublie).
- [x] Renommer, fusionner et separer les locuteurs (separation geree comme une reassignation manuelle segment par segment, pas un re-clustering automatique).
- [x] Gerer explicitement les chevauchements et incertitudes (statut `uncertain` quand la marge de confiance entre les deux meilleurs locuteurs candidats est trop faible; reste a affiner: pas de detection audio du chevauchement lui-meme en v1, voir limite ci-dessous).
- [ ] Constituer un jeu de tests multi-locuteurs non prive.

Critere de sortie: une discussion de deux a cinq personnes peut etre corrigee rapidement et exportee avec des etiquettes stables. Valide: 92 tests Rust (dont 10 sur le clustering, avec des embeddings fabriques a la main, aucun modele reel requis) et 16 tests d'interface passent; compilation complete (bibliotheque et binaire) verifiee apres l'ajout de `pyannote-rs`/`ort`; nouveau modele d'empreinte vocale (WeSpeaker CAM++, ~28 Mo) telecharge, verifie par somme de controle et integre au meme pipeline `pnpm models:prepare` que Whisper. Limite connue non encore couverte: pas de validation de bout en bout sur un enregistrement multi-locuteurs reel (le jeu de tests dedie reste a constituer), et la detection de chevauchement de voix reste indirecte (marge de confiance du clustering) plutot qu'une detection audio dediee.

## Phase 4 - Temps reel

- [x] Capturer le microphone avec sauvegarde incrementale (`cpal` + `hound`, WAV valide ecrit en continu; verifie sur peripherique audio reel, voir "Pipeline temps reel" dans `docs/ARCHITECTURE.md`).
- [x] Ajouter VAD, fenetres glissantes et segments provisoires (VAD par seuil d'energie avec hysteresis, decoupage declenche par le silence ou un plafond de duree).
- [x] Stabiliser le texte sans sauts visuels excessifs (chaque bloc ferme par un silence est definitif; interpretation pragmatique de "provisoire puis consolide" sans diff mot-a-mot, voir limite ci-dessous).
- [x] Supporter pause, reprise, changement de peripherique et recuperation (pause/reprise verifiees sur materiel reel; recuperation apres interruption reutilise le pipeline a posteriori existant sur le WAV partiel deja valide).
- [ ] Mesurer latence et consommation.

Critere de sortie: une session d'une heure reste stable et recuperable, avec une latence cible mesuree et documentee. Valide: 105 tests Rust (dont 17 nouveaux pour `capture::vad`/`capture::chunker`, logique pure sans materiel) et 17 tests d'interface passent; compilation complete verifiee apres l'ajout de `cpal`/`hound`; un test materiel reel (`capture::session::tests::real_microphone_capture_produces_a_valid_recoverable_wav`, `#[ignore]`) confirme sur ce poste: ouverture du peripherique par defaut, evenements de niveau reels, pause/reprise, et un WAV ecrit en continu relu avec succes par le decodeur a posteriori existant (`audio::decode::decode_to_mono_pcm16k`) - preuve concrete que la recuperation apres crash fonctionne, pas seulement en theorie.

Limites connues, non couvertes par cette passe: aucune mesure de latence/consommation avec de la parole reelle n'a ete produite (necessite qu'un humain parle dans le microphone, impossible a simuler dans l'environnement de developpement utilise ici) - a faire avant de considerer le critere de sortie entierement rempli. Le changement de peripherique en cours de session n'a pas ete teste sur du materiel reel (un seul microphone disponible sur ce poste). Aucune session d'une heure n'a ete testee en continu. Les metadonnees, segments stabilises et chemin du WAV partiel survivent bien a une reouverture SQLite, mais la reprise applicative reste a implementer: le pipeline a posteriori refuse actuellement un entretien reste au statut `transcribing`. Validation Android non tentee (voir `docs/ARCHITECTURE.md`).

## Phase 5 - Livraison bureau

- [x] Generer l'executable Windows (build reel sur runner `windows-latest`, jamais de compilation croisee depuis Linux - voir "Livraison" dans `docs/ARCHITECTURE.md`).
- [x] Generer l'installateur NSIS Windows (meme build, `--bundles nsis`).
- [x] Generer le paquet Debian `.deb` (build reel sur runner `ubuntu-22.04`).
- [x] Tester installation et desinstallation (verifie reellement en CI: installation silencieuse, presence de l'executable confirmee sur disque, desinstallation silencieuse, suppression confirmee - sur les deux plateformes). La mise a niveau N-1 vers N et la preservation des donnees utilisateur sont maintenant automatisees; leur execution native attend le prochain tag.
- [x] Publier les sommes de controle des artefacts (SHA-256, fichiers `.sha256` attaches a la release GitHub brouillon).

Valide reellement le 07/09/2026 via `.github/workflows/release.yml` (existait deja mais n'avait jamais ete execute - premiere execution reelle faite dans cette phase): release GitHub brouillon `v0.1.0` (non publique) avec les 4 fichiers (`.deb`, `.exe` NSIS, 2 sommes de controle). Deux bugs reels trouves et corriges pendant cette validation, invisibles avant un vrai build cible: `libasound2-dev` manquant sur les runners Ubuntu (cassait tout le CI depuis l'ajout de `cpal` en Phase 4) et `icon.ico` corrompu (jamais un vrai fichier ICO valide, ne plantait que sur un vrai build Windows avec l'erreur `RC2175`) - regenere via `tauri icon`.

Limites connues: les nouveaux tests de mise a niveau et de conservation a la desinstallation attendent leur premiere execution native sur un tag; l'aspect visuel de l'installateur (ecrans de l'assistant, raccourcis menu demarrer) n'est pas capture par une installation silencieuse automatisee, une verification humaine reste utile; aucune signature de code desktop configuree (hors perimetre, aucun secret ajoute au depot). L'APK Android arm64-v8a est construite et signee sans diarisation, `ort-sys` restant indisponible pour cette cible.

## Phase 6 - Android

- [x] Lire les fichiers importes via une URI `content://` (pas seulement un chemin disque). Code corrige (reutilise `tauri_plugin_fs`, meme mecanisme que le modele Whisper embarque) et compile reellement pour Android en CI. Un job d'installation/lancement sur emulateur arm64-v8a est configure; son execution et le parcours `content://` complet restent a valider, puis a reproduire sur appareil physique avec AND-04 de `E2E_TEST_CAMPAIGN.md`.
- [x] Adapter permissions et stockage. `RECORD_AUDIO` ajoutee de facon persistante (via `build.rs`, survit a la regeneration de `gen/android`); stockage prive utilise pour l'import et les modeles.
- [ ] Adapter capture et cycle de vie. La capture microphone (Phase 4) compile desormais pour Android (apres correction de `minSdkVersion`), mais son fonctionnement reel sur ce backend (AAudio) n'a jamais ete teste sur un appareil; le cycle de vie applicatif (mise en arriere-plan, interruption) n'a pas ete traite dans cette passe.
- [ ] Optimiser un modele quantifie pour telephone. Non fait: le meme modele Whisper Turbo Q5_0 que le bureau est utilise, sans variante plus legere specifique au telephone. La diarisation, elle, est desactivee sur Android (voir limite ci-dessous), donc son modele n'est pas charge la-bas.
- [ ] Tester interruption, verrouillage d'ecran, batterie et temperature. **Non fait dans cet environnement** : le job emulateur arm64-v8a couvre uniquement installation/lancement/desinstallation; interruption, verrouillage et mesures energetiques exigent encore un appareil physique.
- [x] Generer et signer l'APK. Verifie reellement: build signe via un keystore de test (secrets GitHub Actions, jamais commite), signature confirmee par `apksigner verify` en CI. Limite au ABI `arm64-v8a` (voir note ci-dessous).
- [x] Documenter les limites selon la memoire du telephone. Documentation analytique uniquement (tailles de modeles connues); **aucune mesure reelle de memoire/batterie/temperature sur appareil**, a la difference du reste du projet qui exige une verification reelle - ecart assume et signale, pas cache.

Limites reelles trouvees et traitees pendant cette phase (invisibles avant un vrai build cible, comme en Phase 5): la diarisation (`pyannote-rs`/`ort`) ne compile pas du tout pour Android - `ort-sys` ne publie aucun binaire precompile pour cette cible (verifie en lisant son `build.rs`, pas une hypothese) - donc **desactivee sur Android**, retour au comportement mono-locuteur de la Phase 1 (voir "Diarisation" dans `docs/ARCHITECTURE.md`). `cpal` (capture, Phase 4) necessite AAudio, absent du sysroot NDK avant l'API 26: `minSdkVersion` releve de 24 a 26 (Android 8.0+). L'architecture `armv7` (32 bits) rencontre un probleme distinct de generation de bindings (`bindgen`) sur le NDK du runner CI, non resolu; la publication se limite a `arm64-v8a`, qui couvre la quasi-totalite des appareils reels.

Critere de sortie: une APK signee, installable sur un appareil arm64-v8a, capable d'importer un fichier audio via le selecteur systeme et de le transcrire localement avec Whisper. **Non atteint entierement** : la production et la signature sont validees; l'installation/lancement sur emulateur est automatisee mais attend sa premiere execution native. L'import `content://`, la transcription, le microphone, les interruptions, le verrouillage, la batterie et la temperature restent a valider sur appareil physique avant de clore la Phase 6.

## Definition de termine

Release candidate actuelle : `v0.1.3`. Son workflow doit produire et tester le
paquet Debian, l'installateur NSIS Windows et l'APK arm64-v8a avant publication.
Les resultats natifs mettront a jour les validations encore ouvertes de la
section 7 dans `TEST_IMPLEMENTATION_PLAN.md`.

Une tache n'est terminee que si elle comporte des tests pertinents, une documentation a jour, aucune donnee sensible dans les traces et une validation sur la plateforme concernee.

La couverture, les niveaux de preuve et les validations encore necessaires sont
suivis dans [`TEST_STRATEGY.md`](TEST_STRATEGY.md). Les priorites P0 de cette
strategie font partie des criteres de sortie de la prochaine version.
Leur execution etape par etape est suivie dans
[`TEST_IMPLEMENTATION_PLAN.md`](TEST_IMPLEMENTATION_PLAN.md).
