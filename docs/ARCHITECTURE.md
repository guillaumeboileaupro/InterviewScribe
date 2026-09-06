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
| Export | TXT, Markdown, JSON, SRT et VTT |

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

1. Decodage et normalisation en PCM mono.
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

## Diarisation

Whisper ne distingue pas a lui seul les personnes. La diarisation doit exposer une interface stable et plusieurs implementations possibles. Le premier prototype peut exploiter des embeddings vocaux locaux. Les recouvrements de voix doivent etre signales comme incertains plutot que forces vers un seul locuteur.

## Securite et confidentialite

- Aucun enregistrement ou texte dans les journaux.
- Chiffrement offert pour les projets locaux.
- Permissions microphone demandees au moment utile.
- Aucun trafic reseau pendant une transcription locale, hors telechargement explicite d'un modele.
- Suppression coordonnee de la base, de l'audio, des caches et des exports geres.

