# Vision produit

## Probleme

La retranscription manuelle d'un entretien est lente. Les solutions existantes envoient souvent l'audio vers un service distant, melangent les locuteurs ou produisent un texte difficile a relire.

## Utilisateurs

- recruteurs et candidats souhaitant analyser un entretien;
- journalistes et chercheurs conduisant des interviews;
- chefs de projet enregistrant des echanges;
- toute personne ayant besoin d'une trace lisible et locale d'une discussion.

## Parcours principal

1. Creer un entretien.
2. Choisir le microphone ou importer un fichier.
3. Indiquer eventuellement le nombre attendu de personnes.
4. Lancer la transcription.
5. Verifier et renommer les intervenants.
6. Basculer entre texte brut et texte nettoye.
7. Afficher ou masquer les horodatages.
8. Corriger puis exporter.

## Exigences fonctionnelles

### Capture et import

- Selection du microphone et indicateur de niveau sonore.
- Pause, reprise et arret sans perte des donnees.
- Import WAV, MP3, M4A, FLAC, OGG et formats video courants via extraction audio.
- Sauvegarde incrementale pendant l'enregistrement.

### Transcription

- Detection de langue, avec choix manuel possible.
- Mode temps reel avec texte provisoire puis consolidation.
- Mode a posteriori plus precis.
- Horodatages par segment, conserves meme lorsqu'ils sont masques.

### Intervenants

- Estimation automatique du nombre de personnes.
- Attribution `Intervenant 1`, `Intervenant 2`, etc.
- Fusion, separation, renommage et correction manuelle.
- Aucune identification nominale automatique sans action explicite.

### Nettoyage

- Conserver une version brute immuable.
- Proposer une version nettoyee qui retire les hesitations lexicales, repetitions immediates et pauses sans contenu.
- Afficher les differences et permettre d'annuler chaque nettoyage.
- Ne pas corriger une formulation si cela peut changer l'intention.

### Export

- TXT et Markdown pour la lecture.
- JSON pour conserver la structure complete.
- SRT et VTT pour les sous-titres.
- Choix d'inclure les horodatages et les noms des intervenants.

## Exigences non fonctionnelles

- Fonctionnement local par defaut.
- Reprise apres interruption ou plantage.
- Accessibilite WCAG AA.
- Interface utilisable sur petit ecran.
- Tests sur audio multi-locuteurs, accents, bruit et chevauchements.
- Suppression complete et verifiable d'un entretien.

## Hors perimetre initial

- Reconnaissance biometrique automatique d'une personne reelle.
- Resume ou evaluation automatique d'un candidat.
- Synchronisation cloud.
- Traduction automatique complete.
- Enregistrement discret ou sans consentement.

