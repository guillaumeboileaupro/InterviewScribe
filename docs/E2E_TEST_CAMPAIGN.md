# Campagne de test E2E - v0.1.3

## Objectif et mode d'emploi

Ce document sert à **vous** (pas à un script) pour tester l'application
installée pour de vrai, sur votre machine. Pour chaque test :

1. Suivez les étapes.
2. Cochez la case qui correspond à ce que vous observez réellement (QCM).
3. Si vous cochez une case autre que "Comme attendu", ouvrez **Réglages >
   Diagnostics**, cliquez "Copier les journaux", et collez le contenu
   quelque part pour me le transmettre avec le numéro du test (ex. `IMP-02`).

Ne rien deviner ni supposer : si un écran ne correspond pas à la
description, cochez "Autre" et décrivez ce que vous voyez.

**Prérequis** : téléchargez le paquet correspondant à votre plateforme depuis
la [page Releases](https://github.com/guillaumeboileaupro/InterviewScribe/releases/tag/v0.1.3)
et installez-le (voir le README pour le détail par plateforme). Notez ici
avant de commencer :

- Plateforme testée : Windows / Linux / Android (entourer)
- Version/build de l'OS :
- Date du test :

---

## 1. Premier lancement

### GEN-01 - Démarrage de l'application

**Étapes** : lancer l'application pour la première fois après installation.

- [ ] Comme attendu : l'application s'ouvre sur "Mes entretiens", vide, sans erreur.
- [ ] L'application ne démarre pas du tout (rien ne s'affiche).
- [ ] Un message d'erreur s'affiche au démarrage (lequel ? à noter).
- [ ] Autre (décrire) :

### GEN-02 - Écran Réglages

**Étapes** : cliquer sur "Réglages" dans la barre latérale.

- [ ] Comme attendu : la section "Modèles et stockage" affiche "Whisper Large v3 Turbo (Q5_0)" et "Inclus dans l'application".
- [ ] La section affiche "Vérification du modèle intégré…" indéfiniment (ne se termine jamais).
- [ ] Un message d'erreur apparaît à la place du modèle (lequel ?).
- [ ] Autre (décrire) :

### GEN-03 - Thème

**Étapes** : dans Réglages > Apparence, essayer "Clair" puis "Sombre" puis "Suivre le système".

- [ ] Comme attendu : l'interface change de couleurs immédiatement à chaque choix, tout reste lisible.
- [ ] Le thème ne change pas visuellement.
- [ ] Un texte ou bouton devient illisible dans un des thèmes (lequel ?).
- [ ] Autre (décrire) :

---

## 2. Import d'un fichier audio

### IMP-01 - Import et transcription

**Étapes** : "+ Nouvel entretien" > laisser "Importer un fichier audio" >
"Choisir un fichier audio" > sélectionner un fichier audio réel (voix
parlée, quelques minutes).

- [ ] Comme attendu : le bouton passe en "Transcription en cours…" avec une barre de progression qui avance visiblement, puis l'entretien s'ouvre avec le texte transcrit et des horodatages.
- [ ] Le bouton passe en "Transcription en cours…" mais aucune barre de progression n'apparaît ou reste bloquée à 0%.
- [ ] Rien ne se passe après avoir choisi le fichier (pas d'erreur, pas de changement).
- [ ] Un message d'erreur apparaît (lequel ?).
- [ ] La transcription se termine mais le texte est vide ou incohérent avec l'audio.
- [ ] Autre (décrire) :

### IMP-02 - Formats variés (à répéter par format disponible : MP3, M4A, WAV, vidéo...)

**Étapes** : répéter IMP-01 avec un format différent à chaque fois.

- [ ] Comme attendu pour tous les formats testés.
- [ ] Échoue pour un format précis (lequel ? message ?).
- [ ] Autre (décrire) :

### IMP-03 (Android uniquement) - Import depuis le sélecteur système

**Étapes** : sur Android, "+ Nouvel entretien" > "Importer un fichier audio" >
choisir un fichier via l'application Fichiers/Google Drive (pas un chemin
direct).

