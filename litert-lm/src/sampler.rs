//! Sampling parameters for text generation.

use litert_lm_sys as sys;

/// The sampling strategy used during token selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sampler {
    /// Pick the token with maximum logit (argmax).
    Greedy,
    /// Probabilistically pick among the top-k tokens.
    TopK,
    /// Probabilistically pick from the nucleus (top-p).
    TopP,
}

impl Sampler {
    fn to_raw(self) -> sys::Type {
        match self {
            Self::Greedy => sys::kGreedy,
            Self::TopK => sys::kTopK,
            Self::TopP => sys::kTopP,
        }
    }
}

/// Parameters controlling how the model selects the next token.
///
/// # Example
///
/// ```no_run
/// use litert_lm::SamplerParams;
///
/// let params = SamplerParams::default()
///     .top_k(40)
///     .top_p(0.95)
///     .temperature(0.8)
///     .seed(42);
/// ```
#[derive(Debug, Clone)]
pub struct SamplerParams {
    pub(crate) sampler: Sampler,
    pub(crate) top_k: i32,
    pub(crate) top_p: f32,
    pub(crate) temperature: f32,
    pub(crate) seed: i32,
}

impl Default for SamplerParams {
    fn default() -> Self {
        Self {
            sampler: Sampler::TopK,
            top_k: 40,
            top_p: 0.95,
            temperature: 0.8,
            seed: 0,
        }
    }
}

impl SamplerParams {
    /// Set the top-k value. Implies [`Sampler::TopK`] if not already set.
    #[must_use]
    pub fn top_k(mut self, v: i32) -> Self {
        self.top_k = v;
        self.sampler = Sampler::TopK;
        self
    }

    /// Set the nucleus probability threshold.
    #[must_use]
    pub fn top_p(mut self, v: f32) -> Self {
        self.top_p = v;
        self
    }

    /// Set the softmax temperature.
    #[must_use]
    pub fn temperature(mut self, v: f32) -> Self {
        self.temperature = v;
        self
    }

    /// Set the random seed for reproducible generation.
    #[must_use]
    pub fn seed(mut self, v: i32) -> Self {
        self.seed = v;
        self
    }

    /// Use greedy decoding (argmax, deterministic).
    #[must_use]
    pub fn greedy(mut self) -> Self {
        self.sampler = Sampler::Greedy;
        self
    }

    pub(crate) fn to_raw(&self) -> sys::LiteRtLmSamplerParams {
        sys::LiteRtLmSamplerParams {
            type_: self.sampler.to_raw(),
            top_k: self.top_k,
            top_p: self.top_p,
            temperature: self.temperature,
            seed: self.seed,
        }
    }
}
