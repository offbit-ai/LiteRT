//! Compiled models — the executable form of a loaded [`Model`].

use std::ptr::NonNull;

use litert_sys as sys;

use crate::{check, CompilationOptions, Environment, Error, Model, Result, TensorBuffer};

/// Index used to pick a signature (named entry point) when a model defines
/// multiple graphs. The default signature, present in every LiteRT model, is
/// [`SignatureIndex::DEFAULT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SignatureIndex(pub sys::LiteRtParamIndex);

impl SignatureIndex {
    /// The primary signature. Present in every model.
    pub const DEFAULT: Self = Self(0);
}

impl Default for SignatureIndex {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A model compiled for a specific environment and backend.
///
/// Construct one with [`CompiledModel::new`], then [`Self::run`] it against
/// input and output [`TensorBuffer`]s.
pub struct CompiledModel {
    ptr: NonNull<sys::LiteRtCompiledModelT>,
    // Keep the environment and model alive while the compiled model exists.
    _env: Environment,
    _model: Model,
}

impl std::fmt::Debug for CompiledModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledModel")
            .field("ptr", &self.ptr.as_ptr())
            .finish()
    }
}

impl CompiledModel {
    /// Compiles `model` against `env` using `options`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if compilation fails
    /// — commonly because the requested accelerator is unavailable, or
    /// because the model uses an op the selected backend doesn't support.
    pub fn new(env: Environment, model: Model, options: &CompilationOptions) -> Result<Self> {
        let mut raw: sys::LiteRtCompiledModel = std::ptr::null_mut();
        check(unsafe {
            sys::LiteRtCreateCompiledModel(env.as_raw(), model.as_raw(), options.as_raw(), &mut raw)
        })?;
        let ptr = NonNull::new(raw).ok_or(Error::NullPointer)?;
        Ok(Self {
            ptr,
            _env: env,
            _model: model,
        })
    }

    /// `true` if the runtime was able to place every op on an accelerator.
    /// When `false`, at least one op fell back to the CPU reference backend.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if the query fails.
    pub fn is_fully_accelerated(&self) -> Result<bool> {
        let mut out: bool = false;
        check(unsafe { sys::LiteRtCompiledModelIsFullyAccelerated(self.ptr.as_ptr(), &mut out) })?;
        Ok(out)
    }

    /// Executes the default signature synchronously. `inputs` and `outputs`
    /// must already be populated / sized by the caller.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if LiteRT rejects the
    /// buffers or encounters a runtime failure.
    pub fn run(&self, inputs: &mut [TensorBuffer], outputs: &mut [TensorBuffer]) -> Result<()> {
        self.run_signature(SignatureIndex::DEFAULT, inputs, outputs)
    }

    /// Executes a specific signature. Use [`Self::run`] for the common case.
    ///
    /// # Errors
    ///
    /// See [`Self::run`].
    pub fn run_signature(
        &self,
        signature: SignatureIndex,
        inputs: &mut [TensorBuffer],
        outputs: &mut [TensorBuffer],
    ) -> Result<()> {
        // Collect raw pointers without consuming ownership; we borrow through
        // `&mut [TensorBuffer]` so the caller retains the buffers.
        let mut in_raw: Vec<sys::LiteRtTensorBuffer> =
            inputs.iter().map(TensorBuffer::as_raw).collect();
        let mut out_raw: Vec<sys::LiteRtTensorBuffer> =
            outputs.iter().map(TensorBuffer::as_raw).collect();

        check(unsafe {
            sys::LiteRtRunCompiledModel(
                self.ptr.as_ptr(),
                signature.0,
                in_raw.len(),
                in_raw.as_mut_ptr(),
                out_raw.len(),
                out_raw.as_mut_ptr(),
            )
        })
    }
}

impl Drop for CompiledModel {
    fn drop(&mut self) {
        unsafe { sys::LiteRtDestroyCompiledModel(self.ptr.as_ptr()) }
    }
}

// Safety: compiled model + its environment are thread-safe for concurrent
// reads; running requires `&self` but LiteRT serialises internally.
unsafe impl Send for CompiledModel {}
