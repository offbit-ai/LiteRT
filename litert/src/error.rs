//! Error type for the `litert` crate.

use std::ffi::CStr;

use litert_sys::{self as sys, LiteRtStatus};

use crate::ElementType;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the safe LiteRT API.
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The LiteRT runtime reported a non-zero status code.
    #[error("LiteRT status {code}: {message}")]
    Status { code: LiteRtStatus, message: String },

    /// A C API call returned a null handle where a non-null pointer was
    /// required.
    #[error("null pointer returned from the LiteRT C API")]
    NullPointer,

    /// Asked for a typed view of a tensor buffer whose element type differs.
    #[error("tensor type mismatch: expected {expected:?}, got {actual:?}")]
    TypeMismatch {
        expected: ElementType,
        actual: ElementType,
    },

    /// The file path contained non-UTF-8 bytes; the C API cannot accept it.
    #[error("path contains non-UTF-8 characters: {0:?}")]
    InvalidPath(std::path::PathBuf),

    /// Tensor buffer size (in bytes) isn't a multiple of the element size.
    #[error(
        "tensor buffer size {size} is not a whole number of {type_name} elements \
         (size {element_size})"
    )]
    UnalignedBufferSize {
        size: usize,
        element_size: usize,
        type_name: &'static str,
    },

    /// An `std::io` error occurred (e.g., reading a model file).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Converts a raw `LiteRtStatus` into `Result<()>`.
///
/// Formats the error message using `LiteRtGetStatusString` so callers get a
/// meaningful description without needing to match on numeric codes.
pub(crate) fn check(status: LiteRtStatus) -> Result<()> {
    if status == sys::kLiteRtStatusOk {
        return Ok(());
    }
    let message = unsafe {
        let ptr = sys::LiteRtGetStatusString(status);
        if ptr.is_null() {
            String::from("(no description)")
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    };
    Err(Error::Status {
        code: status,
        message,
    })
}
