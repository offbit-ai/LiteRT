//! Safe Rust bindings to LiteRT-LM — Google's on-device LLM inference engine.
//!
//! ```no_run
//! use litert_lm::{Engine, EngineSettings, Session, SamplerParams};
//!
//! # fn main() -> litert_lm::Result<()> {
//! let engine = Engine::new(
//!     EngineSettings::new("gemma2-2b-it.litertlm")
//!         .max_num_tokens(1024),
//! )?;
//!
//! let mut session = engine.create_session(
//!     SamplerParams::default()
//!         .top_k(40)
//!         .temperature(0.8),
//! )?;
//!
//! let response = session.generate("Explain Rust lifetimes briefly")?;
//! println!("{}", response);
//! # Ok(()) }
//! ```

#![warn(missing_docs)]

mod engine;
mod error;
mod sampler;
mod session;

pub use engine::{Backend, Engine, EngineSettings};
pub use error::{Error, Result};
pub use sampler::{Sampler, SamplerParams};
pub use session::Session;