- [ ] Comme attendu : le fichier est importé et transcrit, le titre par défaut est lisible (pas une suite de caractères illisibles).
- [ ] Le titre par défaut est illisible/tronqué (capture d'écran utile).
- [ ] L'import échoue (message ?).
- [ ] Autre (décrire) :

---

## 3. Enregistrement en direct (Windows/Linux uniquement)

### REC-01 - Liste des microphones

**Étapes** : "+ Nouvel entretien" > "Enregistrer avec le microphone".

- [ ] Comme attendu : le menu "Microphone" liste au moins un périphérique réel de votre machine.
- [ ] Le menu reste vide ("Peripherique par defaut" uniquement) alors qu'un micro est branché.
- [ ] Autre (décrire) :

### REC-02 - Démarrer l'enregistrement

**Étapes** : cliquer "Demarrer l'enregistrement", parler quelques secondes.

- [ ] Comme attendu : l'indicateur "● Enregistrement" apparaît, le chrono avance, la barre de niveau audio bouge quand vous parlez.
- [ ] Rien ne se passe du tout (pas de changement d'écran, pas d'erreur visible).
- [ ] Un message d'erreur s'affiche (lequel ? copier le texte exact).
- [ ] L'indicateur "Enregistrement" apparaît mais la barre de niveau reste immobile même en parlant fort.
- [ ] Autre (décrire) :

### REC-03 - Transcription en direct

**Étapes** : continuer à parler ~30 secondes après REC-02.

- [ ] Comme attendu : du texte apparaît progressivement sous "Transcription en direct" sans avoir à arrêter l'enregistrement.
- [ ] Aucun texte n'apparaît après plus d'une minute de parole.
- [ ] Le texte apparaît mais ne correspond pas à ce qui a été dit.
- [ ] Autre (décrire) :

### REC-04 - Pause / Reprise

**Étapes** : cliquer "Pause", attendre 3 secondes, cliquer "Reprendre".

- [ ] Comme attendu : le bouton passe de "Pause" à "‖ Pause" affiché puis à "Reprendre" ; l'enregistrement continue normalement après.
- [ ] Le bouton ne réagit pas au clic.
- [ ] Une erreur apparaît en cliquant "Reprendre".
- [ ] Autre (décrire) :

### REC-05 - Changer de microphone en cours de session

**Étapes** : pendant une pause (REC-04), changer le microphone sélectionné, puis "Reprendre".

- [ ] Comme attendu : la reprise se fait sur le nouveau périphérique sans erreur, l'enregistrement continue.
- [ ] Une erreur apparaît lors du changement.
- [ ] Autre (décrire) :

### REC-06 - Arrêter et terminer

**Étapes** : cliquer "Arreter et terminer".

- [ ] Comme attendu : l'application bascule sur l'écran de l'entretien terminé, avec le texte complet de la session.
- [ ] Rien ne se passe.
- [ ] L'entretien s'ouvre mais est vide alors que vous avez parlé.
- [ ] Autre (décrire) :

### REC-07 - Interruption non gérée (fermer l'app pendant l'enregistrement)

**Étapes** : démarrer un enregistrement (REC-02), puis fermer l'application
directement (croix / kill), sans cliquer "Arreter".

- [ ] Comme attendu : au redémarrage de l'application, l'entretien existe dans la bibliothèque avec au moins l'audio capturé avant la fermeture.
- [ ] L'entretien a disparu / l'audio est illisible ou vide.
- [ ] Autre (décrire) :

---

## 4. Locuteurs / diarisation (Windows/Linux uniquement)

### SPK-01 - Détection automatique

**Étapes** : ouvrir un entretien transcrit à plusieurs voix (import ou enregistrement).

- [ ] Comme attendu : plusieurs locuteurs distincts apparaissent (ex. "Intervenant 1", "Intervenant 2").
- [ ] Tout le texte est attribué à un seul locuteur alors qu'il y a plusieurs voix.
- [ ] Autre (décrire) :

### SPK-02 - Renommer un locuteur

**Étapes** : dans la section "Locuteurs", choisir "Renommer" sur un locuteur, saisir un nouveau nom, valider.

- [ ] Comme attendu : le nouveau nom apparaît partout dans la transcription immédiatement.
- [ ] Le nom ne change pas ou une erreur apparaît.
- [ ] Autre (décrire) :

### SPK-03 - Fusionner deux locuteurs

**Étapes** : "Fusionner avec…", choisir un autre locuteur, "Fusionner".

- [ ] Comme attendu : tous les segments des deux locuteurs sont regroupés sous un seul.
- [ ] Une erreur apparaît ou rien ne se passe.
- [ ] Autre (décrire) :

### SPK-04 - Ajouter / réassigner manuellement

**Étapes** : "+ Ajouter un locuteur" puis réassigner un segment existant à ce nouveau locuteur (menu déroulant sur le segment).

- [ ] Comme attendu : le segment change bien d'attribution.
- [ ] Autre (décrire) :

---

## 5. Édition et nettoyage

### EDT-01 - Nettoyage automatique

**Étapes** : sur un segment avec des hésitations ("euh", répétitions), cliquer "Nettoyer".

- [ ] Comme attendu : les hésitations sont barrées visuellement, le texte reste consultable dans sa version brute originale.
- [ ] Le texte brut original devient inaccessible après nettoyage.
- [ ] Le sens du texte semble modifié (pas juste des hésitations retirées).
- [ ] Autre (décrire) :

### EDT-02 - Modification manuelle + annulation

**Étapes** : "Modifier" sur un segment, changer le texte, "Enregistrer". Puis chercher un moyen d'annuler cette modification.

- [ ] Comme attendu : le nouveau texte est sauvegardé, et il est possible de revenir à une version précédente.
- [ ] La modification ne se sauvegarde pas.
- [ ] Impossible de retrouver le texte brut d'origine après coup.
- [ ] Autre (décrire) :

### EDT-03 - Horodatages

**Étapes** : cocher/décocher "Horodatages" pendant la lecture d'un entretien.

- [ ] Comme attendu : les horodatages apparaissent/disparaissent sans perdre le texte.
- [ ] Autre (décrire) :

---

## 6. Export

### EXP-01 - Chaque format (répéter pour TXT, Markdown, JSON, SRT, VTT, DOCX, PDF, DOC)

**Étapes** : dans un entretien, "Exporter en …", choisir un emplacement, ouvrir le fichier produit avec un logiciel adapté.

- [ ] Comme attendu : le fichier s'ouvre, contient le texte de l'entretien correctement mis en forme pour ce format.
- [ ] Le fichier est vide, corrompu, ou ne s'ouvre pas.
- [ ] Le bouton "Exporter en DOC" est grisé/absent (LibreOffice non détecté) - normal si LibreOffice n'est pas installé, à noter quand même.
- [ ] Autre (décrire) :

---

## 7. Suppression d'un entretien

### DEL-01 - Confirmation

**Étapes** : dans "Mes entretiens", cliquer "Supprimer" sur un entretien.

- [ ] Comme attendu : une boîte de dialogue "Supprimer cet entretien ?" apparaît, avec le focus sur "Conserver l'entretien".
- [ ] La suppression se fait immédiatement sans confirmation.
- [ ] Autre (décrire) :

### DEL-02 - Annulation

**Étapes** : dans la boîte de dialogue (DEL-01), appuyer sur Échap, ou cliquer "Conserver l'entretien".

- [ ] Comme attendu : la boîte se ferme, l'entretien est toujours présent dans la liste.
- [ ] Autre (décrire) :

### DEL-03 - Suppression effective

**Étapes** : cliquer "Supprimer définitivement".

- [ ] Comme attendu : l'entretien disparaît de la liste ; en rouvrant l'explorateur de fichiers, sa copie audio privée n'existe plus.
- [ ] L'entretien disparaît de la liste mais le fichier audio reste sur le disque (à vérifier dans le dossier de données de l'application).
- [ ] Un message d'erreur apparaît.
- [ ] Autre (décrire) :

