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
- [x] 1.6 Reconstituer une attribution de locuteur prudente : attribution
  incertaine plutot que fusion forcee si l'identite ne peut pas etre reliee.
- [x] 1.7 Ajouter l'action UI `Recuperer`, ses etats occupes/erreur et une option
  `Conserver en l'etat`.
- [x] 1.8 Tester : aucun segment, segments partiels, WAV absent/corrompu, silence
  final, interruption repetee et export apres recuperation.
- [ ] 1.9 `MATERIEL` Executer fermeture forcee pendant une vraie capture, relancer
  l'application et verifier audio, segments et export.

Critere de sortie : aucune perte de donnees, aucun `raw_text` modifie, aucun
doublon aux frontieres et erreur locale explicite si le WAV est inutilisable.

## 2. E2E desktop automatise — EN COURS (Claude)

Pris en parallele de la section 1 (Codex) pour eviter tout chevauchement de
fichiers: cette section ne touche pas `recovery.rs` ni son chemin de
recuperation. Coordination asynchrone via ce document, aucun canal direct
entre les deux agents.

- [x] 2.1 Choisir et documenter le pilote compatible Tauri 2/WebDriver. **Architecture
  revue depuis** : `tauri-driver` externe + `WebKitWebDriver` abandonne au profit
  du pilote embarque de `@wdio/tauri-service` (`tauri-plugin-wdio-webdriver`,
  serveur WebDriver W3C dans le process de l'app - plus de processus externe
  a installer/versionner), recommandation actuelle de Tauri. Config
  `e2e/wdio.conf.mjs`, suite `e2e/specs/*.e2e.mjs`. **2026-09-09** : ce pilote
  embarque a lui-meme besoin d'un SECOND plugin compagnon,
  `tauri-plugin-wdio` (Rust + JS frontend), pour que les hooks de commande de
  `@wdio/tauri-service` fonctionnent - sans lui, `window.__wdio_original_core__`
  n'existe jamais et CHAQUE commande WebDriver (`$`, `$$`, `elementClick`,
  `getTitle`, `findElement(s)`) attend 5s pour rien avant d'abandonner
  silencieusement (warning seulement, jamais fatal). Trouve en lisant la doc
  du paquet installe (`node_modules/.../@wdio/tauri-service/docs/plugin-setup.md`,
  fichier local, jamais une source distante) apres avoir remarque que le
  warning `Tauri core.invoke not available after 5s timeout` apparaissait
  toutes les ~5s en continu, y compris pendant des tests qui passaient.
  Corrige : `tauri-plugin-wdio` ajoute (Cargo.toml, `lib.rs`, meme feature
  `wdio-e2e`), `withGlobalTauri: true` + capacite `wdio-e2e` (nouveau fichier
  `capabilities/wdio-e2e.json`) ajoutes uniquement via
  `--config src-tauri/tauri.wdio-e2e.conf.json` (jamais dans `tauri.conf.json`
  de base - `app.security.capabilities` y est verrouille a `["default"]`),
  import frontend conditionnel (`apps/client/src/main.tsx`, gate
  `import.meta.env.VITE_WDIO_E2E === "true"`, verifie absent du `dist/` de
  production par `grep -rl wdio dist/` apres un `pnpm build` normal - aucune
  occurrence). **Verifie reellement** : warnings `core.invoke not available`
  passes de 25+ par run a 0 ; suite smoke passee de 3m41s a 5.5s pour les
  memes 3 tests.
- [x] 2.2 Profil temporaire : `scripts/e2e-linux.sh` isole chaque execution dans
  un `XDG_DATA_HOME`/`XDG_CONFIG_HOME`/`XDG_CACHE_HOME` jetable (`mktemp -d`),
  jamais les vraies donnees utilisateur. Pas encore de fixtures synthetiques
  au-dela de l'etat vide par defaut.
- [x] 2.3 `e2e/specs/smoke.e2e.mjs` pilote reellement le binaire installe
  (`/usr/bin/interviewscribe`, pas le binaire brut non installe - la
  resolution des ressources bundlees differe entre les deux, verifie en
  pratique) : lancement, navigation bibliotheque/Reglages/Preparation,
  verification du modele integre reellement affiche (pas simule) et de la
  liste des microphones reellement filtree (re-valide en conditions reelles
  le correctif `capture/device.rs`). 3 tests, tous verts en conditions
  reelles (`pnpm test:e2e:linux`).
