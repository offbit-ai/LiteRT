//! LLM engine — the entry point for on-device language model inference.

use std::{
    ffi::CString,
    path::{Path, PathBuf},
    ptr::NonNull,
    sync::Arc,
};

use litert_lm_sys as sys;

use crate::{Error, Result, SamplerParams, Session};

/// Configuration for constructing an [`Engine`].
///
/// # Example
///
/// ```no_run
/// use litert_lm::EngineSettings;
///
/// let settings = EngineSettings::new("gemma2-2b-it.litertlm")
///     .max_num_tokens(1024)
///     .cache_dir("/tmp/litert-lm-cache");
/// ```
pub struct EngineSettings {
    model_path: PathBuf,
    max_num_tokens: Option<i32>,
    cache_dir: Option<PathBuf>,
}

impl EngineSettings {
    /// Create settings for a model file at the given path.
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            max_num_tokens: None,
            cache_dir: None,
        }
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

        // Backend strings: null means "auto-detect" for each modality.
        let raw_settings = unsafe {
            sys::litert_lm_engine_settings_create(
                model_str.as_ptr(),
                std::ptr::null(), // backend
                std::ptr::null(), // vision backend
                std::ptr::null(), // audio backend
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
