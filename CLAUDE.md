# Contexte Claude - InterviewScribe

InterviewScribe est un logiciel de transcription d'entretiens local-first pour Windows, Linux et Android.

Avant de travailler:

1. Lire `AGENTS.md` comme source principale des invariants.
2. Lire les documents pertinents dans `docs/`.
3. Charger le skill adapte dans `.claude/skills/`.

Le coeur du produit comporte deux modes:

- temps reel: capture du microphone, transcription incrementale et attribution provisoire des locuteurs;
- a posteriori: import d'un enregistrement, traitement complet, diarisation plus precise et export.

Toujours conserver separement la transcription brute et la vue nettoyee. La suppression des `euh`, repetitions, faux departs et pauses ne doit ni resumer ni reformuler le contenu. Toute identification automatique d'un locuteur reste une suggestion modifiable.

Le style attendu est minimaliste, professionnel et calme. L'interface doit mettre le contenu avant la decoration.

