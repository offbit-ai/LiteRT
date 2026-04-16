//! Controls for LiteRT's internal logging.
//!
//! LiteRT emits `INFO` / `WARNING` messages during environment initialisation
//! (accelerator discovery, XNNPACK setup, etc.). Apps embedding LiteRT often
//! want to quieten this output. Use [`set_global_log_severity`] once at
//! startup to filter messages globally.
//!
//! ## Availability
//!
//! The logger-control symbols (`LiteRtGetDefaultLogger`,
//! `LiteRtSetMinLoggerSeverity`) are exported by the macOS build of
//! `libLiteRt` but **not** by the Linux, Windows, or Android Maven builds we
//! consume today. On platforms that lack these symbols,
//! [`set_global_log_severity`] returns [`Error::Unsupported`] rather than a
//! link error, and LiteRT continues to log at its default verbosity.

use std::sync::OnceLock;

use libloading::Symbol;
use litert_sys as sys;

use crate::{Error, Result};

/// Minimum severity of a log message that LiteRT will emit.
///
/// Messages below the configured threshold are dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LogSeverity {
    /// Fine-grained debugging detail.
    Debug,
    /// Verbose diagnostic output.
    Verbose,
    /// Informational messages about progress.
    Info,
    /// Recoverable problems or unusual conditions.
    Warning,
    /// Errors that impact functionality.
    Error,
    /// Silence everything.
    Silent,
}

impl LogSeverity {
    fn to_raw(self) -> sys::LiteRtLogSeverity {
        match self {
            Self::Debug => sys::kLiteRtLogSeverityDebug,
            Self::Verbose => sys::kLiteRtLogSeverityVerbose,
            Self::Info => sys::kLiteRtLogSeverityInfo,
            Self::Warning => sys::kLiteRtLogSeverityWarning,
            Self::Error => sys::kLiteRtLogSeverityError,
            Self::Silent => sys::kLiteRtLogSeveritySilent,
        }
    }
}

/// Sets the minimum severity of LiteRT's default logger.
///
/// Affects every [`Environment`](crate::Environment) created after this call
/// in the current process. Returns [`Error::Unsupported`] on platforms whose
/// `libLiteRt` build does not export the logger-control symbols — in that
/// case the runtime's default log level stays in place.
///
/// Typical use: call `set_global_log_severity(LogSeverity::Error)` from
/// `main()` (or a test harness) to suppress routine INFO/WARNING chatter.
///
/// # Errors
///
/// - [`Error::Unsupported`] if the running `libLiteRt` doesn't export the
///   logger-control entry points.
/// - [`Error::Status`](crate::Error::Status) if the runtime rejects the call
///   (generally impossible — the default logger always exists).
///
/// # Example
///
/// ```no_run
/// use litert::{set_global_log_severity, LogSeverity};
///
/// // Silence everything below error level (no-op on platforms without
/// // logger-control symbols).
/// let _ = set_global_log_severity(LogSeverity::Error);
/// ```
pub fn set_global_log_severity(severity: LogSeverity) -> Result<()> {
    let hooks = logger_hooks().ok_or(Error::Unsupported("logger-control symbols"))?;
    let logger = unsafe { (hooks.get_default)() };
    if logger.is_null() {
        return Err(Error::NullPointer);
    }
    crate::check(unsafe { (hooks.set_min_severity)(logger, severity.to_raw()) })
}

// --------------------------------------------------------------------------
// Runtime symbol resolution
// --------------------------------------------------------------------------

/// Function pointers resolved at runtime from `libLiteRt`.
struct LoggerHooks {
    get_default: unsafe extern "C" fn() -> sys::LiteRtLogger,
    set_min_severity:
        unsafe extern "C" fn(sys::LiteRtLogger, sys::LiteRtLogSeverity) -> sys::LiteRtStatus,
}

// Safety: the resolved function pointers are immutable and thread-safe to
// call; libloading-owned Library handles are Sync on platforms we target.
unsafe impl Sync for LoggerHooks {}
unsafe impl Send for LoggerHooks {}

fn logger_hooks() -> Option<&'static LoggerHooks> {
    static HOOKS: OnceLock<Option<LoggerHooks>> = OnceLock::new();
    HOOKS.get_or_init(resolve_logger_hooks).as_ref()
}

fn resolve_logger_hooks() -> Option<LoggerHooks> {
    // RTLD_DEFAULT-style lookup: libLiteRt is already loaded because the
    // process is linking against it. We just dlsym its exported symbols
    // through a Library handle obtained from the already-loaded image.
    //
    // On non-Windows POSIX this is spelt `Library::this()` — an empty-name
    // load that resolves through the default search order. On Windows we
    // load the module by filename.
    #[cfg(unix)]
    let lib: libloading::Library = libloading::os::unix::Library::this().into();
    #[cfg(windows)]
    let lib = unsafe { libloading::Library::new("LiteRt.dll") }.ok()?;

    unsafe {
        let get_default: Symbol<unsafe extern "C" fn() -> sys::LiteRtLogger> =
            lib_get(&lib, b"LiteRtGetDefaultLogger\0")?;
        let set_min_severity: Symbol<
            unsafe extern "C" fn(sys::LiteRtLogger, sys::LiteRtLogSeverity) -> sys::LiteRtStatus,
        > = lib_get(&lib, b"LiteRtSetMinLoggerSeverity\0")?;
        Some(LoggerHooks {
            get_default: *get_default.into_raw(),
            set_min_severity: *set_min_severity.into_raw(),
        })
    }
}

unsafe fn lib_get<'a, T>(lib: &'a libloading::Library, name: &[u8]) -> Option<Symbol<'a, T>> {
    lib.get(name).ok()
}
