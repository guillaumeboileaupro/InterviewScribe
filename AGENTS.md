# Instructions Codex - InterviewScribe

## Mission

Construire une application locale, fiable et multiplateforme qui transcrit les entretiens en temps reel ou a posteriori, distingue les intervenants et fournit une version brute ainsi qu'une version nettoyee.

## Invariants produit

- La transcription brute ne doit jamais etre reecrite ni ecrasee par le nettoyage.
- Toute suppression d'hesitation doit rester reversible et tracable.
- Ne jamais inventer un nom de locuteur. Utiliser `Intervenant 1`, `Intervenant 2`, etc. jusqu'a une modification humaine.
- L'horodatage est configurable a l'affichage et a l'export, mais reste conserve dans les donnees internes.
- Le traitement local est le comportement par defaut.
- Une erreur sur un segment ne doit pas faire perdre l'enregistrement complet.
- Toute collecte ou fonction distante requiert un consentement explicite.

## Architecture

- Interface: React, TypeScript et CSS.
- Application native: Tauri 2 et Rust.
- Transcription: Whisper compatible local, avec une couche d'abstraction pour changer de modele.
- Diarisation: composant distinct de la transcription.
- Persistance: SQLite et fichiers audio dans un espace applicatif prive.
- Cibles: Windows `.exe`, installateur Windows `.exe`, Linux `.deb`, Android `.apk`.

## Methode de travail

- Lire `docs/PRODUCT.md`, `docs/ARCHITECTURE.md` et `docs/ROADMAP.md` avant une modification structurante.
- Charger le skill concerne dans `.agents/skills/` avant un travail de transcription, d'interface ou de packaging.
- Ajouter des tests pour les regles de segmentation, nettoyage et conversion de formats.
- Ne pas annoncer une plateforme comme supportee avant la validation de son artefact sur une machine ou un emulateur cible.
- Ne jamais committer de modele Whisper, d'enregistrement reel, de transcription privee, de jeton ou de secret.

## Qualite

- TypeScript strict et aucune utilisation non justifiee de `any`.
- Rust sans `unwrap()` dans les chemins de production.
- Fonctions courtes, responsabilites separees et erreurs explicites.
- Accessibilite clavier, contrastes WCAG AA et mise en page responsive.
- Conserver les performances mesurables: latence, memoire, taille de modele et temps d'export.

