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

## Reste a qualifier

- AMI quatre locuteurs : DER, confusion, couverture `uncertain` et derive ;
- variantes deterministes bruit, tons musicaux et chevauchement ;
- Windows et Android ;
- davantage d'accents francophones.
