# Strategie de test

## Objectif

Cette strategie relie les exigences produit aux preuves attendues. Un test est
considere comme valide uniquement lorsqu'il a ete execute dans l'environnement
qu'il pretend couvrir. Une compilation Android ne vaut donc pas un test sur
appareil, et la creation d'un paquet ne vaut pas son installation.

Les donnees de test doivent etre synthetiques ou publiques et redistribuables.
Aucun audio, transcript, nom reel, empreinte vocale, secret ou chemin sensible
ne doit apparaitre dans Git, les captures CI ou les journaux.

## Etats et vocabulaire

| Etat | Signification |
| --- | --- |
| Automatise | Execute sans intervention et echoue explicitement en cas d'ecart. |
| Manuel | Protocole reproductible necessitant une personne ou du materiel. |
| Planifie | Cas identifie mais pas encore implemente ou execute. |
| Valide | Preuve obtenue sur la plateforme et l'artefact annonces. |
| Bloque | Environnement ou materiel identifie indisponible; jamais assimile a un succes. |

Les tests `#[ignore]` sont des tests opt-in. Ils ne participent pas au resultat
de `cargo test` normal et leur execution doit etre rapportee separement.

## Pyramide de test

### Tests unitaires

- Rust: logique pure du VAD, fenetrage audio, nettoyage, clustering, modeles de
  donnees, migrations et serialisation des exports.
- React/TypeScript: rendu, interactions clavier, etats et adaptation des
  reponses ou erreurs Tauri.
- Node: manifeste des modeles, sommes de controle et erreurs de packaging.
- Ils n'utilisent ni microphone, ni reseau, ni gros modele.

### Tests d'integration

- SQLite en fichier temporaire: creation, migration, cascade, historique et
  reprise apres erreur.
- Audio synthetique: import, decodage, reechantillonnage, VAD, fenetrage et WAV
  recuperable.
- Pipeline texte: segment Whisper simule, attribution, nettoyage reversible,
  persistance puis export.
- Frontend: commandes Tauri simulees, avec succes, latence et erreur.

### Tests fonctionnels et E2E

- Fonctionnels headless: parcours utilisateur complet avec front et backend
  controles, sans dependance au modele reel.
- E2E desktop: application Tauri construite et pilotee sur Linux et Windows.
- E2E Android: APK installee et pilotee sur emulateur pour les fonctions sans
  materiel, puis appareil physique pour le microphone et le cycle de vie.
- Les selecteurs natifs, permissions, WebView et installateurs ne sont declares
  valides que dans un environnement natif reel.

### Qualification audio et non fonctionnelle

- Corpus public versionne par manifeste, jamais par donnees privees.
- Qualite de transcription: WER et CER, avec seuils fixes par corpus et modele.
- Diarisation: DER, confusion de locuteur, nombre de locuteurs et couverture
  des statuts `uncertain`.
- Temps reel: latence p50/p95, memoire maximale, CPU, taille du WAV et absence
  de perte sur une session d'une heure.
- Android physique: batterie, temperature, verrouillage, appel/interruption,
  arriere-plan et reprise.

## Matrice de tracabilite