---

## 8. Lecture audio et commentaire (nouveau v0.1.3)

### AUD-01 - Lecture de l'audio

**Étapes** : ouvrir un entretien transcrit, cliquer play sur le lecteur audio en haut de la page.

- [ ] Comme attendu : l'audio se lit correctement, avec le son de l'enregistrement/import d'origine.
- [ ] Aucun lecteur audio n'apparaît sur la page.
- [ ] Le lecteur apparaît mais rien ne se joue (silence ou erreur).
- [ ] Autre (décrire) :

### AUD-02 - Corrélation transcription/audio

**Étapes** : cliquer sur l'horodatage d'un segment au milieu de la transcription.

- [ ] Comme attendu : la lecture audio saute à ce moment précis et démarre ; le segment en cours de lecture est visuellement surligné pendant qu'il joue.
- [ ] Cliquer sur l'horodatage ne fait rien.
- [ ] L'audio saute au bon endroit mais aucun segment n'est surligné.
- [ ] Autre (décrire) :

### NOTE-01 - Commentaire sur l'entretien

**Étapes** : dans un entretien, section "Commentaire (optionnel)", taper un texte puis "Enregistrer le commentaire". Fermer l'entretien (retour à la liste) et le rouvrir.

- [ ] Comme attendu : le commentaire est toujours présent après avoir rouvert l'entretien.
- [ ] Le commentaire ne se sauvegarde pas / disparaît après réouverture.
- [ ] Un message d'erreur apparaît en cliquant "Enregistrer".
- [ ] Autre (décrire) :

