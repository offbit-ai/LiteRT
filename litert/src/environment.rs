//! LiteRT execution environment — the hardware context shared across models.

use std::{ffi::CString, path::Path, ptr::NonNull};

use litert_sys as sys;

use crate::{check, Accelerators, Error, Result};

/// Accelerator registration and vendor library locations for a runtime environment.
/// Paths are owned so the native runtime may safely retain their pointers.
#[derive(Debug, Default)]
pub struct EnvironmentOptions {
    accelerators: Option<Accelerators>,
    dispatch: Option<CString>,
    compiler: Option<CString>,
}

impl EnvironmentOptions {
    /// Selects which accelerators the environment should register.
    pub fn with_accelerators(mut self, accelerators: Accelerators) -> Self {
        self.accelerators = Some(accelerators);
        self
    }

    /// Sets the directory containing the vendor's Dispatch library.
    /// Returns an error for non-UTF-8 paths or embedded NUL bytes.
    pub fn with_dispatch_library_dir(mut self, path: impl AsRef<Path>) -> Result<Self> {
        self.dispatch = Some(library_path(path.as_ref())?);
        Ok(self)
    }

    /// Sets the directory containing the vendor's compiler plugin.
    /// Returns an error for non-UTF-8 paths or embedded NUL bytes.
    pub fn with_compiler_plugin_library_dir(mut self, path: impl AsRef<Path>) -> Result<Self> {
        self.compiler = Some(library_path(path.as_ref())?);
        Ok(self)
    }

    fn raw(&self) -> Vec<sys::LiteRtEnvOption> {
        let mut options = Vec::new();
        if let Some(accelerators) = self.accelerators {
            options.push(sys::LiteRtEnvOption {
                tag: sys::kLiteRtEnvOptionTagAutoRegisterAccelerators,
                value: sys::LiteRtAny {
                    type_: sys::kLiteRtAnyTypeInt,
                    __bindgen_anon_1: sys::LiteRtAny__bindgen_ty_1 {
                        int_value: accelerators.bits() as i64,
                    },
                },
            });
        }
        for (tag, path) in [
            (sys::kLiteRtEnvOptionTagDispatchLibraryDir, &self.dispatch),
            (
                sys::kLiteRtEnvOptionTagCompilerPluginLibraryDir,
                &self.compiler,
            ),
        ] {
            if let Some(path) = path {
                options.push(sys::LiteRtEnvOption {
                    tag,
                    value: sys::LiteRtAny {
                        type_: sys::kLiteRtAnyTypeString,
                        __bindgen_anon_1: sys::LiteRtAny__bindgen_ty_1 {
                            str_value: path.as_ptr(),
                        },
                    },
                });
            }
        }
        options
    }
}

fn library_path(path: &Path) -> Result<CString> {
    path.to_str()
        .and_then(|path| CString::new(path).ok())
        .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))
}

/// Holds the hardware context (device handles, caches, delegates).
///
/// One [`Environment`] is typically created at application startup and shared
/// across every [`CompiledModel`](crate::CompiledModel). It is internally
/// reference-counted by LiteRT; cloning the [`std::sync::Arc`] is cheap.
pub struct Environment {
    ptr: NonNull<sys::LiteRtEnvironmentT>,
    _options: EnvironmentOptions,
}

impl std::fmt::Debug for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Environment")
            .field("ptr", &self.ptr.as_ptr())
            .finish()
    }
}

impl Environment {
    /// Creates a new environment with no extra options. Enough for CPU and
    /// the default GPU/Metal backends on desktop.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if LiteRT reports an
    /// initialisation failure.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let env = litert::Environment::new()?;
    /// # Ok::<(), litert::Error>(())
    /// ```
    pub fn new() -> Result<Self> {
        Self::with_options(EnvironmentOptions::default())
    }

    /// Creates an environment with explicit accelerator registration and vendor library paths.
    /// Returns a runtime status error or a null-handle error if initialization fails.
    pub fn with_options(options: EnvironmentOptions) -> Result<Self> {
        let raw_options = options.raw();
        let mut raw: sys::LiteRtEnvironment = std::ptr::null_mut();
        // At most three options are built above.
        check(unsafe {
            sys::LiteRtCreateEnvironment(raw_options.len() as i32, raw_options.as_ptr(), &mut raw)
        })?;
        let ptr = NonNull::new(raw).ok_or(crate::Error::NullPointer)?;
        Ok(Self {
            ptr,
            _options: options,
        })
    }

    pub(crate) fn as_raw(&self) -> sys::LiteRtEnvironment {
        self.ptr.as_ptr()
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        unsafe { sys::LiteRtDestroyEnvironment(self.ptr.as_ptr()) }
    }
}

// Safety: the environment is an opaque refcounted context safe for concurrent
// read access across threads. Destructive operations (`Drop`) require unique
// ownership, which Rust's borrow checker enforces.
unsafe impl Send for Environment {}
unsafe impl Sync for Environment {}