| ID | Exigence / invariant | Niveau minimal | Plateforme | Etat actuel | Preuve ou lacune principale |
| --- | --- | --- | --- | --- | --- |
| TXT-01 | `segment.raw_text` reste immuable | Unit + integration DB | Toutes | Automatise | Test transversal DB: nettoyage, edition, fusion, exports brut/nettoye et annulation. |
| TXT-02 | Nettoyage reversible sans changement silencieux du sens | Unit + fonctionnel | Toutes | Automatise partiel | Couverture Rust et UI; enrichir le corpus de formulations ou les hesitations sont signifiantes. |
| TXT-03 | Horodatages conserves lorsqu'ils sont masques | Unit + fonctionnel | Toutes | Automatise | Masquage sur entretien persiste teste: texte conserve et option `showTimestamps: false` transmise a l'export; JSON/SRT/VTT conservent leurs horodatages par contrat backend. |
| SPK-01 | Aucun nom de locuteur invente | Unit + fonctionnel | Toutes | Automatise | Tests Rust et UI de renommage. |
| SPK-02 | Fusion, reattribution et incertitude sont explicites | Unit + fonctionnel | Desktop | Automatise | Clustering synthetique et parcours UI couverts. |
| SPK-03 | Discussion publique de 2 a 5 personnes | Qualification audio | Desktop | Planifie | Corpus multi-locuteurs non prive absent. |
| AUD-01 | Formats courants decodes en mono PCM 16 kHz | Integration | Desktop | Automatise partiel | WAV couvert; ajouter fixtures courtes MP3, M4A, FLAC, OGG et AAC sous licences compatibles. |
| AUD-02 | Silence, bruit et audio corrompu echouent proprement | Unit + integration | Toutes | Automatise partiel | VAD synthetique couvert; matrice de formats et erreurs a completer. |
| CAP-01 | Capture, pause, reprise et WAV recuperable | Integration + materiel | Desktop | Valide Linux partiel | Test microphone Linux opt-in execute; Windows et changement de peripherique restent a valider. |
| CAP-02 | Aucune perte apres interruption | Integration + E2E | Toutes | Automatise partiel | WAV et reouverture SQLite avec segments partiels couverts; la reprise applicative d'un statut `transcribing` reste a implementer. |
| CAP-03 | Session d'une heure stable | Soak test | Desktop | Automatise partiel | Une heure synthetique VAD/fenetrage passee sans perte (601 fenetres, pic 96 000 echantillons); collecteur et budgets disponibles. Session murale avec modele et microphone reste materielle. |
| TRN-01 | Whisper local produit texte et horodatages | Integration modele | Desktop | Valide Linux | `whisper_smoke` opt-in avec modele et audio publics. Windows reste a qualifier. |
| TRN-02 | Aucun doublon aux frontieres de fenetres | Unit + integration | Toutes | Automatise partiel | Chunker couvert; ajouter oracle texte sur chevauchements successifs. |
| EXP-01 | TXT, MD, JSON, SRT, VTT, DOCX et PDF restent lisibles | Unit + integration | Desktop | Automatise | Tests Rust; conserver validation structurelle et texte extrait. |
| EXP-02 | DOC absent ne bloque aucun autre export | Unit + fonctionnel | Desktop | Automatise | Backend et bouton desactive couverts; conversion LibreOffice reste opt-in. |
| UI-01 | Etats vide, chargement, enregistrement, pause, erreur et termine | Component + E2E | Toutes | Automatise partiel | Parcours UI couverts; Axe couvre vide, enregistrement et erreur avec 40 segments. Chargement lent et reprise restent a ajouter. |
| UI-02 | Clavier, focus visible, WCAG AA et cible tactile 44 px | Component + audit | Toutes | Automatise partiel | Axe couvre les principaux etats; landmarks, champs, focus modal/Echap et cibles 44 px sont testes. Ratios clair/sombre/systeme et rendus Chromium 360/768/1440 sont controles; NVDA, TalkBack et appareils reels restent manuels. |
| PRIV-01 | Aucun contenu sensible dans les logs ou le reseau | Integration + audit | Toutes | Automatise partiel | CSP limite `connect-src` a IPC; sources de production sans client reseau ni journalisation. Ajouter un test dynamique avec proxy bloque. |
| DEL-01 | Suppression coordonnee DB, audio, cache et exports geres | Integration + E2E | Toutes | Automatise | Backend: cascade SQLite et audio prive; chemin externe refuse et rollback testes. UI: confirmation, succes et erreur testes. Les exports choisis par l'utilisateur sont explicitement hors gestion. |
| PKG-01 | Installation neuve et desinstallation du paquet | Packaging | Linux/Windows | Valide en CI | `.deb` et NSIS verifies silencieusement sur runners natifs. |
| PKG-02 | Mise a niveau conserve les donnees et migre le schema | Packaging + E2E | Linux/Windows/Android | Planifie | Installer N-1 puis N et verifier projet, schema et desinstallation. |
| AND-01 | APK signee arm64-v8a | Packaging | Android | Valide en CI | Signature verifiee avec `apksigner`; fonctionnement non implique. |
| AND-02 | Import `content://` puis transcription hors ligne | E2E | Android | Planifie | Compilation seulement; test emulateur/appareil absent. |
| AND-03 | Micro, verrouillage, interruption, batterie et temperature | Materiel | Android | Bloque | Appareil physique necessaire. |

