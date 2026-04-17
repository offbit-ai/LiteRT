//! High-level conversation API with proper prompt template handling.
//!
//! Unlike the raw [`Session`](crate::Session), [`Conversation`] wraps the
//! upstream `litert_lm_conversation_*` C API which applies the model's
//! prompt template (e.g. Qwen3's `<|im_start|>user\n...<|im_end|>`) and
//! supports correct token-by-token streaming.

use std::{
    ffi::{c_char, c_void, CStr, CString},
    ptr::NonNull,
    sync::{Arc, Condvar, Mutex},
};

use litert_lm_sys as sys;

use crate::{engine::EngineInner, Error, Result, SamplerParams};

/// A conversation with an LLM, handling prompt formatting and multi-turn
/// context automatically.
pub struct Conversation {
    ptr: NonNull<sys::LiteRtLmConversation>,
    _engine: Arc<EngineInner>,
}

unsafe impl Send for Conversation {}

impl Conversation {
    /// Creates a new conversation from an engine with the given sampler params.
    pub(crate) fn new(engine: Arc<EngineInner>, params: SamplerParams) -> Result<Self> {
        let config = unsafe { sys::litert_lm_session_config_create() };
        if config.is_null() {
            return Err(Error::NullPointer);
        }
        let raw_params = params.to_raw();
        unsafe { sys::litert_lm_session_config_set_sampler_params(config, &raw_params) };

        let conv_config = unsafe {
            sys::litert_lm_conversation_config_create(
                engine.ptr.as_ptr(),
                config,
                std::ptr::null(), // system_message_json
                std::ptr::null(), // tools_json
                std::ptr::null(), // messages_json
                false,            // enable_constrained_decoding
            )
        };
        unsafe { sys::litert_lm_session_config_delete(config) };
        if conv_config.is_null() {
            return Err(Error::NullPointer);
        }

        let conv_ptr =
            unsafe { sys::litert_lm_conversation_create(engine.ptr.as_ptr(), conv_config) };
        unsafe { sys::litert_lm_conversation_config_delete(conv_config) };

        let ptr = NonNull::new(conv_ptr).ok_or(Error::SessionCreationFailed)?;
        Ok(Self {
            ptr,
            _engine: engine,
        })
    }

    /// Sends a message and streams the response token-by-token.
    ///
    /// `on_token` receives each text chunk as it's generated. The model's
    /// prompt template is applied automatically. Blocks until generation
    /// completes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litertlm::{Engine, EngineSettings, SamplerParams};
    /// # fn demo(engine: &Engine) -> litertlm::Result<()> {
    /// let mut conv = engine.create_conversation(SamplerParams::default().top_p(0.95))?;
    /// conv.send_message_stream("Tell me a story", |chunk| {
    ///     print!("{chunk}");
    /// })?;
    /// # Ok(()) }
    /// ```
    pub fn send_message_stream(
        &mut self,
        prompt: &str,
        mut on_token: impl FnMut(&str),
    ) -> Result<()> {
        let message_json = format!(
            r#"{{"role":"user","content":[{{"type":"text","text":{}}}]}}"#,
            serde_json_escape(prompt)
        );
        let msg_cstr = CString::new(message_json).map_err(|_| Error::NullPointer)?;

        struct State<'a> {
            cb: &'a mut dyn FnMut(&str),
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
                // Conversation callback sends JSON: {"content":[{"text":"..."}]}
                // or plain text depending on the engine version. Try to extract
                // text from JSON; fall back to raw string.
                let raw = CStr::from_ptr(chunk).to_string_lossy();
                let text = extract_text_from_json(&raw).unwrap_or_else(|| raw.to_string());
                if !text.is_empty() {
                    (state.cb)(&text);
                }
            }
            if is_final {
                *state.done.lock().unwrap() = true;
                state.cond.notify_one();
            }
        }

        let done = Mutex::new(false);
        let cond = Condvar::new();
        let mut state = State {
            cb: &mut on_token,
            error: None,
            done: &done,
            cond: &cond,
        };

        let ret = unsafe {
            sys::litert_lm_conversation_send_message_stream(
                self.ptr.as_ptr(),
                msg_cstr.as_ptr(),
                std::ptr::null(), // extra_context
                Some(trampoline),
                &mut state as *mut State as *mut c_void,
            )
        };

        if ret != 0 {
            return Err(Error::GenerationFailed(format!(
                "conversation stream returned {ret}"
            )));
        }

        let guard = done.lock().unwrap();
        let _guard = cond.wait_while(guard, |d| !*d).unwrap();

        if let Some(err) = state.error {
            return Err(Error::GenerationFailed(err));
        }
        Ok(())
    }

    /// Sends a message and returns the full response (blocking).
    pub fn send_message(&mut self, prompt: &str) -> Result<String> {
        let mut response = String::new();
        self.send_message_stream(prompt, |chunk| {
            response.push_str(chunk);
        })?;
        Ok(response)
    }
}

impl Drop for Conversation {
    fn drop(&mut self) {
        unsafe { sys::litert_lm_conversation_delete(self.ptr.as_ptr()) }
    }
}

/// Minimal JSON string escaping for the message payload.
fn serde_json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\x20' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Try to extract text from a conversation JSON chunk.
/// Format: `{"content":[{"type":"text","text":"..."}]}` or just `{"text":"..."}`
fn extract_text_from_json(raw: &str) -> Option<String> {
    // Quick check: if it doesn't look like JSON, return None
    let trimmed = raw.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    // Naive extraction: find "text":" and extract the value
    let marker = r#""text":""#;
    let start = trimmed.find(marker)? + marker.len();
    let rest = &trimmed[start..];
    // Find the closing quote (handling escaped quotes)
    let mut end = 0;
    let mut escape = false;
    for c in rest.chars() {
        if escape {
            escape = false;
        } else if c == '\\' {
            escape = true;
        } else if c == '"' {
            break;
        }
        end += c.len_utf8();
    }
    Some(rest[..end].replace("\\n", "\n").replace("\\\"", "\""))
}
