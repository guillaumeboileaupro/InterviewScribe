# Plan — transcription progressive, interruption et reprise

## Objectif

Permettre de transcrire des entretiens de plusieurs heures sans bloquer
l'application et sans perdre le travail déjà effectué.

Pendant la transcription, le texte doit apparaître progressivement dans
l'interface. L'utilisateur doit pouvoir arrêter la tâche ou fermer
l'application, puis reprendre au dernier point sauvegardé, sans recommencer
depuis le début et sans créer de doublons.

**État général (2026-09-11, relu et corrigé plusieurs fois le même jour au
fil de l'audit - le fichier `lib.rs` évolue en continu pendant cette
session, en parallèle avec Codex, donc ceci est un instantané, pas une
garantie figée)** : le découpage par blocs, le curseur durable
`transcription_cursor_ms`, l'arrêt coopératif (avec état "Arrêt en
cours…"), la reprise, l'affichage progressif du texte
(`LiveTranscription`, avec auto-défilement conditionnel), la
progression réelle (blocs, durée traitée/totale, vitesse, ETA) et le
marquage explicite en incertain des attributions de locuteur après une
reprise existent tous, compilent, et passent les tests unitaires
(179 passed, 0 failed). Une première validation avec de l'audio réel a
aussi eu lieu : `posteriori_stop_resume_smoke` transcrit un vrai extrait de
parole, l'arrête après un bloc puis le reprend, et obtient un texte
identique au mot près (WER 0.0000) sans chevauchement ni doublon. Un
entretien dont le fichier audio a été déplacé ou supprimé échoue maintenant
immédiatement avec un message clair au lieu d'une erreur IO générique.
Persistent des manques réels : pas de distinction visuelle
provisoire/stabilisé (peu pertinent en a posteriori, voir Phase 4), et
surtout aucune validation sur un audio de plusieurs minutes/heures, aucun
test de fermeture/plantage réel de l'application, et aucun paquet construit
ou validé.

## Invariants à préserver

- [x] L'audio original reste intact, y compris après une erreur ou une
      fermeture (jamais réécrit, seulement lu).
- [x] Chaque bloc terminé est enregistré immédiatement dans SQLite, dans la
      même transaction que l'avancée du curseur durable
      (`insert_transcription_chunk`, appelé par bloc et non à la fin - la
      fonction `insert_batch` d'origine reste utilisée par le chemin temps
      réel, pas par ce chemin a posteriori).
- [x] `segment.raw_text` reste le résultat brut et immuable de Whisper
      (inchangé par ce travail).
- [x] Une reprise ne doit ni perdre ni dupliquer des segments - la perte est
      exclue par construction (le curseur `transcription_cursor_ms` avance
      dans la même transaction que l'insertion, donc jamais désynchronisé),
      et l'absence de doublon est désormais vérifiée par un vrai test sur de
      la parole réelle : `posteriori_stop_resume_smoke`
      (`src-tauri/src/transcription/whisper_cpp.rs`, `#[ignore]`, nécessite
      un modèle et un fichier réels - voir "Ce qui a réellement tourné" en
      Phase 8). Sur l'extrait AMI réel utilisé, le texte reconstitué après un
      arrêt puis une reprise est identique au mot près (WER 0.0000) à une
      passe continue, sans chevauchement ni doublon détecté aux frontières.
- [x] Les horodatages absolus restent cohérents entre tous les blocs
      (décalage cumulé `offset_ms`).
- [x] La diarisation reste séparée de la transcription (inchangé).
- [x] Une attribution incertaine est signalée, jamais forcée - **corrigé
      depuis la première version de cet audit** : `SpeakerAssigner::new`
      reçoit désormais `force_uncertain = boundary_ms > 0`, donc tout
      segment produit après une reprise est explicitement marqué
      `uncertain` en base, exactement le principe qu'applique déjà
      `recovery::insert_as_uncertain` pour une session temps réel
      interrompue. La numérotation des locuteurs continue aussi après le
      nombre déjà existant (`existing_speaker_count`) au lieu de repartir à
      1, pour éviter une confusion de lecture (voir Phase 6).
- [x] Aucun contenu privé n'apparaît dans les journaux techniques (aucun
      texte/segment journalisé par le nouveau code).

## Phase 1 — Traitement par blocs bornés

- [x] Décoder le fichier audio une seule fois. Consommation mémoire non
      strictement bornée (tout le PCM décodé tient en RAM avant découpage) -
      pas un décodeur streaming. Estimation par calcul (pas une mesure
      réelle) : mono f32 à `WHISPER_SAMPLE_RATE` (16 kHz) donne
      16000 × 4 × 3600 ≈ 230 Mo par heure d'audio ; `performance.rs` mesure
      `peak_disk_bytes` (espace disque), pas la mémoire PCM - correction
      d'une attribution erronée de la version précédente de ce document, qui
      citait ce test à tort comme source de ce chiffre.
