//! Safe Rust bindings to LiteRT-LM — Google's on-device LLM inference engine.
//!
//! ```no_run
//! use litertlm::{Backend, Engine, EngineSettings, SamplerParams};
//!
//! # fn main() -> litertlm::Result<()> {
//! let engine = Engine::new(
//!     EngineSettings::new("model.litertlm")
//!         .backend(Backend::Gpu)
//!         .max_num_tokens(512),
//! )?;
//!
//! let mut session = engine.create_session(
//!     SamplerParams::default()
//!         .top_p(0.95)
//!         .temperature(0.7),
//! )?;
//!
//! let response = session.generate("Explain Rust lifetimes briefly")?;
//! println!("{response}");
//! # Ok(()) }
//! ```

#![warn(missing_docs)]

pub mod conversation;
mod engine;
mod error;
mod sampler;
mod session;

pub use conversation::Conversation;
pub use engine::{Backend, Engine, EngineSettings};
pub use error::{Error, Result};
pub use sampler::{Sampler, SamplerParams};
pub use session::Session;
