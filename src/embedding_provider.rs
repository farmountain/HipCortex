// Chain-of-Thought: provide zero-config embeddings so callers don't need
// to bring their own embedding model. Default provider uses character n-gram
// hashing (fast, deterministic, no ML deps). Override via trait impl or
// set HIPCORTEX_EMBEDDING_MODEL env var for ONNX/Candle path.
//
// Design: EmbeddingProvider trait → default HashEmbeddingProvider → pluggable.

/// Trait for pluggable embedding generation.
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding vector for text input.
    fn embed(&self, text: &str) -> Vec<f32>;

    /// Dimensionality of the embeddings produced.
    fn dimension(&self) -> usize;

    /// Name of this provider (for logging/metrics).
    fn name(&self) -> &str;
}

/// Default embedding provider using character n-gram hashing.
///
/// Produces a fixed-size embedding by hashing overlapping character n-grams
/// into bins. No ML framework required. Deterministic for identical input.
/// Good enough for keyword/concept matching and lightweight semantic search.
pub struct HashEmbeddingProvider {
    dim: usize,
    ngram_min: usize,
    ngram_max: usize,
}

impl HashEmbeddingProvider {
    /// Create a new hash-based embedding provider.
    /// `dim` should be a power of 2 for best distribution (default 128).
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            ngram_min: 2,
            ngram_max: 5,
        }
    }

    /// Default 128-dimensional provider — good balance of quality vs speed.
    pub fn default_dim() -> Self {
        Self::new(128)
    }
}

impl EmbeddingProvider for HashEmbeddingProvider {
    fn embed(&self, text: &str) -> Vec<f32> {
        let lower = text.to_lowercase();
        let chars: Vec<char> = lower.chars().collect();
        let mut bins = vec![0.0_f32; self.dim];

        if chars.is_empty() {
            return bins;
        }

        for n in self.ngram_min..=self.ngram_max {
            if chars.len() < n {
                continue;
            }
            for window in chars.windows(n) {
                let ngram: String = window.iter().collect();
                // Hash ngram to a bin index
                let hash = fxhash::hash64(&ngram);
                let idx = (hash % self.dim as u64) as usize;
                // Weight: shorter n-grams get higher weight (more frequent)
                let weight = 1.0 / (n as f32).sqrt();
                bins[idx] += weight;
            }
        }

        // L2 normalize
        let norm: f32 = bins.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut bins {
                *v /= norm;
            }
        }

        bins
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn name(&self) -> &str {
        "hash-ngram"
    }
}

/// Simple fxhash-based hasher (no external dep needed — we inline a basic one).
mod fxhash {
    pub fn hash64(s: &str) -> u64 {
        // FNV-1a 64-bit hash — fast, good distribution
        let mut h: u64 = 0xcbf29ce484222325;
        for byte in s.bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// No-op embedding provider for testing.
pub struct NoopEmbeddingProvider;

impl EmbeddingProvider for NoopEmbeddingProvider {
    fn embed(&self, _text: &str) -> Vec<f32> {
        vec![0.0; 128]
    }

    fn dimension(&self) -> usize {
        128
    }

    fn name(&self) -> &str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_embedding_deterministic() {
        let provider = HashEmbeddingProvider::default_dim();
        let a = provider.embed("hello world");
        let b = provider.embed("hello world");
        assert_eq!(a.len(), b.len());
        assert_eq!(a, b, "same input should produce same embedding");
    }

    #[test]
    fn test_hash_embedding_dimension() {
        let provider = HashEmbeddingProvider::new(256);
        assert_eq!(provider.embed("test").len(), 256);
    }

    #[test]
    fn test_hash_embedding_normalized() {
        let provider = HashEmbeddingProvider::default_dim();
        let v = provider.embed("some text to embed");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.001 || norm == 0.0,
            "embedding should be L2 normalized, got norm {}",
            norm
        );
    }

    #[test]
    fn test_similar_texts_closer() {
        let provider = HashEmbeddingProvider::default_dim();
        let a = provider.embed("machine learning is great");
        let b = provider.embed("machine learning is awesome");
        let c = provider.embed("quantum physics and relativity");

        let sim_ab = cosine(&a, &b);
        let sim_ac = cosine(&a, &c);
        assert!(
            sim_ab > sim_ac,
            "similar texts should have higher cosine similarity: ab={:.4} ac={:.4}",
            sim_ab,
            sim_ac
        );
    }

    #[test]
    fn test_empty_text() {
        let provider = HashEmbeddingProvider::default_dim();
        let v = provider.embed("");
        assert_eq!(v.len(), 128);
        assert!(v.iter().all(|x| *x == 0.0));
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }
}
