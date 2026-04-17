//! Multimodal input types for LLM inference.

use litert_lm_sys as sys;

/// An input to the LLM engine. Models may accept text, images, audio, or
/// combinations thereof depending on their encoder sections.
#[derive(Debug, Clone)]
pub enum Input<'a> {
    /// A text prompt or message.
    Text(&'a str),
    /// Raw image bytes (JPEG, PNG, or the format the model expects).
    Image(&'a [u8]),
    /// Signals the end of an image input sequence (for models that
    /// process multiple image frames).
    ImageEnd,
    /// Raw audio bytes (WAV, PCM, or the format the model expects).
    Audio(&'a [u8]),
    /// Signals the end of an audio input sequence.
    AudioEnd,
}

impl<'a> Input<'a> {
    /// Convenience: create a text input.
    pub fn text(s: &'a str) -> Self {
        Self::Text(s)
    }

    /// Convenience: create an image input from raw bytes.
    pub fn image(data: &'a [u8]) -> Self {
        Self::Image(data)
    }

    /// Convenience: create an audio input from raw bytes.
    pub fn audio(data: &'a [u8]) -> Self {
        Self::Audio(data)
    }

    pub(crate) fn to_raw(&self) -> sys::InputData {
        match self {
            Input::Text(s) => sys::InputData {
                type_: sys::kInputText,
                data: s.as_ptr().cast(),
                size: s.len(),
            },
            Input::Image(b) => sys::InputData {
                type_: sys::kInputImage,
                data: b.as_ptr().cast(),
                size: b.len(),
            },
            Input::ImageEnd => sys::InputData {
                type_: sys::kInputImageEnd,
                data: std::ptr::null(),
                size: 0,
            },
            Input::Audio(b) => sys::InputData {
                type_: sys::kInputAudio,
                data: b.as_ptr().cast(),
                size: b.len(),
            },
            Input::AudioEnd => sys::InputData {
                type_: sys::kInputAudioEnd,
                data: std::ptr::null(),
                size: 0,
            },
        }
    }
}

/// Build the JSON `content` array for the Conversation API from a slice of
/// inputs. Text becomes `{"type":"text","text":"..."}`, images become
/// `{"type":"image"}`, audio becomes `{"type":"audio"}`.
pub(crate) fn inputs_to_content_json(inputs: &[Input<'_>]) -> String {
    let mut parts = Vec::new();
    for input in inputs {
        match input {
            Input::Text(s) => {
                parts.push(format!(
                    r#"{{"type":"text","text":{}}}"#,
                    crate::conversation::serde_json_escape(s)
                ));
            }
            Input::Image(_) => {
                parts.push(r#"{"type":"image"}"#.to_string());
            }
            Input::ImageEnd => {
                // Not represented in JSON — handled at the InputData level
            }
            Input::Audio(_) => {
                parts.push(r#"{"type":"audio"}"#.to_string());
            }
            Input::AudioEnd => {}
        }
    }
    format!("[{}]", parts.join(","))
}
