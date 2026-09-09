# Qualification manuelle et materielle

Les tests qui exigent un microphone reel, un lecteur d'ecran ou un telephone
physique sont executes avec le protocole correspondant de
`E2E_TEST_CAMPAIGN.md`. Ils ne sont jamais marques reussis par defaut.

Apres l'execution :

1. ouvrir l'action GitHub `Manual hardware qualification report` ;
2. choisir le protocole, la plateforme et le resultat reellement observes ;
3. saisir uniquement la version de l'application ou le commit teste ;
4. conserver le rapport JSON produit avec les autres preuves de la release.

Le formulaire n'accepte aucun commentaire libre. Ne jamais joindre un audio,
une transcription, un numero de serie, un IMEI, un compte ou un jeton. Une
capture necessaire est conservee hors du depot apres verification manuelle
qu'elle ne contient aucune donnee privee.

Le workflow nocturne `Nightly qualification` peut aussi etre declenche
manuellement pour les tests avec modeles et corpus publics. Une attestation
`passed` ne remplace pas leur resultat automatise et ne ferme pas une case
`MATERIEL` sans execution du protocole sur la plateforme indiquee.
