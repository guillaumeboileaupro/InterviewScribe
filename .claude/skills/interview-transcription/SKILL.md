---
name: interview-transcription
description: Implement or review InterviewScribe transcription, diarization, timestamps, cleanup, export, audio capture, or privacy behavior.
---

# Interview transcription

Read `AGENTS.md`, `docs/PRODUCT.md` and `docs/ARCHITECTURE.md`. Keep raw text immutable, cleaned text reversible, timestamps stored, unknown speakers neutral and processing local by default. Whisper performs transcription, not speaker diarization. Test silence, meaningful filler words, repetitions, overlap, noise and interrupted recovery.

