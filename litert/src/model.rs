//! Loading TFLite / LiteRT model files into memory.

use std::{ffi::CString, path::Path, ptr::NonNull, sync::Arc};

use litert_sys as sys;

use crate::{check, Error, Result};

/// An immutable, reference-counted handle to a parsed LiteRT model.
///
/// Created with [`Model::from_file`] or [`Model::from_bytes`]. Cheap to clone
/// (it's wrapped in an `Arc`). The underlying C object is released when the
/// last clone is dropped.
#[derive(Clone)]
pub struct Model {
    inner: Arc<ModelInner>,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("ptr", &self.inner.ptr.as_ptr())
            .field("strong_count", &Arc::strong_count(&self.inner))
            .finish()
    }
}

struct ModelInner {
    ptr: NonNull<sys::LiteRtModelT>,
    // Kept alive when the model was built from a `&[u8]`, since
    // `LiteRtCreateModelFromBuffer` does not take ownership of the bytes.
    _owned_bytes: Option<Box<[u8]>>,
}

// Safety: `LiteRtModel` is immutable after construction and the C runtime
// synchronises its internal refcount.
unsafe impl Send for ModelInner {}
unsafe impl Sync for ModelInner {}

impl Model {
    /// Loads a model from a `.tflite` / `.litertlm` file on disk.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPath`] if the path contains non-UTF-8 bytes,
    /// or [`Error::Status`](crate::Error::Status) if LiteRT cannot parse the
    /// file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let model = litert::Model::from_file("mobilenet_v1.tflite")?;
    /// # Ok::<(), litert::Error>(())
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))?;
        let cstr = CString::new(path_str).map_err(|_| Error::InvalidPath(path.to_path_buf()))?;

        let mut raw: sys::LiteRtModel = std::ptr::null_mut();
        check(unsafe { sys::LiteRtCreateModelFromFile(cstr.as_ptr(), &mut raw) })?;
        let ptr = NonNull::new(raw).ok_or(Error::NullPointer)?;
        Ok(Self {
            inner: Arc::new(ModelInner {
                ptr,
                _owned_bytes: None,
            }),
        })
    }

    /// Loads a model from an owned byte buffer.
    ///
    /// The buffer is retained for the lifetime of the [`Model`] since the C
    /// API stores a non-owning pointer into it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if the bytes don't
    /// form a valid serialized model.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let bytes = std::fs::read("mobilenet_v1.tflite")?;
    /// let model = litert::Model::from_bytes(bytes)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_bytes(bytes: impl Into<Box<[u8]>>) -> Result<Self> {
        let bytes: Box<[u8]> = bytes.into();
        let mut raw: sys::LiteRtModel = std::ptr::null_mut();
        check(unsafe {
            sys::LiteRtCreateModelFromBuffer(bytes.as_ptr().cast(), bytes.len(), &mut raw)
        })?;
        let ptr = NonNull::new(raw).ok_or(Error::NullPointer)?;
        Ok(Self {
            inner: Arc::new(ModelInner {
                ptr,
                _owned_bytes: Some(bytes),
            }),
        })
    }

    /// Number of signatures (named entry points) defined in the model.
    ///
    /// Most standard `.tflite` models expose a single default signature.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if the call fails.
    pub fn signature_count(&self) -> Result<usize> {
        let mut count: sys::LiteRtParamIndex = 0;
        check(unsafe { sys::LiteRtGetNumModelSignatures(self.as_raw(), &mut count) })?;
        Ok(count)
    }

    /// Returns the signature at the given index. Use `0` for the default
    /// signature (present in every model).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if the index is out
    /// of range.
    pub fn signature(&self, index: usize) -> Result<crate::Signature> {
        crate::Signature::new(self.clone(), index)
    }

    pub(crate) fn as_raw(&self) -> sys::LiteRtModel {
        self.inner.ptr.as_ptr()
    }
}

impl Drop for ModelInner {
    fn drop(&mut self) {
        unsafe { sys::LiteRtDestroyModel(self.ptr.as_ptr()) }
    }
}
