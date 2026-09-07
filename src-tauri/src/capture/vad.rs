/// Energy-threshold voice activity detection. Deterministic and cheap - no
/// ML model, no new bundled resource (see docs/ARCHITECTURE.md "Pipeline
/// temps reel"): the goal here is finding the silences that delimit a chunk
/// to transcribe, not word-level detection.
#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    /// RMS energy above which speech is considered to have started.
    pub high_threshold: f32,
    /// RMS energy below which speech is considered to have stopped. Kept
    /// lower than `high_threshold` (hysteresis) so a level hovering around
    /// one fixed threshold doesn't flicker active/inactive every frame.
    pub low_threshold: f32,
    /// How long energy must stay below `low_threshold` before speech is
    /// considered actually over - absorbs the brief dips between syllables
    /// so a word isn't split into several chunks.
    pub hangover_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            high_threshold: 0.02,
            low_threshold: 0.01,
            hangover_ms: 300,
        }
    }
}

pub struct Vad {
    config: VadConfig,
    active: bool,
    silence_run_ms: u32,
}

impl Vad {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            active: false,
            silence_run_ms: 0,
        }
    }

    /// Feeds one frame's RMS energy (see `rms` below) and its duration in
    /// milliseconds; returns whether speech is considered active after this
    /// frame.
    pub fn process_frame(&mut self, energy: f32, frame_ms: u32) -> bool {
        if self.active {
            if energy < self.config.low_threshold {
                self.silence_run_ms += frame_ms;
                if self.silence_run_ms >= self.config.hangover_ms {
                    self.active = false;
                }
            } else {
                self.silence_run_ms = 0;
            }
        } else if energy >= self.config.high_threshold {
            self.active = true;
            self.silence_run_ms = 0;
        }
        self.active
    }
}

/// Root-mean-square energy of a frame of samples in [-1.0, 1.0] range.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> VadConfig {
        VadConfig {
            high_threshold: 0.5,
            low_threshold: 0.2,
            hangover_ms: 300,
        }
    }

    #[test]
    fn pure_silence_stays_inactive() {
        let mut vad = Vad::new(config());
        for _ in 0..10 {
            assert!(!vad.process_frame(0.0, 20));
        }
    }

    #[test]
    fn energy_above_high_threshold_activates_immediately() {
        let mut vad = Vad::new(config());
        assert!(vad.process_frame(0.6, 20));
    }

    #[test]
    fn a_level_between_thresholds_never_triggers_activation() {
        let mut vad = Vad::new(config());
        for _ in 0..20 {
            assert!(!vad.process_frame(0.35, 20));
        }
    }

    #[test]
    fn a_level_between_thresholds_does_not_end_an_active_run() {
        let mut vad = Vad::new(config());
        assert!(vad.process_frame(0.6, 20));
        // Between low and high: neither speech nor silence by the hysteresis
        // rule, so an already-active run must not reset its hangover timer
        // and must stay active.
        for _ in 0..20 {
            assert!(vad.process_frame(0.35, 20));
        }
    }

    #[test]
    fn a_brief_dip_shorter_than_hangover_does_not_end_speech() {
        let mut vad = Vad::new(config());
        assert!(vad.process_frame(0.6, 20));
        // 100ms of near-silence, well under the 300ms hangover.
        for _ in 0..5 {
            assert!(vad.process_frame(0.0, 20));
        }
        // Speech resumes: still considered the same active run.
        assert!(vad.process_frame(0.6, 20));
    }

    #[test]
    fn silence_longer_than_hangover_ends_speech() {
        let mut vad = Vad::new(config());
        assert!(vad.process_frame(0.6, 20));
        let mut last = true;
        for _ in 0..20 {
            last = vad.process_frame(0.0, 20);
        }
        assert!(!last);
    }

    #[test]
    fn a_full_speech_then_silence_cycle_activates_exactly_once() {
        let mut vad = Vad::new(config());
        let mut transitions_to_active = 0;
        let mut was_active = false;
        let mut frames = vec![0.0, 0.0, 0.6, 0.6, 0.6];
        frames.extend(std::iter::repeat_n(0.0, 20)); // 400ms of trailing silence, past the 300ms hangover
        for energy in frames {
            let active = vad.process_frame(energy, 20);
            if active && !was_active {
                transitions_to_active += 1;
            }
            was_active = active;
        }
        assert_eq!(transitions_to_active, 1);
        assert!(!was_active);
    }

    #[test]
    fn rms_of_silence_is_zero() {
        assert_eq!(rms(&[0.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn rms_of_empty_slice_is_zero() {
        assert_eq!(rms(&[]), 0.0);
    }

    #[test]
    fn rms_of_a_constant_tone_matches_its_amplitude() {
        let samples = vec![0.5, -0.5, 0.5, -0.5];
        assert!((rms(&samples) - 0.5).abs() < 1e-6);
    }
}