## Suites et commandes

### Controle local et pull request

```bash
pnpm check
```

Cette commande execute lint, format, tests frontend et packaging, build web,
formatage/clippy/tests Rust. Elle ne telecharge pas les modeles et n'execute pas
les tests ignores.

### Modele Whisper reel

```bash
pnpm models:verify
INTERVIEWSCRIBE_TEST_MODEL=/chemin/modele.bin \
INTERVIEWSCRIBE_TEST_WAV=/chemin/audio-public.wav \
cargo test --manifest-path src-tauri/Cargo.toml --lib whisper_smoke -- --ignored
```

### Microphone reel

```bash
cargo test --manifest-path src-tauri/Cargo.toml \
  real_microphone_capture_produces_a_valid_recoverable_wav -- --ignored --nocapture
```

Le test doit etre lance dans un environnement ou l'enregistrement est annonce
et consenti. Le rapport conserve seulement mesures et resultat, jamais l'audio.

## Frequence CI

| Declencheur | Suites obligatoires |
| --- | --- |
| Commit local | Tests lies au changement; `pnpm check` avant livraison. |
| Pull request | `pnpm check` reparti entre jobs web et Rust. |
| Nuit / manuel | Whisper reel, corpus audio, E2E desktop et tests longs sans materiel. |
| Tag de version | Construction native, installation, lancement, mise a niveau, desinstallation, signature et sommes de controle. |
| Qualification Android | Emulateur puis appareil arm64-v8a physique, avec rapport distinct. |

## Backlog d'implementation

L'ordre d'execution detaille, les cases et le journal de preuves sont maintenus
dans [`TEST_IMPLEMENTATION_PLAN.md`](TEST_IMPLEMENTATION_PLAN.md). Ce document
de strategie definit la couverture attendue; le plan d'implementation definit
ce qui doit etre fait ensuite, et dans quel ordre.

### P0 - invariants et perte de donnees

1. Implementer puis tester la reprise applicative d'une capture restee au
   statut `transcribing` apres redemarrage.
2. Ajouter un test dynamique avec proxy bloque pour completer les gardes
   statiques de confidentialite et d'absence de trafic deja en place.

### P1 - parcours et plateformes

1. Installer un harnais E2E Tauri desktop et couvrir import, correction,
   nettoyage, intervenants et export.
2. Etendre Axe aux etats UI manquants et executer l'audit visuel de contraste
   en clair, sombre et mobile.
3. Tester la mise a niveau N-1 vers N des paquets Linux et Windows.
4. Executer import `content://` et transcription sur emulateur Android hors
   ligne, puis sur appareil physique.

### P2 - qualite audio et performance

1. Constituer un corpus public avec manifeste, licence, empreinte et attendu.
2. Automatiser WER/CER et DER avec seuils de non-regression.
3. Ajouter les formats audio courts manquants et les cas corrompus.
4. Mesurer session d'une heure, latence p50/p95, memoire, CPU et espace disque.
5. Executer le protocole batterie/temperature/cycle de vie Android.

## Criteres de sortie d'une version

- Toutes les suites P0 et les tests lies aux changements passent.
- Aucun test obligatoire n'est ignore, neutralise ou relance silencieusement.
- Les migrations N-1 vers N sont testees sur une copie synthetique.
- Les artefacts sont testes sur leur plateforme cible et leurs sommes sont
  publiees.
- Toute validation manuelle indique date, plateforme, version, protocole et
  resultat, sans donnee sensible.
- Les limites restantes sont listees dans `docs/ROADMAP.md`; elles ne sont pas
  presentees comme validees.
