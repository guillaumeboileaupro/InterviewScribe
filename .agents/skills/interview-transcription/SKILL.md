---
name: interview-transcription
description: Implement or review InterviewScribe transcription, diarization, timestamps, cleanup, export, audio capture, or privacy behavior.
---

# Interview transcription

Read `AGENTS.md`, `docs/PRODUCT.md` and `docs/ARCHITECTURE.md` before changing the audio or text pipeline.

Preserve these boundaries:

- Treat raw transcription as immutable evidence.
- Store cleaned text as reversible edits over raw segments.
- Keep timestamps internally even when hidden in the UI or export.
- Label unknown speakers neutrally and allow manual correction.
- Never claim Whisper performs speaker diarization.
- Mark overlapping or uncertain speech instead of inventing certainty.
- Keep processing local by default and exclude sensitive content from logs.

When changing transcription behavior, test silence, filler words used meaningfully, immediate repetitions, speaker overlap, noisy audio, interrupted recordings and partial recovery.

