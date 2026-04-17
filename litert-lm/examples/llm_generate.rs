//! End-to-end LLM text generation with LiteRT-LM.
//!
//! Downloads Qwen3-0.6B on first run and generates a response.
//!
//! Usage:
//!     cargo run --example llm_generate --release -- "Your prompt here"
//!     cargo run --example llm_generate --release -- --stream "Your prompt"
//!     cargo run --example llm_generate --release -- --cpu "Your prompt"
//!     LITERT_LM_MODEL=/path/to/model.litertlm cargo run --example llm_generate --release

use std::{error::Error, fs, io::Read, path::PathBuf};

use litertlm::{Backend, Engine, EngineSettings, SamplerParams};

const MODEL_REPO: &str = "litert-community/Qwen3-0.6B";
const MODEL_FILE: &str = "Qwen3-0.6B.litertlm";

fn main() -> Result<(), Box<dyn Error>> {
    // Must be set before any LiteRT code runs — TFLite reads it during init.
    std::env::set_var("TF_CPP_MIN_LOG_LEVEL", "3");

    let args: Vec<String> = std::env::args().skip(1).collect();

    let use_cpu = args.iter().any(|a| a == "--cpu");
    let use_stream = args.iter().any(|a| a == "--stream");
    let prompt = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("Explain Rust lifetimes in one sentence.");

    let model_path = match std::env::var("LITERT_LM_MODEL") {
        Ok(p) => PathBuf::from(p),
        Err(_) => ensure_model()?,
    };

    let backend = if use_cpu { Backend::Cpu } else { Backend::Gpu };
    let cache_dir = std::env::temp_dir().join("litert-lm-cache");
    fs::create_dir_all(&cache_dir)?;

    let engine = Engine::new(
        EngineSettings::new(&model_path)
            .backend(backend)
            .max_num_tokens(512)
            .cache_dir(&cache_dir),
    )?;

    let sampler = SamplerParams::default()
        .top_p(0.95)
        .temperature(0.7)
        .seed(42);

    eprintln!("model: {MODEL_FILE}  backend: {backend:?}");

    if use_stream {
        // Conversation API: proper prompt template + token-by-token streaming
        let mut conv = engine.create_conversation(sampler)?;
        conv.send_message_stream(prompt, |chunk| {
            print!("{chunk}");
            use std::io::Write;
            std::io::stdout().flush().ok();
        })?;
        println!();
    } else {
        // Session API: blocking, returns full response at once
        let mut session = engine.create_session(sampler)?;
        let response = session.generate(prompt)?;
        println!("{response}");
    }
    Ok(())
}

fn ensure_model() -> Result<PathBuf, Box<dyn Error>> {
    let cache_dir = std::env::temp_dir().join("litert-lm-examples");
    fs::create_dir_all(&cache_dir)?;

    let model_path = cache_dir.join(MODEL_FILE);
    if model_path.exists() {
        return Ok(model_path);
    }

    let url = format!("https://huggingface.co/{MODEL_REPO}/resolve/main/{MODEL_FILE}");
    eprintln!("downloading {MODEL_FILE} (~586 MB, one-time)");

    let mut buf = Vec::new();
    ureq::get(&url)
        .call()?
        .into_reader()
        .read_to_end(&mut buf)?;

    fs::write(&model_path, &buf)?;
    eprintln!("cached to {}", model_path.display());
    Ok(model_path)
}
