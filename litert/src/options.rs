//! Compilation options controlling how a model is compiled for execution.

use std::ptr::NonNull;

use litert_sys as sys;

use crate::{check, Result};

/// Hardware accelerator selection, represented as a bitset so a model can be
/// compiled to target multiple backends simultaneously.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Accelerators(sys::LiteRtHwAcceleratorSet);

impl Accelerators {
    /// No accelerator. The compiled model will fail to run.
    pub const NONE: Self = Self(sys::kLiteRtHwAcceleratorNone as _);
    /// CPU reference backend. Always available.
    pub const CPU: Self = Self(sys::kLiteRtHwAcceleratorCpu as _);
    /// GPU backend (Metal on Apple, WebGPU / OpenCL elsewhere).
    pub const GPU: Self = Self(sys::kLiteRtHwAcceleratorGpu as _);
    /// NPU backend. Available on a narrower set of platforms.
    pub const NPU: Self = Self(sys::kLiteRtHwAcceleratorNpu as _);

    /// Combine with another accelerator bit.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Raw bitset value, for passing to the C API.
    #[must_use]
    pub const fn bits(self) -> sys::LiteRtHwAcceleratorSet {
        self.0
    }

    /// `true` if any of `other`'s bits are set in `self`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0 && other.0 != 0
    }
}

impl std::ops::BitOr for Accelerators {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl Default for Accelerators {
    fn default() -> Self {
        Self::CPU
    }
}

/// Options passed to [`CompiledModel::new`](crate::CompiledModel::new).
pub struct CompilationOptions {
    ptr: NonNull<sys::LiteRtOptionsT>,
}

impl std::fmt::Debug for CompilationOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompilationOptions")
            .field("ptr", &self.ptr.as_ptr())
            .finish()
    }
}

impl CompilationOptions {
    /// Creates a new options object with default settings (CPU-only).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NullPointer`](crate::Error::NullPointer) if the C API
    /// refused to allocate, or [`Error::Status`](crate::Error::Status) if the
    /// runtime reported a failure.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use litert::{Accelerators, CompilationOptions};
    ///
    /// let options = CompilationOptions::new()?
    ///     .with_accelerators(Accelerators::GPU | Accelerators::CPU)?;
    /// # Ok::<(), litert::Error>(())
    /// ```
    pub fn new() -> Result<Self> {
        let mut raw: sys::LiteRtOptions = std::ptr::null_mut();
        check(unsafe { sys::LiteRtCreateOptions(&mut raw) })?;
        let ptr = NonNull::new(raw).ok_or(crate::Error::NullPointer)?;
        let mut this = Self { ptr };
        this.set_accelerators(Accelerators::CPU)?;
        Ok(this)
    }

    /// Selects which hardware backends the compiler may use.
    ///
    /// # Errors
    ///
    /// Returns an error if the LiteRT runtime rejects the combination.
    pub fn set_accelerators(&mut self, accelerators: Accelerators) -> Result<()> {
        check(unsafe {
            sys::LiteRtSetOptionsHardwareAccelerators(self.ptr.as_ptr(), accelerators.bits())
        })
    }

    /// Builder-style accelerator setter.
    ///
    /// # Errors
    ///
    /// See [`Self::set_accelerators`].
    pub fn with_accelerators(mut self, accelerators: Accelerators) -> Result<Self> {
        self.set_accelerators(accelerators)?;
        Ok(self)
    }

    pub(crate) fn as_raw(&self) -> sys::LiteRtOptions {
        self.ptr.as_ptr()
    }
}

impl Drop for CompilationOptions {
    fn drop(&mut self) {
        unsafe { sys::LiteRtDestroyOptions(self.ptr.as_ptr()) }
    }
}

// Safety: a LiteRtOptions handle carries no thread-local state and is only
// mutated via exclusive borrow in the safe API.
unsafe impl Send for CompilationOptions {}
