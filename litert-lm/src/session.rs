//! Inference sessions — stateful conversations with the model.

use std::{
    ffi::{c_char, c_void, CStr},
    ptr::NonNull,
    sync::Arc,
};

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
    /// # use litertlm::{Engine, EngineSettings, SamplerParams};
    /// # fn demo(engine: &Engine) -> litertlm::Result<()> {
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
            return Err(Error::GenerationFailed("returned null".into()));
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

    /// Generates a response with token-by-token streaming.
    ///
    /// `on_token` is called for each generated chunk. Return `true` to
    /// continue, `false` to cancel early.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GenerationFailed`] if the engine reports an error
    /// via the callback.
    pub fn generate_stream(
        &mut self,
        prompt: &str,
        mut on_token: impl FnMut(&str) -> bool,
    ) -> Result<()> {
        use std::sync::{Condvar, Mutex};

        struct State<'a> {
            cb: &'a mut dyn FnMut(&str) -> bool,
            error: Option<String>,
            done: &'a Mutex<bool>,
            cond: &'a Condvar,
        }

        unsafe extern "C" fn trampoline(
            data: *mut c_void,
            chunk: *const c_char,
            is_final: bool,
            error_msg: *const c_char,
        ) {
            let state = &mut *(data as *mut State);
            if !error_msg.is_null() {
                state.error = Some(CStr::from_ptr(error_msg).to_string_lossy().into_owned());
                *state.done.lock().unwrap() = true;
                state.cond.notify_one();
                return;
            }
            if !chunk.is_null() {
                let s = CStr::from_ptr(chunk).to_string_lossy();
                (state.cb)(s.as_ref());
            }
            if is_final {
                *state.done.lock().unwrap() = true;
                state.cond.notify_one();
            }
        }

        let input = sys::InputData {
            type_: sys::kInputText,
            data: prompt.as_ptr().cast(),
            size: prompt.len(),
        };

        let done = Mutex::new(false);
        let cond = Condvar::new();
        let mut state = State {
            cb: &mut on_token,
            error: None,
            done: &done,
            cond: &cond,
        };

        let ret = unsafe {
            sys::litert_lm_session_generate_content_stream(
                self.ptr.as_ptr(),
                &input,
                1,
                Some(trampoline),
                &mut state as *mut State as *mut c_void,
            )
        };

        if ret != 0 {
            return Err(Error::GenerationFailed(format!("stream returned {ret}")));
        }

        // Block until the callback signals completion (is_final=true).
        // The C API's stream function is non-blocking; without this wait
        // the process would exit before generation finishes.
        let guard = done.lock().unwrap();
        let _guard = cond.wait_while(guard, |d| !*d).unwrap();

        if let Some(err) = state.error {
            return Err(Error::GenerationFailed(err));
        }
        Ok(())
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
