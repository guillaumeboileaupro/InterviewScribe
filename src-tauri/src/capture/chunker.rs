/// Splits a live audio stream into chunks ready for a full `Transcriber`
/// pass, using the VAD's speech/silence decisions as the natural cut points
/// (see docs/ARCHITECTURE.md "Pipeline temps reel"). A chunk closes either
/// on a long-enough silence after speech, or on a safety duration cap so a
/// session with no pauses at all still produces bounded chunks.
pub struct Chunker {
    max_duration_ms: u32,
    silence_to_close_ms: u32,
    buffer: Vec<f32>,
    buffered_ms: u32,
    silence_run_ms: u32,
    has_speech: bool,
}

#[derive(Debug, PartialEq)]
pub enum ChunkEvent {
    /// The buffer is still accumulating; nothing to transcribe yet.
    Provisional,
    /// A chunk is complete and ready to hand to a `Transcriber`.
    Ready(Vec<f32>),
}

impl Chunker {
    pub fn new(max_duration_ms: u32, silence_to_close_ms: u32) -> Self {
        Self {
            max_duration_ms,
            silence_to_close_ms,
            buffer: Vec::new(),
            buffered_ms: 0,
            silence_run_ms: 0,
            has_speech: false,
        }
    }

    /// Feeds one frame of samples and the VAD's decision for it.
    pub fn push_frame(&mut self, frame: &[f32], speech_active: bool, frame_ms: u32) -> ChunkEvent {
        self.buffer.extend_from_slice(frame);
        self.buffered_ms += frame_ms;

        if speech_active {
            self.has_speech = true;
            self.silence_run_ms = 0;
        } else {
            self.silence_run_ms += frame_ms;
        }

        let silence_closes_chunk =
            self.has_speech && self.silence_run_ms >= self.silence_to_close_ms;
        let duration_caps_chunk = self.buffered_ms >= self.max_duration_ms;

        if silence_closes_chunk || duration_caps_chunk {
            self.close_chunk()
        } else {
            ChunkEvent::Provisional
        }
    }

    /// Closes whatever has accumulated so far, even if empty or silent -
    /// used when the session is stopped or paused so no captured audio is
    /// ever silently dropped.
    pub fn flush(&mut self) -> ChunkEvent {
        if self.buffer.is_empty() {
            ChunkEvent::Provisional
        } else {
            self.close_chunk()
        }
    }

    fn close_chunk(&mut self) -> ChunkEvent {
        let chunk = std::mem::take(&mut self.buffer);
        self.buffered_ms = 0;
        self.silence_run_ms = 0;
        self.has_speech = false;
        ChunkEvent::Ready(chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_only_never_closes_a_chunk_before_the_duration_cap() {
        let mut chunker = Chunker::new(10_000, 500);
        for _ in 0..10 {
            assert_eq!(
                chunker.push_frame(&[0.0; 320], false, 20),
                ChunkEvent::Provisional
            );
        }
    }

    #[test]
    fn a_long_silence_after_speech_closes_the_chunk() {
        let mut chunker = Chunker::new(10_000, 100);
        assert_eq!(
            chunker.push_frame(&[0.5; 320], true, 20),
            ChunkEvent::Provisional
        );
        assert_eq!(
            chunker.push_frame(&[0.0; 320], false, 20),
            ChunkEvent::Provisional
        );
        match chunker.push_frame(&[0.0; 320], false, 100) {
            ChunkEvent::Ready(samples) => assert_eq!(samples.len(), 320 * 3),
            ChunkEvent::Provisional => panic!("expected the chunk to close"),
        }
    }

    #[test]
    fn a_brief_silence_does_not_close_the_chunk() {
        let mut chunker = Chunker::new(10_000, 500);
        chunker.push_frame(&[0.5; 320], true, 20);
        assert_eq!(
            chunker.push_frame(&[0.0; 320], false, 50),
            ChunkEvent::Provisional
        );
    }

    #[test]
    fn the_duration_cap_closes_a_chunk_even_without_silence() {
        let mut chunker = Chunker::new(100, 10_000);
        chunker.push_frame(&[0.5; 320], true, 60);
        match chunker.push_frame(&[0.5; 320], true, 60) {
            ChunkEvent::Ready(_) => {}
            ChunkEvent::Provisional => panic!("expected the duration cap to close the chunk"),
        }
    }

    #[test]
    fn no_samples_are_lost_across_several_closed_chunks() {
        let mut chunker = Chunker::new(10_000, 100);
        let mut collected = Vec::new();

        let frame_a = vec![1.0_f32; 320];
        let frame_b = vec![2.0_f32; 320];

        if let ChunkEvent::Ready(samples) = chunker.push_frame(&frame_a, true, 20) {
            collected.extend(samples);
        }
        if let ChunkEvent::Ready(samples) = chunker.push_frame(&frame_b, false, 20) {
            collected.extend(samples);
        }
        // Long silence closes the first chunk (frame_a then frame_b).
        if let ChunkEvent::Ready(samples) = chunker.push_frame(&[0.0; 320], false, 100) {
            collected.extend(samples);
        }

        assert_eq!(collected.len(), 320 * 3);
        assert_eq!(&collected[0..320], &frame_a[..]);
        assert_eq!(&collected[320..640], &frame_b[..]);
    }

    #[test]
    fn flush_closes_a_partial_chunk() {
        let mut chunker = Chunker::new(10_000, 10_000);
        chunker.push_frame(&[0.5; 320], true, 20);
        match chunker.flush() {
            ChunkEvent::Ready(samples) => assert_eq!(samples.len(), 320),
            ChunkEvent::Provisional => panic!("expected flush to close the chunk"),
        }
    }

    #[test]
    fn flush_on_an_empty_buffer_is_a_no_op() {
        let mut chunker = Chunker::new(10_000, 10_000);
        assert_eq!(chunker.flush(), ChunkEvent::Provisional);
    }
}
