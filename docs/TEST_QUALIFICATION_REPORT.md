# Rapport de qualification audio

Ce rapport ne contient ni audio ni transcription brute. Les sources restent
hors Git et sont identifiees uniquement par leur empreinte publique.

## Campagne 2026-09-08 — francais mono-locuteur

| Champ | Valeur |
| --- | --- |
| Plateforme | Linux x86_64, execution locale `--release` |
| Modele | Whisper Large v3 Turbo Q5_0 fourni avec l'application |
| Source | `wikimedia-fr-chat-gorge` |
| SHA-256 source | `bf667abb1a8811ef3ed96f8838abc895122f717242b3679f9efb91394f3fe5fd` |
| Langue imposee | `fr` |
| WER | 0,0000 |
| CER | 0,0000 |
| Seuils | WER <= 0,45 ; CER <= 0,30 |
| Duree du test | 149,15 s, hors compilation initiale |
| Resultat | Reussi |

Commande reproductible :

```bash
INTERVIEWSCRIBE_TEST_MODEL=/chemin/ggml-large-v3-turbo-q5_0.bin \
INTERVIEWSCRIBE_TEST_WAV=/chemin/fr-chat-gorge.ogg \
cargo test --release --manifest-path src-tauri/Cargo.toml \
  whisper_public_french_quality_thresholds -- --ignored --nocapture
```

## Campagne 2026-09-08 — AMI quatre locuteurs

| Champ | Valeur |
| --- | --- |
| Plateforme | Linux x86_64, execution locale |
| Modeles | Whisper Large v3 Turbo Q5_0 et WeSpeaker CAM++ fournis avec l'application |
| Source | `ami-es2002a-mix-headset`, fenetre 69–89 s |
| SHA-256 fixture | `0b9d3c2cb5a59334ec783348afc1382515b27461f1ac12470cc513616b0daea4` |
| Reference | 59 mots ; au moins 2,7 s de parole par locuteur A-D |
| Segments / clusters | 6 / 4 |
| WER | 0,0678 |
| CER | 0,0500 |
| DER sur tours de parole | 0,5148 |
| DER naif sur regions de mots | 0,5200 |
| Parole manquee | 0 ms |
| Fausse alarme | 2 790 ms |
| Confusion de locuteur | 6 070 ms |
| Couverture `uncertain` | 0,0000 |
| Doublons | 0,0000 |
| Seuil DER | <= 0,50 |
| Duree du test | 190,73 s, hors compilation initiale |
| Resultat | Echec explicite : DER depasse le seuil de 0,0148 |

Un premier extrait de 10 s a ete rejete comme non representatif : seulement
deux clusters sur quatre et DER 0,9618. Le passage a 20 s a bien produit quatre
clusters et fortement ameliore les mesures. La reference mot a mot a ensuite ete
convertie en tours de parole en regroupant, pour un meme locuteur, les pauses de
500 ms ou moins. Les deux DER sont publies pour rendre ce choix visible. Les
6 070 ms de confusion montrent que le depassement restant vient principalement
des segments Whisper qui couvrent plusieurs tours de parole, alors que le pipeline
attribue actuellement une seule empreinte et un seul locuteur a chaque segment.
Le seuil reste volontairement inchange.

Commande reproductible apres `pnpm corpus:prepare` :

```bash
INTERVIEWSCRIBE_TEST_MODEL=/chemin/ggml-large-v3-turbo-q5_0.bin \
INTERVIEWSCRIBE_TEST_DIARIZATION_MODEL=/chemin/wespeaker_en_voxceleb_CAM++.onnx \
INTERVIEWSCRIBE_TEST_WAV=tests/corpus/generated/multi-clean-4.wav \
INTERVIEWSCRIBE_TEST_AMI_REFERENCE=tests/corpus/generated/ami-reference.json \
cargo test --manifest-path src-tauri/Cargo.toml \
  ami_four_speaker_quality_metrics -- --ignored --nocapture
```

## Reste a qualifier

- AMI quatre locuteurs : diagnostiquer le DER 0,5200 puis qualifier la derive ;
- variantes deterministes bruit, tons musicaux et chevauchement ;
- Windows et Android ;
- davantage d'accents francophones.
