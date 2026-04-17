//! LLM engine — the entry point for on-device language model inference.

use std::{
    ffi::CString,
    path::{Path, PathBuf},
    ptr::NonNull,
    sync::Arc,
};

use litert_lm_sys as sys;

use crate::{Error, Result, SamplerParams, Session};

/// Inference backend for the LLM engine.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// CPU reference backend. Always available, slower.
    Cpu,
    /// GPU backend (Metal on Apple, WebGPU elsewhere). Falls back to CPU if
    /// no GPU accelerator is registered.
    #[default]
    Gpu,
    /// NPU backend. Available on a narrow set of platforms.
    Npu,
}

impl Backend {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Gpu => "GPU",
            Self::Npu => "NPU",
        }
    }
}

/// Configuration for constructing an [`Engine`].
///
/// # Example
///
/// ```no_run
/// use litert_lm::EngineSettings;
///
/// let settings = EngineSettings::new("gemma2-2b-it.litertlm")
///     .backend("gpu")
///     .max_num_tokens(1024)
///     .cache_dir("/tmp/litert-lm-cache");
/// ```
pub struct EngineSettings {
    model_path: PathBuf,
    backend: Backend,
    vision_backend: Backend,
    audio_backend: Backend,
    max_num_tokens: Option<i32>,
    cache_dir: Option<PathBuf>,
}

impl EngineSettings {
    /// Create settings for a model file at the given path.
    /// Defaults to [`Backend::Gpu`] for all modalities.
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            backend: Backend::default(),
            vision_backend: Backend::default(),
            audio_backend: Backend::default(),
            max_num_tokens: None,
            cache_dir: None,
        }
    }

    /// Set the main inference backend (text / prefill+decode).
    #[must_use]
    pub fn backend(mut self, backend: Backend) -> Self {
        self.backend = backend;
        self
    }

    /// Set the vision encoder backend. Models without a vision encoder
    /// ignore this setting.
    #[must_use]
    pub fn vision_backend(mut self, backend: Backend) -> Self {
        self.vision_backend = backend;
        self
    }

    /// Set the audio encoder backend. Models without an audio encoder
    /// ignore this setting.
    #[must_use]
    pub fn audio_backend(mut self, backend: Backend) -> Self {
        self.audio_backend = backend;
        self
    }

    /// Maximum number of tokens (context window size).
    #[must_use]
    pub fn max_num_tokens(mut self, n: i32) -> Self {
        self.max_num_tokens = Some(n);
        self
    }

    /// Directory for runtime caches (KV cache, compiled shaders, etc.).
    #[must_use]
    pub fn cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(dir.into());
        self
    }
}

/// An LLM engine loaded from a `.litertlm` model file.
///
/// Create one via [`Engine::new`], then spawn [`Session`]s for inference.
/// The engine is reference-counted and thread-safe (`Send + Sync`).
pub struct Engine {
    inner: Arc<EngineInner>,
}

pub(crate) struct EngineInner {
    pub(crate) ptr: NonNull<sys::LiteRtLmEngine>,
}

// Safety: LiteRtLmEngine is documented as thread-safe; sessions
// serialise internally.
unsafe impl Send for EngineInner {}
unsafe impl Sync for EngineInner {}

impl Engine {
    /// Loads a model and creates an engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EngineCreationFailed`] if the C API returns null
    /// (model file missing, unsupported format, resource exhaustion, etc.).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use litert_lm::{Engine, EngineSettings};
    ///
    /// let engine = Engine::new(EngineSettings::new("gemma2-2b-it.litertlm"))?;
    /// # Ok::<(), litert_lm::Error>(())
    /// ```
    pub fn new(settings: EngineSettings) -> Result<Self> {
        let model_str = path_to_cstring(&settings.model_path)?;
        let backend_str = CString::new(settings.backend.as_str()).unwrap();
        let vision_str = CString::new(settings.vision_backend.as_str()).unwrap();
        let audio_str = CString::new(settings.audio_backend.as_str()).unwrap();

        let raw_settings = unsafe {
            sys::litert_lm_engine_settings_create(
                model_str.as_ptr(),
                backend_str.as_ptr(),
                vision_str.as_ptr(),
                audio_str.as_ptr(),
            )
        };
        if raw_settings.is_null() {
            return Err(Error::NullPointer);
        }

        if let Some(n) = settings.max_num_tokens {
            unsafe { sys::litert_lm_engine_settings_set_max_num_tokens(raw_settings, n) };
        }
        if let Some(ref dir) = settings.cache_dir {
            let dir_str = path_to_cstring(dir)?;
            unsafe { sys::litert_lm_engine_settings_set_cache_dir(raw_settings, dir_str.as_ptr()) };
        }

        let engine_ptr = unsafe { sys::litert_lm_engine_create(raw_settings) };
        unsafe { sys::litert_lm_engine_settings_delete(raw_settings) };

        let ptr = NonNull::new(engine_ptr).ok_or(Error::EngineCreationFailed)?;
        Ok(Self {
            inner: Arc::new(EngineInner { ptr }),
        })
    }

    /// Creates a new inference session with the given sampling parameters.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SessionCreationFailed`] if the runtime rejects the
    /// configuration.
    pub fn create_session(&self, params: SamplerParams) -> Result<Session> {
        Session::new(self.inner.clone(), params)
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw(&self) -> *mut sys::LiteRtLmEngine {
        self.inner.ptr.as_ptr()
    }
}

impl Drop for EngineInner {
    fn drop(&mut self) {
        unsafe { sys::litert_lm_engine_delete(self.ptr.as_ptr()) }
    }
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("ptr", &self.inner.ptr.as_ptr())
            .finish()
    }
}

fn path_to_cstring(path: &Path) -> Result<CString> {
    let s = path
        .to_str()
        .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))?;
    CString::new(s).map_err(|_| Error::InvalidPath(path.to_path_buf()))
}