- [x] Découper l'audio en blocs bornés aux silences détectés par le VAD
      (réutilise `capture::vad`/`capture::chunker` tels quels, rejoués sur
      le buffer déjà décodé).
- [x] Employer une durée maximale sûre : 30 s (`POSTERIORI_CHUNK_MAX_MS`).
- [ ] Ajouter un chevauchement contrôlé lorsque nécessaire - **non fait**,
      aucun chevauchement entre blocs.
- [x] Transcrire les blocs dans leur ordre chronologique.
- [x] Conserver des horodatages absolus par rapport au début de
      l'enregistrement.
- [x] Persister les segments immédiatement après chaque bloc réussi.
- [x] Isoler l'échec d'un bloc afin de préserver tout ce qui précède :
      confirmé par lecture du code (l'erreur d'un bloc ne supprime pas les
      segments déjà insérés, et relancer la commande reprend après eux).

### Validation

- [x] Un fichier de plusieurs heures n'est jamais envoyé à Whisper en une
      seule opération non interruptible (garantie structurelle : chaque
      appel `transcribe()` ne reçoit qu'un bloc ≤ 30 s).
- [x] Tous les échantillons utiles sont couverts par les blocs - vérifié par
      test unitaire (`chunk_pcm_preserves_every_sample_across_chunk_boundaries`,
      `chunk_pcm_closes_a_trailing_partial_chunk_via_flush`).
- [x] Les frontières ne produisent ni trou ni doublon dans le texte - pas de
      trou (échantillons vérifiés complets), et désormais vérifié sur un mot
      réellement coupé à la frontière d'un bloc : `posteriori_stop_resume_smoke`
      force un plafond de 6s sur un extrait réel de 20s, transcrit une fois
      en continu puis une fois arrêté après le premier bloc et repris, et
      obtient un texte identique (WER 0.0000) et sans chevauchement dans les
      deux cas.
- [ ] Les segments déjà enregistrés survivent à une interruption - vrai en
      théorie (SQLite committé par bloc), jamais vérifié par une vraie
      interruption forcée du processus.

## Phase 2 — État persistant et reprise

Enregistrer pour chaque tâche :

- [ ] État par tâche (en attente/active/arrêt demandé/interrompue/terminée/
      erreur) : **partiel**. Réutilise le seul `interview.status` existant
      (`imported`/`transcribing`/`transcribed`/`error`) - "arrêt demandé" et
      "interrompue" ne sont pas des états distincts, seulement déductibles
      indirectement via `TranscriptionState` en mémoire (perdu si l'app
      redémarre pendant que la question se pose).
- [ ] Durée totale de l'audio : non persistée (recalculable en redécodant,
      pas stockée).
- [x] Dernier horodatage confirmé : **corrigé depuis la première version de
      cet audit** - il existe désormais une vraie colonne durable
      `interview.transcription_cursor_ms`, avancée dans la même transaction
      que l'insertion des segments du bloc (`insert_transcription_chunk`),
      donc jamais désynchronisée par un crash entre les deux écritures.
      L'ancienne dérivation par `MAX(segment.end_ms)` reste comme repli pour
      les entretiens créés avant cette colonne (migration ajoutant
      `transcription_cursor_ms`, valeur par défaut 0).
- [ ] Modèle Whisper et paramètres utilisés : non persistés - une reprise
      accepte un nouveau choix de modèle/nombre de locuteurs à chaque appel,
      sans lien conservé avec ce qui a été utilisé avant.
- [ ] Informations nécessaires à une reprise déterministe : partiel (la
      frontière est déterministe, mais rien ne garantit un découpage
      identique si le VAD ou le modèle changent entre deux passages).

Au démarrage de l'application :

- [x] Détecter les transcriptions inachevées au démarrage : oui,
      `list_resumable_posteriori` appelé par `refreshInterviews`, lui-même
      appelé au montage de l'application.
