//! Inference sessions — stateful conversations with the model.

use std::{ffi::CStr, ptr::NonNull, sync::Arc};

use litert_lm_sys as sys;

use crate::{engine::EngineInner, Error, Result, SamplerParams};

/// A stateful session for generating text with an [`Engine`](crate::Engine).
///
/// Each session maintains its own conversation context (KV cache). Create
/// multiple sessions to run independent conversations in parallel.
pub struct Session {
    ptr: NonNull<sys::LiteRtLmSession>,
    _engine: Arc<EngineInner>,
}

// Send but !Sync — individual sessions aren't designed for shared access.
unsafe impl Send for Session {}

impl Session {
    pub(crate) fn new(engine: Arc<EngineInner>, params: SamplerParams) -> Result<Self> {
        let config = unsafe { sys::litert_lm_session_config_create() };
        if config.is_null() {
            return Err(Error::NullPointer);
        }

        let raw_params = params.to_raw();
        unsafe {
            sys::litert_lm_session_config_set_sampler_params(config, &raw_params);
        }

        let session_ptr =
            unsafe { sys::litert_lm_engine_create_session(engine.ptr.as_ptr(), config) };
        unsafe { sys::litert_lm_session_config_delete(config) };

        let ptr = NonNull::new(session_ptr).ok_or(Error::SessionCreationFailed)?;
        Ok(Self {
            ptr,
            _engine: engine,
        })
    }

    /// Generates a response for the given text prompt.
    ///
    /// Returns the model's full response as a string. The conversation
    /// context is updated — subsequent calls see prior turns.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GenerationFailed`] if the engine returns null.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litert_lm::{Engine, EngineSettings, SamplerParams};
    /// # fn demo(engine: &Engine) -> litert_lm::Result<()> {
    /// let mut session = engine.create_session(SamplerParams::default())?;
    /// let response = session.generate("Explain Rust lifetimes briefly")?;
    /// println!("{response}");
    /// # Ok(()) }
    /// ```
    pub fn generate(&mut self, prompt: &str) -> Result<String> {
        let input = sys::InputData {
            type_: sys::kInputText,
            data: prompt.as_ptr().cast(),
            size: prompt.len(),
        };
        let responses =
            unsafe { sys::litert_lm_session_generate_content(self.ptr.as_ptr(), &input, 1) };
        if responses.is_null() {
            return Err(Error::GenerationFailed);
        }

        let num = unsafe { sys::litert_lm_responses_get_num_candidates(responses) };
        let text = if num > 0 {
            let raw = unsafe { sys::litert_lm_responses_get_response_text_at(responses, 0) };
            if raw.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(raw) }
                    .to_string_lossy()
                    .into_owned()
            }
        } else {
            String::new()
        };

        unsafe { sys::litert_lm_responses_delete(responses) };
        Ok(text)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { sys::litert_lm_session_delete(self.ptr.as_ptr()) }
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("ptr", &self.ptr.as_ptr())
            .finish()
    }
}