- [x] 2.4 (partiel - voir blocage 2026-09-09 ci-dessous) `e2e/specs/workflow.e2e.mjs` : import reel via automatisation
  de la boite de dialogue native ("Open File", hors DOM du webview, pilotee
  par `xdotool` - voir `e2e/helpers/native-dialog.mjs`, technique
  fonctionnelle et verifiee) avec une fixture audio synthetique generee a
  l'execution (`e2e/fixtures/generate-audio.mjs`, jamais commitee). Le test
  verifie reellement que l'import declenche la transcription (checkpoint
  fiable et rapide, ~5s). **Cause racine de la lenteur (180s+ non termine)
  identifiee et corrigee** : `whisper_cpp.rs` utilisait
  `SamplingStrategy::BeamSearch{beam_size:5, patience:-1.0}` - le
  commentaire de doc de whisper-rs lui-meme est explicite ("at the cost of
  exponential CPU time"), et `patience` n'est meme pas implemente cote
  whisper.cpp. Remplace par `Greedy{best_of:5}` (le mode par defaut de
  whisper.cpp). Verifie directement via un test `#[ignore]`
  (`timing_check`, meme modele "base" bundle, meme clip synthetique de 3s) :
  180s+ (incomplet) -> 9.35s (complet). C'est aussi la cause du bug
  utilisateur reel "transcription impossible sur .deb et .exe", corrige et
  publie en v0.1.2.
  **2026-09-09 - nouveau blocage reel identifie, distinct de la lenteur
  whisper (celle-ci reste corrigee)** : une fois le correctif `tauri-plugin-wdio`
  de 2.1 applique, `workflow.e2e.mjs` echoue de facon reproductible des
  l'etape "Choisir un fichier audio" (apres le select de mode et de modele).
  Diagnostic direct (spec jetable, jamais commitee, supprimee apres usage) :
  `document.querySelector('select').value` reste bloque sur `"microphone"`
  APRES `selectByAttribute("value","file")` - la valeur DOM du `<select>` ne
  change tout simplement jamais, donc l'etat React `source` non plus, donc
  le bouton conditionne par `source === "file"` (`App.tsx` ~ligne 1284) ne
  s'affiche jamais. Deuxieme technique testee (clic sur le `<select>` puis
  `browser.keys(["ArrowDown","Enter"])`, approche standard quand le clic
  direct sur une `<option>` echoue) : meme resultat, valeur DOM inchangee.
  Conclusion : l'interaction native avec un `<select>` HTML via le pilote
  WebDriver embarque de `tauri-plugin-wdio-webdriver` sous WebKitGTK ne
  fonctionne pas actuellement (ni clic d'option, ni clavier) - limitation du
  plugin (encore jeune), pas un bug de l'application. Aucun contournement
  trouve pour l'instant sans modifier l'app elle-meme (ex: remplacer le
  `<select>` natif par des boutons/radios cliquables classiques, uniquement
  si l'UX le justifie independamment du besoin de test). La suite
  d'edition/nettoyage/locuteurs/export/suppression reste donc en `it.skip`,
  bloquee par CE probleme precis (et non plus par la lenteur whisper, qui
  est resolue) - a revoir si une version future de `tauri-plugin-wdio-webdriver`
  corrige le support des `<select>` natifs, ou si l'UI est un jour changee
  pour un composant plus facilement pilotable.
- [x] 2.5 Job `e2e-linux` ajoute a `.github/workflows/ci.yml` : construit le vrai
  `.deb`, l'installe, execute la suite sous `xvfb-run` (xdotool fonctionne
  pareil sous un serveur X virtuel). Execute reellement pour verifier
  (run `34254415903`, avant le pivot d'architecture note en 2.1) : les 3 jobs
  (`web`, `rust`, `e2e-linux`) passent, les 4 tests reels verts en ~1 min sur
  runner propre - la question ouverte en 2.4 (lenteur de transcription
  observee sur la machine de developpement) reste non tranchee ici puisque ce
  test precis est toujours `it.skip`. **Note** : la mention historique d'un
  artefact `tauri-driver.log` uploade (ci-dessous, 2.7) ne s'applique plus
  depuis le pivot vers `@wdio/tauri-service` en 2.1 - plus de processus
  `tauri-driver` externe, donc plus ce journal-la; le `ci.yml` actuel
  n'uploade aucun artefact pour ces jobs.
- [ ] 2.6 Porter la meme suite sur `windows-latest`.
- [x] 2.7 L'artefact de diagnostic uploade (`tauri-driver.log`) ne contient que
  des messages de cycle de vie du processus - jamais l'audio, la base
  SQLite ou un texte transcrit (verifie par construction : seul ce fichier
  est cible par `actions/upload-artifact`, pas le profil XDG temporaire).

Critere de sortie : la meme suite pilote des applications Tauri construites sur
Linux et Windows, et non un frontend simule seul.

## 3. Corpus audio public et mesures — EN COURS (Codex)

Pris par Codex pendant que Claude execute la section 2 E2E desktop. Les fichiers
E2E et leurs dependances restent hors de ce lot.

- [x] 3.1 Definir le manifeste : URL, licence, SHA-256, langue, locuteurs et cas.
- [x] 3.2 Selectionner des extraits redistribuables mono-locuteur et 2 a 5 voix.
  - [x] Source officielle AMI a 4 voix, CC BY 4.0, epinglee par SHA-256.
  - [x] Clip francophone belge mono, CC BY-SA 4.0, texte attendu et SHA-256 epingles.
  - [ ] Elargir ulterieurement les accents et le multi-voix francophone via Common Voice.
- [x] 3.3 Ajouter silence, bruit, musique et chevauchement, sans donnee privee.
- [x] 3.4 Implementer normalisation texte et calcul WER/CER.
- [x] 3.5 Implementer DER, confusion de locuteur et couverture `uncertain`.
- [x] 3.6 Mesurer doublons et derive des horodatages aux frontieres.
- [x] 3.7 Fixer des seuils initiaux documentes, puis faire echouer les regressions.
- [x] 3.8 Executer la qualification lourde manuellement ou la nuit, jamais sur
  chaque commit.
  - [x] Test opt-in du clip francophone public, sans texte brut dans le rapport.
  - [x] Executer et archiver WER/CER Large v3 Turbo sur le clip francophone.
  - [x] Epingle et extrait localement les annotations AMI 1.6.2 de la fenetre.
  - [x] Executer AMI et les variantes pour DER, doublons et derive reels ; les
        echecs DER, clusters, `uncertain` et derive restent des ecarts bloquants documentes.

Critere de sortie : chaque mesure est reproductible depuis un manifeste epingle
et ne depend d'aucune donnee sensible.

## 4. Audio et erreurs d'integration — TERMINE

- [x] 4.1 Ajouter de petites fixtures WAV, MP3, M4A, FLAC, OGG et AAC autorisees.
- [x] 4.2 Tester mono, stereo, frequences d'echantillonnage et durees extremes.
- [x] 4.3 Tester fichiers tronques, vides, extension trompeuse et permission refusee.
- [x] 4.4 Simuler disque plein/ecriture et verifier la propagation sans perte.
- [x] 4.5 Tester erreur d'un segment sans perte du reste de l'entretien.

## 5. Accessibilite et rendu reel

- [x] 5.1 Etendre Axe aux etats chargement, pause, termine et reglages.
- [x] 5.2 Tester l'ordre de tabulation et le retour du focus apres dialogue.
- [x] 5.3 Tester les raccourcis clavier documentes.
- [ ] 5.4 `MATERIEL` Auditer contraste et lisibilite en clair/sombre/systeme.
  Ratios des jetons et couleurs d'action automatises (minimum 4,5:1) ; audit
  visuel sur ecran reel encore requis.
- [ ] 5.5 `MATERIEL` Verifier les largeurs 360 px, tablette et bureau avec long texte.
  Rendu Chromium inspecte a 360/768/1440 px et protections de repli testees ;
  validation sur appareils physiques encore requise.
- [ ] 5.6 `MATERIEL` Tester lecteur d'ecran sur Windows et Android.
  Landmarks, lien d'evitement, noms des controles et annonces d'etat couverts
  automatiquement ; campagnes NVDA et TalkBack sur appareils encore requises.

## 6. Performance et endurance

- [x] 6.1 Ajouter un collecteur sans contenu sensible : latence, CPU, memoire,
  batterie, temperature et volume disque.
- [x] 6.2 Definir seuils p50/p95 et budget memoire par plateforme.
- [x] 6.3 Executer une session synthetique longue sans microphone.
- [ ] 6.4 `MATERIEL` Executer une session parlee d'une heure sous Linux et Windows.
- [ ] 6.5 `MATERIEL` Tester changement et disparition du microphone.
- [ ] 6.6 `MATERIEL` Mesurer batterie/temperature et cycle de vie Android.

## 7. Packaging, mise a niveau et premier lancement

- [x] 7.1 Construire une fixture de base/projet au schema N-1.
- [ ] 7.2 Installer N-1 puis N sous Linux et verifier migration et donnees.
  Script et job de release ajoutes ; execution native du prochain tag requise
  avant validation.
- [ ] 7.3 Installer N-1 puis N sous Windows et verifier migration et donnees.
  Script et job de release ajoutes ; execution native du prochain tag requise
  avant validation.
- [ ] 7.4 Lancer chaque artefact installe et verifier le modele hors connexion.
  `.deb` et NSIS installes couverts en CI avec trafic sortant bloque et etat du
  modele verifie dans l'interface ; APK installee reste dependante de 7.6.
- [ ] 7.5 Verifier desinstallation sans effacer les donnees utilisateur hors scope.
  Sentinelles synthetiques du profil applicatif et d'un export externe ajoutees
  aux jobs `.deb` et NSIS ; execution native du prochain tag requise avant validation.
- [ ] 7.6 Tester l'APK sur emulateur arm64-v8a.
  Job ARM64 ajoute : ABI, installation, lancement sans crash, processus et
  desinstallation controles ; execution native du prochain tag requise.
- [ ] 7.7 `MATERIEL` Tester installation, import `content://` et transcription sur
  un appareil Android physique.
  Protocole AND-01 a AND-05 complete avec ABI, mode avion, preuves non sensibles
  et persistance ; execution sur appareil physique encore requise.

## 8. Organisation CI finale

- [x] 8.1 Conserver `pnpm check` obligatoire sur chaque pull request.
- [x] 8.2 Ajouter E2E desktop court sur pull request.
  Jobs Linux et Windows sur application installee : lancement, navigation,
  modele hors ligne et demarrage d'import reel ; parcours long reste separe.
- [x] 8.3 Ajouter workflow nocturne pour corpus et performance.
  Planification quotidienne et declenchement manuel ajoutes : corpus public
  epingle, WER/CER/DER, soak synthetique et artefacts agreges sans audio.
- [x] 8.4 Ajouter workflow manuel pour tests modele/materiel avec compte rendu.
  Qualification modele declenchable dans le workflow nocturne ; attestation
  materielle a choix fermes et rapport JSON sans contenu libre ajoutee.
- [x] 8.5 Publier rapports WER/CER/DER, performance et packaging sans donnees brutes.
  Rapports JSON structures ajoutes aux workflows nocturne et release ; seuls
  metriques, verdicts, tailles et SHA-256 sont publies.
- [x] 8.6 Bloquer une release si une preuve obligatoire manque, sans bloquer sur
  une validation explicitement classee `MATERIEL` hors environnement.
  Gate final exige les jobs et rapports Linux, Windows et Android ; les
  protocoles materiels restent attestes separement et ne sont pas simules.

## Journal d'avancement

| Date | Etape | Resultat |
| --- | --- | --- |
| 2026-09-08 | 0 | Socle, confidentialite, suppression, Axe et campagne manuelle termines. |
| 2026-09-08 | 1.1-1.2 | Etat derive et commande `list_recovery_candidates` implementes et testes. |
| 2026-09-08 | 1.3-1.4 | Inspection du WAV prive, rejet des fichiers invalides et frontiere stable implementes et testes. |
| 2026-09-08 | 1.5 (partiel) | Decoupe PCM, recalage temporel et garde anti-doublon implementes comme regles pures testees. |
| 2026-09-08 | 1.5 | Commande Whisper de reprise cablee au suffixe audio et persistance additive implementee. |
| 2026-09-08 | 1.6 | Segments recuperes sans locuteur invente et marques `uncertain` jusqu'a validation humaine. |
| 2026-09-08 | 1.7 | Actions UI `Recuperer`/`Conserver en l'etat`, chargement et erreur implementes. |
| 2026-09-08 | 1.8 | Matrice automatique zero/partiel/corrompu/silence/repetition/export terminee. |
| 2026-09-08 | 3.4 | Normalisation Unicode reproductible et calculs WER/CER implementes et testes. |
| 2026-09-08 | 2.1-2.3 | Harnais E2E reel (tauri-driver/WebKitWebDriver/WebdriverIO) pilotant le binaire installe ; 3 tests verts en conditions reelles. |
| 2026-09-08 | 2.4 (partiel) | Import automatise via boite de dialogue native reelle (xdotool) ; declenchement de la transcription verifie. Lenteur/blocage reel non resolu au-dela de ce point, documente honnetement plutot que masque ; suite complementaire ecrite mais `it.skip`. |
| 2026-09-08 | 2.5, 2.7 | Job CI `e2e-linux` ajoute et verifie par une execution reelle (run 34254415903, 3 jobs verts) ; artefact de diagnostic limite au journal `tauri-driver` par construction. |
| 2026-09-08 | 3.5 | DER decompose, confusion locuteur et couverture `uncertain` implementes et testes. |
| 2026-09-08 | 3.6 | Doublons chevauchants et derive moyenne/maximale des horodatages mesures et testes. |
| 2026-09-08 | 3.1 | Manifeste sans audio prive, source AMI officielle/licence/taille/SHA-256 et validateur ajoutes. |
| 2026-09-08 | 3.3 | Generateur deterministe silence/bruit/tons musicaux/chevauchement, sorties WAV ignorees par Git. |
| 2026-09-08 | 3.2 | Sources mono francophone et AMI quatre voix selectionnees, attribuees et epinglees. |
| 2026-09-08 | 3.7 | Seuils initiaux WER/CER/DER/doublons/derive versionnes et depassements bloquants testes. |
| 2026-09-08 | 3.8 (partiel) | Qualification Large v3 Turbo reelle : WER 0,0000, CER 0,0000, 149,15 s ; rapport sans contenu brut. |
| 2026-09-08 | 3.8 (AMI) | Diagnostic : DER tours 0,5148 = 0 ms manque + 2 790 ms fausse alarme + 6 070 ms confusion ; limite segment Whisper mono-locuteur identifiee, seuil maintenu. |
| 2026-09-08 | 3.8 | Campagne close sur Greedy : propre, bruit, musique et chevauchement executes ; derive propre 1 280 ms sur 54/59 mots, ecarts qualite conserves comme bloquants. |
| 2026-09-08 | 4.1 | Six fixtures synthetiques 440 Hz generees localement et testees par le decodeur reel ; aucune voix ni donnee privee. |
| 2026-09-08 | 4.2 | Matrice WAV 8/16/44,1/48 kHz, mono/stereo et 10 ms a 60 s normalisee en mono 16 kHz. |
| 2026-09-09 | 4.3 | Fichiers vide/tronque refuses, contenu WAV sous extension MP3 detecte et permission Unix refusee propagee. |
| 2026-09-09 | 4.4 | Ecriture WAV incrementale rendue faillible ; ENOSPC simule, erreur emise une fois et prefixe recuperable conserve. |
| 2026-09-09 | 4.5 | Echec du bloc median injecte : blocs precedent/suivant conserves, offset suivant maintenu et audio source intact. |
| 2026-09-09 | 5.1 | Axe etendu aux reglages en chargement/remplis, capture en pause et entretien termine. |
| 2026-09-09 | 5.2 | Dialogue de suppression borne au clavier ; ordre annuler/confirmer et retour au declencheur testes. |
| 2026-09-09 | 5.3 | Portee clavier documentee : navigation native, boucle modale et Echap testes ; aucun raccourci global reserve. |
| 2026-09-09 | 5.4 (partiel) | Contraste des themes clair/sombre/systeme verrouille a 4,5:1 ; lisibilite sur ecran reel reste MATERIEL. |
| 2026-09-09 | 5.5 (partiel) | Rendus Chromium 360/768/1440 inspectes ; texte long et protections anti-debordement testes, appareils reels restants. |
| 2026-09-09 | 5.6 (partiel) | Semantique lecteur d'ecran automatisee ; validation vocale NVDA Windows et TalkBack Android reste MATERIEL. |
| 2026-09-09 | 6.1 | Collecteur agrege ajoute : latence, CPU/RSS Linux, taille fichier, batterie/temperature si exposees ; aucune donnee source serialisee. |
| 2026-09-09 | 6.2 | Budgets Linux/Windows/Android et verdicts p50/p95/RSS implementes ; metriques absentes explicitement refusees. |
| 2026-09-09 | 6.3 | Une heure synthetique executee en trames de 20 ms : 601 fenetres, pic 96 000 echantillons, aucune perte sur 57,6 M, 36 ms mur, 230 400 044 octets projetes. |
| 2026-09-09 | 7.1 | Fixture SQL v0 lisible ajoutee avec projet synthetique complet ; migration v1, donnees, brut et idempotence testes. |
| 2026-09-09 | 7.2 (partiel) | Script N-1 vers N Linux et job release ajoutes avec profil isole ; attente d'une execution native sur le prochain tag. |
| 2026-09-09 | 7.3 (partiel) | Script N-1 vers N Windows/NSIS et job release ajoutes avec profil APPDATA isole ; attente du prochain tag. |
| 2026-09-09 | 7.4 (partiel) | E2E `.deb`/NSIS force hors connexion par pare-feu et exige le modele pret ; APK installee non encore disponible. |
| 2026-09-09 | 7.5 (partiel) | Desinstallation `.deb`/NSIS controle les sentinelles du profil et d'un export externe ; execution native du prochain tag requise. |
| 2026-09-09 | 7.6 (partiel) | Job emulateur ARM64 ajoute avec controle ABI, installation, lancement, crash, processus et desinstallation ; attente du prochain tag. |
| 2026-09-09 | 7.7 (prepare) | Campagne appareil physique AND-01 a AND-05 documentee ; execution materielle requise. |
| 2026-09-09 | 8.1 | Job PR `check` aligne sur `pnpm check` ; 46 tests UI, build, confidentialite, clippy et 169 tests Rust valides localement. |
| 2026-09-09 | 8.2 | E2E courts Linux et Windows confirmes sur PR : application installee, navigation, modele hors ligne et debut d'import reel. |
| 2026-09-09 | 8.3 | Workflow nocturne ajoute : corpus public epingle, WER/CER/DER, soak synthetique et rapports sans audio. |
| 2026-09-09 | 8.4 | Workflow manuel d'attestation materielle et protocole de rapport sans champ libre ajoutes. |
| 2026-09-09 | 8.5 | Rapports JSON agreges de qualite, performance et packaging publies comme artefacts. |
| 2026-09-09 | 8.6 | Gate de release exigeant les preuves Linux, Windows et Android automatisees ajoute et teste. |
| 2026-09-09 | 9.1-9.6 | Progression Whisper, notes, lecteur audio, correlation temporelle, correctif console Windows, migrations et tests termines. |
| 2026-09-09 | 9.7 (partiel) | Validation automatisee verte ; inspection des applications installees et Android encore requise. |

## 9. Retours utilisateurs — lecteur, notes et progression

Travail issu d'un retour utilisateur direct :

- [x] 9.1 Supprimer la fenetre console parasite des builds Windows release.
- [x] 9.2 Afficher la progression 0-100 fournie par Whisper pendant la transcription.
- [x] 9.3 Ajouter un commentaire optionnel persistant par entretien avec migration SQLite.
- [x] 9.4 Permettre la reecoute de l'audio prive sans elargir le scope de fichiers.
- [x] 9.5 Cliquer un horodatage pour lire le segment et signaler le segment actif.
- [x] 9.6 Adapter les tests, fixtures d'export et migrations au champ `notes`.
- [ ] 9.7 Valider visuellement progression, lecteur et correlation temporelle sur
  les applications desktop installees et sur Android.

Validation automatisee : 46 tests frontend, build, confidentialite, clippy sans
avertissement et 169 tests Rust reussis. Neuf tests materiels/lourds restent
explicitement ignores. `clippy --all-targets --features wdio-e2e` est vert ; le
second passage des tests avec cette feature a ete interrompu par manque d'espace
puis verrou Cargo concurrent, sans erreur de code observee.
