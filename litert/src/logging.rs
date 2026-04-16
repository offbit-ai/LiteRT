//! Controls for LiteRT's internal logging.
//!
//! LiteRT emits `INFO` / `WARNING` messages during environment initialisation
//! (accelerator discovery, XNNPACK setup, etc.). Apps embedding LiteRT often
//! want to quieten this output. Use [`set_global_log_severity`] once at
//! startup to filter messages globally.

use litert_sys as sys;

use crate::{check, Result};

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
/// in the current process, as well as messages emitted during LiteRT's static
/// initialisation in already-created environments.
///
/// Typical use: call `set_global_log_severity(LogSeverity::Error)` from
/// `main()` (or a test harness) to suppress routine INFO/WARNING chatter.
///
/// # Errors
///
/// Returns [`Error::Status`](crate::Error::Status) if the underlying call
/// fails (generally impossible — the default logger always exists).
///
/// # Example
///
/// ```no_run
/// use litert::{set_global_log_severity, LogSeverity};
///
/// // Silence everything below error level.
/// set_global_log_severity(LogSeverity::Error)?;
/// # Ok::<(), litert::Error>(())
/// ```
pub fn set_global_log_severity(severity: LogSeverity) -> Result<()> {
    let logger = unsafe { sys::LiteRtGetDefaultLogger() };
    if logger.is_null() {
        return Err(crate::Error::NullPointer);
    }
    check(unsafe { sys::LiteRtSetMinLoggerSeverity(logger, severity.to_raw()) })
}
