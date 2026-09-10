# Direction UI/UX

## Intention

L'application doit evoquer un outil editorial professionnel: calme, precis, lisible et discret. Le texte et l'etat de l'enregistrement sont prioritaires.

## Ecrans

- Accueil: nouveau direct, importer un fichier, entretiens recents.
- Preparation: microphone, langue, modele et nombre de personnes optionnel.
- Session: chronometre, niveau audio, controles et transcription en cours.
- Edition: locuteurs, segments, brut/nettoye, recherche et export.
- Reglages: modeles locaux, stockage, confidentialite et apparence.

## Systeme visuel

- Palette neutre avec un accent unique bleu profond.
- Couleurs des locuteurs reservees aux reperes, jamais a de grandes surfaces.
- Typographie sans serif pour l'interface et largeur de lecture limitee.
- Espacement regulier, bordures fines et ombres tres discretes.
- Rayons moderes, sans effet verre, gradient decoratif ou animation gratuite.
- Mode clair et sombre avec contrastes WCAG AA.

## Regles d'interaction

- Un controle primaire visible par etape.
- Etat d'enregistrement impossible a confondre avec pause ou arret.
- Confirmation avant suppression definitive d'un entretien.
- Annulation disponible pour toute modification de transcription.
- Raccourcis clavier documentes sur bureau.
- Cibles tactiles d'au moins 44 par 44 pixels sur mobile.
- Ne pas utiliser la couleur comme seul indicateur.

### Navigation clavier sur bureau

- `Tab` et `Maj+Tab` parcourent les controles dans l'ordre visuel. Dans une
  confirmation modale, le focus reste borne aux actions du dialogue.
- `Entree` et `Espace` activent les boutons et controles natifs lorsqu'ils ont
  le focus.
- `Echap` ferme la confirmation de suppression sans supprimer l'entretien et
  rend le focus au bouton qui l'a ouverte.
- Aucun raccourci global avec lettre ou modificateur n'est reserve pour le
  moment, afin de ne pas intercepter la saisie ou les raccourcis du systeme.


## Interface du prototype

L’interface utilise le logo approuve dans `assets/interviewscribe-logo.svg` et
les couleurs du skill `brand-system-design`. La navigation regroupe la
bibliotheque, un apercu de l’editeur et les reglages. Sur petit ecran, elle
passe au-dessus du contenu. Les themes clair, sombre et systeme sont disponibles;
le choix est conserve uniquement pendant la session.

La bibliotheque charge les entretiens depuis SQLite et presente un etat vide en leur absence. Les extraits affiches sont des
exemples fictifs explicitement identifies. L’apercu de l’editeur est en lecture
seule et permet de masquer les horodatages sans supprimer les donnees.
La preparation permet d’importer un fichier et de le transcrire avec Whisper local.
Les modeles Base, Small et Large v3 Turbo sont fournis avec l’application. Base
est selectionne par defaut pour garder l’interface utilisable sur CPU; les deux
profils plus precis restent explicites. Les reglages affichent leur nom et leur
etat, sans bouton de telechargement. Un paquet incomplet affiche
une erreur demandant de reinstaller la version complete.
La capture microphone propose demarrage, pause, reprise et arret. La page de
detail conserve le transcript comme contenu principal et ajoute un lecteur
audio compact ainsi qu'un commentaire optionnel. Les horodatages sont des
boutons clavier qui lancent la lecture au debut du segment; le segment actif est
signale par un fond discret en plus de la position visible du lecteur.

La transcription a posteriori affiche une progression globale et cinq etapes
distinctes: modele, preparation audio, Whisper, intervenants et sauvegarde. La
capture montre separement l'ecriture audio continue et la portion stabilisee;
la recuperation et les exports ont egalement un etat de travail visible. Les
horodatages passent en `HH:MM:SS` apres une heure. Les transcriptions longues
sont rendues par lots de 200 segments, tandis que la vue en direct reste bornee
aux 100 derniers segments. L'edition reversible, les intervenants et les exports TXT, Markdown,
JSON, SRT, VTT, DOCX et PDF sont disponibles. Une validation visuelle du lecteur,
de la progression et de la correlation segment/audio reste necessaire sur les
applications installees et sur Android.
