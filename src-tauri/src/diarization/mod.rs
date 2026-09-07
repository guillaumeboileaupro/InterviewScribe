use crate::error::AppError;

/// One detected speaker cluster: a running mean of every embedding assigned to
/// it so far, not a single fixed reference vector - this lets the cluster
/// track natural voice variation instead of drifting away from a speaker
/// whose first utterance happened to be atypical.
struct SpeakerCluster {
    centroid: Vec<f32>,
    count: usize,
}

impl SpeakerCluster {
    fn new(embedding: &[f32]) -> Self {
        Self {
            centroid: embedding.to_vec(),
            count: 1,
        }
    }

    fn update(&mut self, embedding: &[f32]) {
        let n = self.count as f32;
        for (c, e) in self.centroid.iter_mut().zip(embedding) {
            *c = (*c * n + e) / (n + 1.0);
        }
        self.count += 1;
    }
}

/// The speaker a segment was assigned to, plus whether that assignment is
/// confident enough to show without a caveat. Never forced silently: a
/// low-margin match still returns the best guess (index), but flags
/// `uncertain` so the caller can mark the segment instead of pretending
/// certainty it doesn't have (docs/ARCHITECTURE.md: "les recouvrements de
/// voix doivent etre signales comme incertains plutot que forces vers un
/// seul locuteur").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Assignment {
    pub speaker_index: usize,
    pub uncertain: bool,
}

/// Incremental, deterministic speaker clustering over voice embeddings.
/// Reuses Whisper's own segment boundaries (see transcription::RawSegment) -
/// this never runs its own voice-activity detection, only groups the audio
/// spans Whisper already produced.
pub struct Clusterer {
    clusters: Vec<SpeakerCluster>,
    max_speakers: Option<usize>,
    match_threshold: f32,
    uncertain_margin: f32,
}

impl Clusterer {
    /// `max_speakers` is the optional user hint ("nombre attendu de
    /// personnes" in docs/PRODUCT.md); `None` estimates the count freely.
    pub fn new(max_speakers: Option<usize>) -> Self {
        Self {
            clusters: Vec::new(),
            max_speakers,
            match_threshold: 0.55,
            uncertain_margin: 0.08,
        }
    }

    pub fn assign(&mut self, embedding: &[f32]) -> Assignment {
        if self.clusters.is_empty() {
            self.clusters.push(SpeakerCluster::new(embedding));
            return Assignment {
                speaker_index: 0,
                uncertain: false,
            };
        }

        let mut scores: Vec<(usize, f32)> = self
            .clusters
            .iter()
            .enumerate()
            .map(|(index, cluster)| (index, cosine_similarity(&cluster.centroid, embedding)))
            .collect();
        scores.sort_by(|a, b| b.1.total_cmp(&a.1));
        let (best_index, best_score) = scores[0];
        let runner_up_score = scores.get(1).map(|(_, score)| *score);
        let uncertain = runner_up_score
            .map(|runner_up| (best_score - runner_up).abs() < self.uncertain_margin)
            .unwrap_or(false);

        let at_capacity = self
            .max_speakers
            .is_some_and(|max| self.clusters.len() >= max);

        if best_score >= self.match_threshold || at_capacity {
            self.clusters[best_index].update(embedding);
            Assignment {
                speaker_index: best_index,
                uncertain,
            }
        } else {
            self.clusters.push(SpeakerCluster::new(embedding));
            Assignment {
                speaker_index: self.clusters.len() - 1,
                uncertain: false,
            }
        }
    }

    pub fn speaker_count(&self) -> usize {
        self.clusters.len()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Converts the interview's f32 PCM (the shared decode pipeline's output) to
/// the i16 samples pyannote_rs::EmbeddingExtractor expects.
pub fn pcm_f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

/// Slices the interview's full PCM buffer to one segment's audio, given
/// millisecond timestamps and the pipeline's fixed 16kHz sample rate.
pub fn slice_pcm_ms(pcm: &[f32], start_ms: i64, end_ms: i64) -> &[f32] {
    const SAMPLES_PER_MS: i64 = 16;
    let start = (start_ms * SAMPLES_PER_MS).max(0) as usize;
    let end = ((end_ms * SAMPLES_PER_MS).max(0) as usize).min(pcm.len());
    if start >= end {
        &[]
    } else {
        &pcm[start..end]
    }
}

/// Thin adapter over pyannote_rs so `Clusterer` above never depends on the
/// ONNX model or the `ort`/`pyannote-rs` types directly - it only ever sees
/// `&[f32]` embeddings, which keeps its logic testable with hand-built
/// vectors and no model file (see tests below).
///
/// Not available on Android: `ort-sys`'s prebuilt-binary table has no
/// Android entry at all (confirmed by reading its build.rs - not a missing
/// NDK/cmake toolchain issue like whisper-rs-sys had, an actual gap in what
/// upstream publishes), so `pyannote-rs`/`ort` are Android-excluded at the
/// Cargo.toml dependency level too (`[target.'cfg(not(target_os =
/// "android"))'.dependencies]`) - this type simply cannot exist on that
/// target. See docs/ARCHITECTURE.md "Diarisation".
#[cfg(not(target_os = "android"))]
pub struct EmbeddingExtractor {
    inner: pyannote_rs::EmbeddingExtractor,
}

#[cfg(not(target_os = "android"))]
impl EmbeddingExtractor {
    pub fn load(model_path: &std::path::Path) -> Result<Self, AppError> {
        let inner = pyannote_rs::EmbeddingExtractor::new(model_path)
            .map_err(|err| AppError::Model(format!("modele de diarisation invalide: {err}")))?;
        Ok(Self { inner })
    }

