//! Safe Rust bindings to LiteRT 2.x — Google's on-device ML inference runtime.
//!
//! ```no_run
//! use litert::{CompilationOptions, CompiledModel, Environment, Model};
//!
//! # fn main() -> litert::Result<()> {
//! let env     = Environment::new()?;
//! let model   = Model::from_file("mobilenet_v1.tflite")?;
//! let options = CompilationOptions::new()?;
//! let compiled = CompiledModel::new(env, model, &options)?;
//! # Ok(()) }
//! ```

#![warn(missing_docs)]

mod compiled_model;
mod element_type;
mod environment;
mod error;
mod logging;
mod model;
mod options;
mod signature;
mod tensor_buffer;

pub use compiled_model::{CompiledModel, SignatureIndex};
pub use element_type::{ElementType, TensorElement};
pub use environment::Environment;
pub use error::{Error, Result};
pub use logging::{set_global_log_severity, LogSeverity};
pub use model::Model;
pub use options::{Accelerators, CompilationOptions};
pub use signature::Signature;
pub use tensor_buffer::{ReadGuard, TensorBuffer, TensorShape, WriteGuard};

pub(crate) use error::check;