---

## 9. Diagnostics

### DIA-01 - Panneau Diagnostics

**Étapes** : Réglages > section Diagnostics.

- [ ] Comme attendu : des lignes horodatées apparaissent (au moins celles des tests précédents).
- [ ] Le panneau reste vide malgré les actions déjà effectuées.
- [ ] Autre (décrire) :

### DIA-02 - Copier les journaux

**Étapes** : cliquer "Copier les journaux", puis coller (Ctrl+V) dans un éditeur de texte.

- [ ] Comme attendu : le bouton affiche "Copié ✓" et le contenu collé correspond exactement à ce qui est affiché.
- [ ] Le collage ne donne rien.
- [ ] Autre (décrire) :

---

## 10. Android spécifique

Avant AND-01, activer les options développeur et relever uniquement les données
non sensibles suivantes (ne pas copier le numéro de série, l'IMEI ou le compte
Google) :

- modèle commercial du téléphone :
- version Android / niveau API :
- ABI indiquée par `adb shell getprop ro.product.cpu.abi` (attendu : `arm64-v8a`) :
- nom et SHA-256 de l'APK testée :
- batterie au début du test :

### AND-01 - Installation

**Étapes** : installer l'APK (source inconnue autorisée).

- [ ] Comme attendu : l'installation se termine sans avertissement autre que celui, normal, de "source inconnue".
- [ ] L'installation échoue (message ?).
- [ ] Autre (décrire) :

Conserver comme preuves une capture de l'écran d'accueil et la sortie de
`adb shell dumpsys package com.guillaumeboileau.interviewscribe | grep versionName`.

### AND-02 - Permission microphone

**Étapes** : essayer de démarrer un enregistrement si l'option existe sur cette version, sinon noter qu'elle est absente.

- [ ] La demande de permission microphone apparaît au bon moment.
- [ ] Aucune demande de permission n'apparaît.
- [ ] La fonction d'enregistrement n'existe pas sur Android dans cette version - normal, à confirmer.
- [ ] Autre (décrire) :

### AND-03 - Diarisation absente (attendu)

**Étapes** : importer un fichier à plusieurs voix sur Android.

- [ ] Comme attendu : un seul locuteur unique est créé pour tout le texte (limitation connue et documentée, pas un bug).
- [ ] Autre comportement observé (décrire) :

### AND-04 - Import `content://` et transcription hors connexion

**Préparation** : copier un court fichier audio de test non privé dans
Téléchargements. Activer le mode avion, puis vérifier que Wi-Fi et données
mobiles sont désactivés.

**Étapes** : dans InterviewScribe, choisir "+ Nouvel entretien" > "Importer un
fichier audio", sélectionner le fichier via le sélecteur système Android, puis
attendre la fin de la transcription.

- [ ] Comme attendu : le sélecteur revient dans l'application, l'import aboutit et une transcription non vide est produite sans réseau.
- [ ] Le sélecteur renvoie dans l'application mais l'URI `content://` est refusée ou illisible.
- [ ] L'import aboutit mais la transcription échoue (message à recopier sans contenu privé).
- [ ] Une connexion réseau semble nécessaire ou le mode avion empêche le traitement.
- [ ] Autre (décrire) :

Conserver une capture du résultat avec uniquement le corpus public et noter la
durée audio ainsi que la durée de traitement observée.

### AND-05 - Persistance après redémarrage

**Étapes** : fermer complètement InterviewScribe depuis les applications
récentes, la relancer et rouvrir l'entretien créé par AND-04.

- [ ] Comme attendu : l'entretien, sa transcription et ses horodatages sont présents et l'audio reste lisible.
- [ ] L'entretien ou une partie de ses données a disparu.
- [ ] L'application plante ou reste bloquée au redémarrage.
- [ ] Autre (décrire) :

À la fin, noter la batterie restante et confirmer que les captures et journaux
ne contiennent ni entretien réel, ni identifiant de téléphone, ni jeton.

---

## Récapitulatif à me renvoyer

Pour chaque test coché autrement que "Comme attendu" :

```
ID du test :
Plateforme :
Ce qui a été observé :
Contenu du panneau Diagnostics (copier/coller) :
```
