//! Error type for the `litert` crate.

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

    /// A requested feature is not available in the current `libLiteRt` build.
    ///
    /// The wrapped `&'static str` names the missing capability (e.g.,
    /// `"logger-control symbols"`).
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
}

/// Converts a raw `LiteRtStatus` into `Result<()>`.
///
/// Formats the error message from an in-crate lookup table rather than the
/// runtime's `LiteRtGetStatusString`, because that symbol isn't exported on
/// every target. Values that fall outside the known set get a generic label.
pub(crate) fn check(status: LiteRtStatus) -> Result<()> {
    if status == sys::kLiteRtStatusOk {
        return Ok(());
    }
    Err(Error::Status {
        code: status,
        message: status_message(status).to_string(),
    })
}

fn status_message(status: LiteRtStatus) -> &'static str {
    match status {
        sys::kLiteRtStatusErrorInvalidArgument => "invalid argument",
        sys::kLiteRtStatusErrorMemoryAllocationFailure => "memory allocation failure",
        sys::kLiteRtStatusErrorRuntimeFailure => "runtime failure",
        sys::kLiteRtStatusErrorMissingInputTensor => "missing input tensor",
        sys::kLiteRtStatusErrorUnsupported => "unsupported",
        sys::kLiteRtStatusErrorNotFound => "not found",
        sys::kLiteRtStatusErrorTimeoutExpired => "timeout expired",
        sys::kLiteRtStatusErrorWrongVersion => "wrong version",
        sys::kLiteRtStatusErrorUnknown => "unknown error",
        sys::kLiteRtStatusErrorAlreadyExists => "already exists",
        sys::kLiteRtStatusCancelled => "cancelled",
        sys::kLiteRtStatusErrorFileIO => "file I/O error",
        sys::kLiteRtStatusErrorInvalidFlatbuffer => "invalid flatbuffer",
        sys::kLiteRtStatusErrorDynamicLoading => "dynamic loading error",
        sys::kLiteRtStatusErrorSerialization => "serialization error",
        sys::kLiteRtStatusErrorCompilation => "compilation error",
        sys::kLiteRtStatusErrorIndexOOB => "index out of bounds",
        sys::kLiteRtStatusErrorInvalidIrType => "invalid IR type",
        sys::kLiteRtStatusErrorInvalidGraphInvariant => "invalid graph invariant",
        sys::kLiteRtStatusErrorGraphModification => "graph modification error",
        sys::kLiteRtStatusErrorInvalidToolConfig => "invalid tool config",
        sys::kLiteRtStatusLegalizeNoMatch => "legalization: no match",
        sys::kLiteRtStatusErrorInvalidLegalization => "invalid legalization",
        sys::kLiteRtStatusPatternNoMatch => "pattern: no match",
        sys::kLiteRtStatusInvalidTransformation => "invalid transformation",
        sys::kLiteRtStatusErrorUnsupportedRuntimeVersion => "unsupported runtime version",
        sys::kLiteRtStatusErrorUnsupportedCompilerVersion => "unsupported compiler version",
        sys::kLiteRtStatusErrorIncompatibleByteCodeVersion => "incompatible bytecode version",
        sys::kLiteRtStatusErrorUnsupportedOpShapeInferer => "unsupported op shape inferer",
        sys::kLiteRtStatusErrorShapeInferenceFailed => "shape inference failed",
        _ => "unrecognized status code",
    }
}