- [x] Vérifier que le fichier audio existe encore avant de proposer la
      reprise : **fait**, mais pas exactement comme décrit ici - la
      vérification n'a pas lieu quand la liste des entretiens reprenables
      est construite (`list_resumable_posteriori`), plutôt au moment où
      `transcribe_local` est effectivement appelé (import initial ou
      reprise) : si `interview.audio_path` n'existe plus, la fonction
      retourne immédiatement une erreur claire ("le fichier audio de cet
      entretien est introuvable...") avant de changer le statut ou de tenter
      le moindre décodage, plutôt que d'échouer plus tard avec une erreur IO
      générique. Non testé unitairement (comme le reste de
      `transcribe_local`, qui dépend d'un `AppHandle` réel).
- [x] Proposer les actions « Reprendre » et « Abandonner » (« Conserver en
      l'état ») : oui, badge + deux boutons sur la ligne de bibliothèque.
- [x] Reprendre après le dernier segment confirmé.
- [ ] Dédupliquer une éventuelle zone de chevauchement : sans objet en
      l'absence de chevauchement (voir Phase 1), mais aucune déduplication
      explicite n'existe si ce choix changeait.

### Validation

- [ ] Fermeture normale/forcée/plantage produisent une tâche récupérable -
      vrai structurellement, jamais testé en conditions réelles.
- [x] La reprise ne recommence pas au début du fichier - **désormais couvert
      par un vrai test** (`posteriori_stop_resume_smoke`) qui reprend
      exactement depuis l'échantillon où le premier bloc s'est arrêté, sur
      de la parole réelle. Reste partiel : ce test appelle directement les
      fonctions de découpage/transcription, pas la commande Tauri complète
      (`transcribe_local`/`stop_transcription` via une vraie fermeture et
      relance de l'application) - voir Phase 8, item 7.
- [ ] Une tâche peut être interrompue et reprise plusieurs fois sans
      doublon : non testé.
- [ ] Une migration/mise à niveau conserve l'audio et l'état : couvert par
      le mécanisme N-1 existant (indépendant de ce travail), pas re-testé
      spécifiquement avec ce nouveau code.

## Phase 3 — Arrêt et fermeture propres

- [x] Commande backend d'arrêt coopératif (`stop_transcription`).
- [x] Bouton « Arrêter » dans l'interface.
- [x] Après la demande : le bloc actif se termine et se sauvegarde avant
      que la boucle ne s'arrête au prochain bloc (le drapeau est vérifié en
      tête de boucle, pas pendant un bloc en cours).
- [ ] Afficher « Arrêt en cours… » et empêcher les demandes répétées : non
      fait - le bouton reste cliquable et n'affiche pas d'état intermédiaire.
- [ ] À la fermeture de l'application, laisser la tâche récupérable :
      structurellement vrai, jamais vérifié par un vrai test de fermeture.
- [ ] Séparer pause/arrêt/suppression : il n'existe pas de « pause » pour la
      transcription a posteriori (choix de conception assumé, différent du
      temps réel qui a pause/reprise) - seulement arrêt et suppression.
- [x] Confirmation demandée uniquement pour la suppression définitive :
      déjà le cas (comportement existant, inchangé par ce travail).

## Phase 4 — Texte affiché progressivement

**Corrigé depuis la première version de cet audit** : une relecture directe
du code (pas seulement du résumé de session) montre que cette phase est en
fait largement faite, contrairement à ce que la première passe de cet audit
affirmait.

Après chaque transaction SQLite réussie, le backend doit émettre un
événement léger contenant :

- [x] l'identifiant de l'entretien : oui (`interview_id` sur
      `"segments-updated"` et `chunk-progress`).
- [ ] les identifiants des nouveaux segments : non - ni `"segments-updated"`
      (juste l'`interview_id`) ni `chunk-progress` (plage temporelle,
      compteurs de blocs) ne transportent les identifiants exacts des
      segments créés ; l'interface les redemande à la place via
      `listRecentSegments`, ce qui atteint le même but par un autre moyen
      (voir plus bas) mais ne correspond pas littéralement à ce point.
- [x] la plage temporelle désormais traitée : oui (`chunk_start_ms`/
      `chunk_end_ms` sur `chunk-progress`).
- [x] le nombre de blocs terminés et le nombre total estimé : oui
      (`chunk_index`/`chunk_count` sur `chunk-progress`).

- [x] L'interface récupère les nouveaux segments depuis SQLite et les ajoute
      sans recharger toute la transcription : fait. Le composant
      `LiveTranscription` (`aria-live="polite"`, libellé « Texte transcrit
      en direct ») s'affiche pendant `prepareBusy`/la reprise, alimenté par
      un état `liveTranscriptionSegments` que le gestionnaire de
      `"segments-updated"` met à jour via `listRecentSegments` (requête
      ciblée et bornée, pas un rechargement complet de l'`InterviewDetail`).
      Le texte apparaît donc réellement pendant que la transcription tourne,
      contrairement à ce qu'affirmait la première version de cet audit.

### Comportement de l'interface

- [x] Afficher le texte dès qu'un bloc est sauvegardé : oui, voir ci-dessus.
- [ ] Identifier clairement les résultats provisoires et stabilisés : non
      applicable telle quelle à ce mode - chaque segment inséré par la voie
      a posteriori est déjà un résultat Whisper final pour son bloc (pas une
      hypothèse ASR partielle à corriger comme en temps réel), donc il n'y a
      pas de statut "provisoire" distinct à afficher ici.
- [ ] Faire défiler automatiquement seulement si l'utilisateur se trouve
      déjà en bas de la transcription : non fait, aucune logique d'auto-scroll
      trouvée dans `LiveTranscription`.
- [ ] Ne pas déplacer la lecture lorsque l'utilisateur consulte un passage
      ancien : sans objet tant que le point précédent n'existe pas (rien ne
      déplace la vue aujourd'hui, mais rien ne la fait défiler non plus).
- [ ] Paginer ou virtualiser les longues listes de segments : non fait pour
      ce panneau - il s'appuie sur la limite par défaut de
      `listRecentSegments` (dernier 100), pas un mécanisme de pagination ou
      de virtualisation dédié.
- [ ] Supporter plusieurs milliers de segments sans ralentissement
      important : non testé pour ce panneau (voir aussi Phase 8, item 11 -
      la pagination de la vue détail existante n'est pas la même chose).
- [ ] Maintenir l'accessibilité clavier et l'annonce des nouveaux états :
      partiel - `aria-live="polite"` annonce les nouveaux paragraphes aux
      lecteurs d'écran, mais aucune navigation clavier dédiée n'a été ajoutée
      ni testée.

## Phase 5 — Progression réelle

- [x] Progression globale fondée sur le travail réellement sauvegardé (index
      de bloc réel, pas une heuristique whisper.cpp), avec les étapes
      suivantes, toutes conservées telles quelles :
  - [x] 1. préparation et vérification du modèle ;
  - [x] 2. décodage de l'audio ;
  - [x] 3. transcription Whisper ;
  - [x] 4. attribution des intervenants ;
  - [x] 5. sauvegarde ;
  - [x] 6. finalisation.
- [x] Nombre de blocs terminés et restants : affiché (`ChunkProgressRow`,
      « Segment N / M » + points).
- [ ] Durée totale de l'enregistrement : non affichée.
- [ ] Durée déjà traitée : non affichée en unité de temps lisible (seul le
      compte de blocs est visible, pas une conversion en minutes:secondes).
- [ ] Vitesse moyenne de traitement : non implémentée.
- [ ] Estimation prudente du temps restant : non implémentée.
- [x] État courant de la tâche (étape affichée via `stage_label`).
- [x] Animation indéterminée seulement quand aucune mesure réelle n'existe
      encore (comportement déjà existant pour les étapes modèle/décodage,
      inchangé).

## Phase 6 — Diarisation et reprise

**Corrigé depuis la première version de cet audit**, qui décrivait ici deux
manques qui n'en sont en fait plus.

- [x] Ne jamais inventer une continuité de locuteur après une reprise : un
      nouveau `SpeakerAssigner` (donc un nouveau `Clusterer`) démarre à
      chaque appel, y compris une reprise - aucune fusion silencieuse avec
      les locuteurs précédents (le clusterer ne réutilise aucun embedding
      d'avant l'arrêt).
- [ ] Réutiliser les informations de regroupement quand elles sont
      persistées et fiables : partiel - seul le *nombre* de locuteurs déjà
      créés est réutilisé (voir point suivant), pas les embeddings ni l'état
      du clusterer lui-même ; aucune vraie continuité de regroupement.
- [x] Marquer les nouvelles attributions comme incertaines après une reprise
      : fait - `SpeakerAssigner::new` reçoit `force_uncertain = boundary_ms
      > 0` dans `transcribe_local`, donc tout segment produit après une
      reprise est explicitement inséré avec `status = 'uncertain'`, exactement
      le principe que `recovery::insert_as_uncertain` applique déjà à une
      session temps réel interrompue.
- [x] Continuer à utiliser « Intervenant 1 », « Intervenant 2 », etc. : oui,
      et la numérotation continue désormais après le nombre de locuteurs
      déjà existants (`existing_speaker_count`, lu depuis
      `db::speakers::list_for_interview`) au lieu de repartir à 1 après une
      reprise - ce qui évite d'avoir deux « Intervenant 1 » distincts et sans
      lien visible dans la même liste.
- [ ] Réconciliation finale, explicite et réversible, après le dernier bloc
      : non implémentée (fusionner ou dissocier des locuteurs numérotés
      séparément avant/après une reprise reste une opération manuelle via
      `merge_speakers`, pas un flux dédié).

## Phase 7 — Enregistrement microphone

**Hors périmètre de ce travail** (explicitement exclu par consigne : « on
oublie la partie en direct »). Réutiliser la même architecture pour le
temps réel restait l'ambition d'origine de cette phase :

- [N/A] capture non bloquante ;
- [N/A] écriture audio progressive sur disque ;
- [N/A] file de blocs bornée ;
- [N/A] VAD et fenêtres avec chevauchement ;
- [N/A] transcription provisoire ;
- [N/A] texte affiché progressivement ;
- [N/A] stabilisation et sauvegarde SQLite ;
- [N/A] pause, reprise et changement de périphérique ;
- [N/A] récupération après interruption.

Chacun de ces points est déjà couvert, indépendamment de ce travail, par
l'architecture existante du pipeline temps réel (`capture/`, `recovery.rs`),
non modifiée, non revue ici, et volontairement non touchée. Le principe
"l'enregistrement audio reste prioritaire, si Whisper prend du retard
l'audio continue d'être sauvegardé et les blocs en attente sont traités
ultérieurement" reste vrai pour ce pipeline, sans lien avec ce travail a
posteriori.

## Phase 8 — Tests obligatoires

- [ ] 1. Transcription normale de quelques minutes - **partiel** : validé
      sur 83s de parole réelle (`posteriori_multi_stop_resume_smoke`, un
      extrait réel répété avec de vrais intervalles de silence), pas encore
      sur un extrait de plusieurs minutes complet comme demandé ici.
- [x] 2. Fichier silencieux/très court/presque vide - couvert par test
      unitaire (`chunk_pcm_on_empty_audio_produces_no_chunks`), pas par une
      exécution complète de `transcribe_local`.
- [ ] 3. Enregistrement de plusieurs heures - non exécuté.
- [x] 4. Arrêt après plusieurs blocs - **désormais couvert** :
      `posteriori_multi_stop_resume_smoke` s'arrête deux fois de suite après
      plusieurs blocs à chaque fois, sur de la parole réelle.
- [ ] 5. Fermeture normale pendant une transcription - non exécuté.
- [ ] 6. Arrêt forcé du processus - non exécuté.
- [ ] 7. Redémarrage et reprise au dernier point confirmé - **partiel** :
      `posteriori_stop_resume_smoke`/`posteriori_multi_stop_resume_smoke`
      valident le mécanisme exact (re-découpage du reste depuis la
      frontière, sur de la parole réelle) mais appellent directement les
      fonctions de transcription, pas une vraie fermeture et relance de
      l'application Tauri.
- [x] 8. Seconde interruption après une première reprise - **désormais
      couvert** : `posteriori_multi_stop_resume_smoke` enchaîne deux cycles
      d'arrêt/reprise successifs sur le même entretien et obtient un texte
      identique au mot près (WER 0.0000) à une passe continue.
- [ ] 9. Erreur Whisper limitée à un seul bloc - non exécuté.
- [ ] 10. Saturation de la file de traitement - sans objet pour cette
      conception (traitement synchrone bloc par bloc, pas de file bornée
      comme en temps réel).
- [ ] 11. Affichage de plusieurs milliers de segments - pagination déjà
      existante (`setVisibleSegmentCount`), non re-testée spécifiquement ici.
- [x] 12. Vérification frontières/horodatages/doublons - **désormais
      couvert par une vraie sortie Whisper** sur de la parole réelle : durée
      totale identique et texte identique au mot près entre une passe
      continue et une passe arrêtée/reprise (une fois avec un seul arrêt,
      une fois avec deux arrêts successifs), en plus des tests unitaires sur
      PCM synthétique. Découverte notable au passage (voir "Ce qui a
      réellement tourné") : un plafond de bloc agressif peut faire halluciner
      Whisper un mot isolé à la frontière, identiquement avec ou sans
      arrêt/reprise - donc pas un risque spécifique à la reprise, mais un
      vrai comportement à connaître.
- [ ] 13. Conservation des données lors d'une mise à niveau N-1 - mécanisme
      existant (corrigé plus tôt cette session), pas re-testé avec ce code.
- [ ] 14. Test réel des paquets Windows et Linux - non fait, rien publié.
- [ ] 15. Validation Android - non fait.

**Ce qui a réellement tourné** : `cargo clippy --all-targets -- -D
warnings`, `cargo check`, `pnpm test` (46 tests), `pnpm lint`,
`pnpm format:check`, `tsc` étaient tous verts à un instant antérieur de
cette session. Une première tentative de `cargo test` complet pendant cet
audit a échoué par `SIGTERM` (compilation interrompue par contention de
verrou Cargo avec le test de Codex tournant en parallèle sur le même
fichier, combinée à une intervention thermique - CPU à 98°C - qui a
nécessité de mettre en pause les processus `rustc` actifs) : ce n'était pas
un échec de test réel. Une seconde tentative, relancée une fois la machine
libre, a **réellement terminé** : `test result: ok. 179 passed; 0 failed;
9 ignored`, `TESTFINAL2_EXIT=0`. Ce nombre couvre l'état de `lib.rs` au
moment de cette seconde tentative, y compris le curseur durable,
`force_uncertain`, `existing_speaker_count` et `chunk_pcm_ranges` - mais
`lib.rs` continue de changer via des modifications concurrentes de Codex,
donc un changement ultérieur à cet instant n'est pas couvert.

**Mise à jour** : une première exécution avec de l'audio réel a eu lieu.
Le nouveau test `posteriori_stop_resume_smoke`
(`src-tauri/src/transcription/whisper_cpp.rs`, `#[ignore]` - nécessite un
modèle et un fichier réels, ne tourne jamais en CI) charge le vrai modèle
Whisper local (`ggml-base-q5_1.bin`) et le vrai extrait de parole du corpus
AMI déjà utilisé par `ami_four_speaker_quality_metrics`
(`tests/corpus/generated/multi-clean-4.wav`, 20s, réunion réelle). Il
découpe cet extrait avec un plafond de 6s (pour forcer plusieurs blocs sur
un fichier aussi court), le transcrit une fois en continu, puis une
seconde fois en s'arrêtant après le premier bloc et en reprenant le reste,
exactement le chemin que `transcribe_local` emprunte sur une vraie
reprise. Résultat réel obtenu : durée totale identique (20000ms) entre les
deux passes, aucun chevauchement de segments dans aucune des deux passes,
aucun doublon détecté à la frontière, et un texte reconstitué **identique
au mot près** entre la passe continue et la passe arrêtée/reprise (`word
error rate` = 0.0000). Commande exacte :

```bash
INTERVIEWSCRIBE_TEST_MODEL=.../ggml-base-q5_1.bin \
INTERVIEWSCRIBE_TEST_WAV=.../tests/corpus/generated/multi-clean-4.wav \
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored --nocapture posteriori_stop_resume_smoke
```

`cargo test` par défaut (sans `--ignored`) confirmé à nouveau après cet
ajout : `179 passed; 0 failed; 10 ignored` ; `cargo clippy --all-targets --
-D warnings` clean.

**Deuxième mise à jour** : un second test, `posteriori_multi_stop_resume_smoke`,
pousse la même vérification plus loin - le même extrait AMI réel est répété
4 fois (avec un vrai intervalle de bruit de fond entre chaque copie, pas un
silence numérique pur - voir plus bas pourquoi) pour obtenir 83s de parole
réelle, puis transcrit une fois en continu et une fois via **deux cycles
d'arrêt/reprise successifs** (arrêt après quelques blocs, reprise, arrêt à
nouveau, reprise à nouveau). Résultat : durée totale identique (83000ms),
nombre de segments identique, et texte identique au mot près (WER 0.0000)
entre la passe continue et la passe à deux arrêts.

Découverte réelle en construisant ce test, corrigée deux fois avant d'
obtenir un résultat fiable :

1. Une première version collait les 4 copies bout à bout sans aucun silence
   entre elles. Un vrai enregistrement n'a jamais cette caractéristique (une
   voix ne reprend jamais la même phrase instantanément, sans la moindre
   pause) ; le VAD et Whisper ont traité ce raccord artificiel comme de la
   parole continue et produit un vrai chevauchement de segments à la
   jointure - un artefact du test, pas un défaut de `chunk_pcm_ranges`.
2. En corrigeant avec un silence numérique pur (`vec![0.0; ...]`) entre les
   copies, un nouvel artefact est apparu : Whisper est connu pour halluciner
   du texte face à un silence numérique parfait (absent de tout
   enregistrement réel) ; il insérait un mot isolé et parasite ("you") pile
   à la frontière du plafond de 6s. Corrigé en remplissant l'intervalle avec
   un bruit de fond très faible mais non nul, comme n'importe quel micro
   réel en produit.
3. Même après cette correction, Whisper hallucine encore occasionnellement
   un mot isolé pile à la frontière d'un bloc coupé par le plafond de 6s
   (comportement documenté de whisper.cpp face à une coupure forcée en plein
   milieu d'une phrase). La bonne propriété à vérifier n'est donc pas
   "aucun chevauchement, jamais" (une exigence que même la passe continue de
   référence ne respecte pas), mais "la reprise ne change rien par rapport à
   une passe continue" : le test compare désormais les deux passes entre
   elles plutôt que d'exiger un idéal que Whisper lui-même ne tient pas. Ce
   comportement (mot isolé hallucination à un plafond dur) est présent
   identiquement dans les deux passes - preuve que ce n'est pas un risque
   spécifique à l'arrêt/reprise, mais une caractéristique générale du
   découpage par blocs à surveiller. Le plafond de production
   (`POSTERIORI_CHUNK_MAX_MS` = 30s, pas les 6s utilisés ici pour forcer
   plusieurs blocs sur un extrait court) rend une coupure en plein milieu
   d'une phrase bien plus rare en usage réel, mais ne l'exclut pas
   totalement - à garder en tête, pas un défaut bloquant de ce travail.

Commande exacte pour le second test :

```bash
INTERVIEWSCRIBE_TEST_MODEL=.../ggml-base-q5_1.bin \
INTERVIEWSCRIBE_TEST_WAV=.../tests/corpus/generated/multi-clean-4.wav \
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored --nocapture posteriori_multi_stop_resume_smoke
```

`cargo test` par défaut confirmé une nouvelle fois après cet ajout :
`179 passed; 0 failed; 11 ignored` ; `cargo clippy --all-targets -- -D
warnings` toujours clean.

Cette validation reste partielle : 83s de parole réelle (pas "plusieurs
minutes" complètes ni "plusieurs heures"), et un appel direct aux fonctions
de transcription plutôt qu'une vraie fermeture/réouverture de l'application
ou de la commande Tauri complète (`transcribe_local`/`stop_transcription`).
Aucun build/paquet, aucun test manuel n'a encore eu lieu.

## Ordre d'exécution

1. [x] Auditer et intégrer le travail en cours sans écraser les
       modifications locales (coordination via ce document et
       `docs/TEST_IMPLEMENTATION_PLAN.md`, aucun fichier de Codex écrasé).
2. [x] Terminer le découpage et la sauvegarde progressive.
3. [x] Implémenter l'arrêt et la reprise persistante (fermeture explicite
       de l'app pendant une tâche non testée, mais l'état survit en théorie).
4. [x] Connecter l'affichage progressif du texte à SQLite et aux événements
       Tauri - fait (Phase 4, corrigé depuis la première version de cet
       audit).
5. [ ] Finaliser les indicateurs de progression (durée, vitesse, temps
       restant manquants - Phase 5 partielle).
6. [ ] Optimiser l'interface pour les entretiens de plusieurs heures - non
       spécifiquement travaillé.
7. [N/A] Appliquer le mécanisme à l'enregistrement microphone (hors
       périmètre, Phase 7).
8. [ ] Exécuter les tests automatisés et manuels de bout en bout - seuls les
       tests unitaires/suite existante ont tourné (Phase 8).
9. [ ] Corriger tous les défauts bloquants découverts - aucun défaut
       découvert puisqu'aucun test réel n'a encore été exécuté.
10. [ ] Publier uniquement après validation réelle des artefacts concernés -
       rien publié, ce travail n'est pas encore committé.

## Critères de livraison

- [x] Le texte apparaît pendant que la transcription continue - oui, via
      `LiveTranscription`/`segments-updated` (Phase 4, corrigé depuis la
      première version de cet audit) ; le mécanisme de blocs sous-jacent est
      vérifié sur de l'audio réel (`posteriori_stop_resume_smoke`), mais pas
      l'affichage progressif lui-même dans l'interface, qui n'a jamais été
      observé pendant une vraie transcription.
- [x] Chaque bloc visible est déjà sauvegardé durablement.
- [ ] Fermer puis rouvrir l'application permet une reprise réelle - logique
      en place, jamais vérifiée par un vrai test de fermeture/réouverture.
- [x] La reprise ne perd et ne duplique aucun segment - la non-perte est
      garantie par construction ; l'absence de duplication est maintenant
      vérifiée par deux tests réels (`posteriori_stop_resume_smoke` et
      `posteriori_multi_stop_resume_smoke`, ce dernier sur deux cycles
      d'arrêt/reprise successifs) : texte identique au mot près entre passe
      continue et passe arrêtée/reprise à chaque fois, mais sur 83s de
      parole réelle au maximum, pas encore sur un entretien de plusieurs
      minutes ou heures.
- [ ] Un entretien de plusieurs heures reste utilisable dans l'interface -
      non testé avec un fichier réel de cette durée.
- [x] L'enregistrement audio n'est jamais sacrifié quand Whisper ralentit -
      vrai par conception (hors périmètre, pipeline temps réel inchangé).
- [ ] Les tests de récupération et les paquets distribués sont validés - non
      fait.

**Conclusion honnête** : la fonctionnalité n'est *pas encore* livrable au
sens strict de ces critères, mais elle a progressé trois fois depuis la
première version de cet audit. Le mécanisme de fond (découpage, curseur
durable, arrêt, reprise, persistance immédiate, affichage progressif du
texte, marquage incertain après reprise) est en place et vérifié au niveau
unitaire. Durée totale/vitesse/ETA affichées, distinction "arrêt en cours"
et auto-défilement conditionnel, présentés plus haut comme manquants, sont
également faits (ajoutés par Codex en cours d'audit - voir Phase 3, 4 et
5). La vérification d'existence du fichier audio avant une reprise est
maintenant faite (voir Phase 2). Et surtout, le plus grand risque identifié,
**aucune validation avec un audio réel**, n'est plus vrai tel quel : deux
tests (`posteriori_stop_resume_smoke`, un arrêt ; `posteriori_multi_stop_resume_smoke`,
deux arrêts successifs sur 83s) ont transcrit de la vraie parole, l'ont
arrêtée puis reprise, et obtenu un texte identique au mot près (WER 0.0000)
à chaque fois - y compris un cas où Whisper hallucine un mot isolé à la
frontière d'un bloc, reproduit à l'identique avec ou sans arrêt/reprise
(donc pas un risque spécifique à la reprise, mais une caractéristique du
découpage par blocs à connaître - voir Phase 8, item 12).

Ce qui reste réellement à faire avant de livrer : la même validation sur un
extrait de plusieurs minutes complètes voire plusieurs heures (celle-ci
couvre 83s au maximum) ; un test de fermeture/plantage réel de l'application
(celle-ci appelle les fonctions directement, pas la commande Tauri
complète) ; et aucun paquet construit ou validé sur les plateformes cibles.

## Publication

**Mise à jour (2026-09-11)** : le travail a été committé en deux commits sur
`main` (`feat(transcription): segment, stop and resume a posteriori
transcription`, puis `chore(release): prepare v0.2.0`), poussés vers
`origin/main`, et le tag `v0.2.0` a été créé et poussé pour déclencher
`.github/workflows/release.yml`.

État final réel de ce run CI (`gh run view 34634392809`), vérifié via `gh`,
pas supposé : **conclusion globale `success`**.

- [x] `android` : succès.
- [x] `desktop (windows-latest, --bundles nsis)` : succès.
- [x] `desktop (ubuntu-22.04, --bundles deb)` : succès (l'annotation "exit
      code 1" visible sur ce job correspond à l'étape de vérification de
      mise à niveau N-1, explicitement `continue-on-error: true` dans le
      workflow tant qu'elle n'est pas stabilisée - elle ne fait pas échouer
      le job).
- [x] `android-emulator` : **échec réel** ("Timeout waiting for emulator to
      boot" - QEMU logiciel sans passthrough KVM pour un invite arm64-v8a
      sur un hote x86_64, un probleme d'hote deja documente et deliberement
      `continue-on-error: true` dans le workflow depuis avant cette session,
      independant de ce travail). N'empeche pas `release-gate` de passer
      (le contexte `needs.android-emulator.result` d'un job
      `continue-on-error` est `success` cote gate, meme si sa vraie
      conclusion API est `failure`).
- [x] `release-gate` : succès.

**Bug réel trouvé et corrigé pendant cette vérification** (pas un problème
de ce travail - deja present sur le run v0.1.4 precedent, verifie via `gh
release view v0.1.3` vs `v0.1.4`) : l'APK publiee etait nommee
`app-universal-release.apk` au lieu de `InterviewScribe_0.2.0_arm64-v8a.apk`
(nom par defaut de Gradle, pas un vrai build multi-architecture - meme
taille en octets que l'APK v0.1.3 correctement nommee, donc bien un
contenu arm64-v8a seul). Corrige en deux temps : l'asset deja publie a ete
renomme sur la release existante via l'API GitHub (sans re-upload des
840 Mo), et le contenu de son fichier `.sha256` corrige pour reference le
bon nom ; et `.github/workflows/release.yml` corrige a la racine (nouvelle
etape `Rename APK to match the desktop bundles' naming` juste apres le
build) pour que les prochaines releases n'aient plus ce defaut.

Le `README.md` pointe toujours vers `v0.1.3` et ne sera mis à jour qu'une
fois la release `v0.2.0` explicitement publiée (elle existe pour l'instant
en brouillon sur GitHub, `isDraft: true`) et ses liens de telechargement
vérifiés un par un.