    pub fn extract(&mut self, pcm_slice: &[f32]) -> Result<Vec<f32>, AppError> {
        let samples = pcm_f32_to_i16(pcm_slice);
        self.inner
            .compute(&samples)
            .map(|embedding| embedding.collect())
            .map_err(|err| {
                AppError::Transcription(format!("extraction d'empreinte vocale echouee: {err}"))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec3(x: f32, y: f32, z: f32) -> Vec<f32> {
        vec![x, y, z]
    }

    #[test]
    fn cosine_similarity_identical_vectors_is_one() {
        let a = vec3(1.0, 2.0, 3.0);
        assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors_is_zero() {
        let a = vec3(1.0, 0.0, 0.0);
        let b = vec3(0.0, 1.0, 0.0);
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn two_distinct_embeddings_become_two_confident_clusters() {
        let mut clusterer = Clusterer::new(None);
        let a = clusterer.assign(&vec3(1.0, 0.0, 0.0));
        let b = clusterer.assign(&vec3(0.0, 1.0, 0.0));
        assert_ne!(a.speaker_index, b.speaker_index);
        assert!(!a.uncertain);
        assert!(!b.uncertain);
        assert_eq!(clusterer.speaker_count(), 2);
    }

    #[test]
    fn a_similar_third_embedding_joins_the_first_cluster_and_moves_its_centroid() {
        let mut clusterer = Clusterer::new(None);
        clusterer.assign(&vec3(1.0, 0.0, 0.0));
        clusterer.assign(&vec3(0.0, 1.0, 0.0));
        let third = clusterer.assign(&vec3(0.9, 0.1, 0.0));
        assert_eq!(third.speaker_index, 0);
        assert_eq!(clusterer.speaker_count(), 2);

        // The centroid moved towards the average of the two speaker-0
        // embeddings, not stuck on the original fixed vector.
        assert_eq!(clusterer.clusters[0].centroid, vec3(0.95, 0.05, 0.0));
    }

    #[test]
    fn an_embedding_equidistant_between_two_clusters_is_uncertain() {
        let mut clusterer = Clusterer::new(None);
        clusterer.assign(&vec3(1.0, 0.0, 0.0));
        clusterer.assign(&vec3(0.0, 1.0, 0.0));
        let ambiguous = clusterer.assign(&vec3(1.0, 1.0, 0.0));
        assert!(ambiguous.uncertain);
    }

    #[test]
    fn max_speakers_hint_forces_best_match_instead_of_a_new_cluster() {
        let mut clusterer = Clusterer::new(Some(2));
        clusterer.assign(&vec3(1.0, 0.0, 0.0));
        clusterer.assign(&vec3(0.0, 1.0, 0.0));
        let third = clusterer.assign(&vec3(0.0, 0.0, 1.0));
        assert_eq!(clusterer.speaker_count(), 2);
        assert!(third.speaker_index < 2);
    }

    #[test]
    fn a_single_embedding_stays_a_single_speaker() {
        let mut clusterer = Clusterer::new(None);
        let only = clusterer.assign(&vec3(0.5, 0.5, 0.5));
        assert_eq!(only.speaker_index, 0);
        assert!(!only.uncertain);
        assert_eq!(clusterer.speaker_count(), 1);
    }

    #[test]
    fn slice_pcm_ms_extracts_the_right_sample_range() {
        let pcm: Vec<f32> = (0..320).map(|i| i as f32).collect(); // 20ms at 16kHz
        let slice = slice_pcm_ms(&pcm, 5, 10);
        assert_eq!(slice.len(), 80); // 5ms at 16 samples/ms
        assert_eq!(slice[0], 80.0);
    }

    #[test]
    fn slice_pcm_ms_clamps_to_buffer_bounds() {
        let pcm: Vec<f32> = vec![0.0; 160];
        assert!(slice_pcm_ms(&pcm, 0, 1_000).len() <= 160);
        assert!(slice_pcm_ms(&pcm, 1_000, 2_000).is_empty());
    }

    #[test]
    fn pcm_f32_to_i16_round_trips_extremes() {
        let converted = pcm_f32_to_i16(&[-1.0, 0.0, 1.0]);
        assert_eq!(converted[1], 0);
        assert!(converted[0] < -32000);
        assert!(converted[2] > 32000);
    }
}
