//! End-to-end LLM text generation with LiteRT-LM.
//!
//! Downloads a `.litertlm` model on first run and generates a short response.
//! Demonstrates the full Engine → Session → generate pipeline.
//!
//! Run with:
//!     cargo run --example llm_generate --release
//!
//! Use Qwen3 instead of the default Gemma:
//!     cargo run --example llm_generate --release -- --qwen
//!
//! Use a local model:
//!     LITERT_LM_MODEL=/path/to/model.litertlm cargo run --example llm_generate --release

use std::{error::Error, fs, io::Read, path::PathBuf};

use litert_lm::{Backend, Engine, EngineSettings, SamplerParams};

struct ModelInfo {
    repo: &'static str,
    file: &'static str,
    label: &'static str,
}

const GEMMA_270M: ModelInfo = ModelInfo {
    repo: "litert-community/Qwen3-0.6B",
    file: "Qwen3-0.6B.litertlm",
    label: "Qwen3 0.6B (~586 MB, public)",
};

const QWEN3_06B: ModelInfo = ModelInfo {
    repo: "litert-community/Qwen3-0.6B",
    file: "Qwen3-0.6B.litertlm",
    label: "Qwen3 0.6B (~586 MB)",
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let use_qwen = args.iter().any(|a| a == "--qwen");
    let model_info = if use_qwen { &QWEN3_06B } else { &GEMMA_270M };

    let model_path = match std::env::var("LITERT_LM_MODEL") {
        Ok(p) => PathBuf::from(p),
        Err(_) => ensure_model(model_info)?,
    };

    println!("model: {} ({})", model_path.display(), model_info.label);
    println!("loading engine...");

    let use_cpu = args.iter().any(|a| a == "--cpu");
    let backend = if use_cpu { Backend::Cpu } else { Backend::Gpu };
    println!("backend: {backend:?}");
    let cache_dir = std::env::temp_dir().join("litert-lm-cache");
    fs::create_dir_all(&cache_dir)?;
    let mut settings = EngineSettings::new(&model_path)
        .backend(backend)
        .max_num_tokens(512)
        .cache_dir(&cache_dir);
    // For text-only models on CPU: vision/audio backends must be GPU
    // (upstream quirk — CPU path fatally requires encoder sections).
    if use_cpu {
        settings = settings
            .vision_backend(Backend::Gpu)
            .audio_backend(Backend::Gpu);
    }
    let engine = Engine::new(settings)?;

    println!("creating session...");
    let mut session =
        engine.create_session(SamplerParams::default().top_k(40).temperature(0.7).seed(42))?;

    let prompt = "Explain Rust lifetimes in one sentence.";
    println!("prompt: {prompt}");
    println!("generating...\n");

    let response = session.generate(prompt)?;
    println!("{response}");

    Ok(())
}

fn ensure_model(info: &ModelInfo) -> Result<PathBuf, Box<dyn Error>> {
    let cache_dir = std::env::temp_dir().join("litert-lm-examples");
    fs::create_dir_all(&cache_dir)?;

    let model_path = cache_dir.join(info.file);
    if model_path.exists() {
        return Ok(model_path);
    }

    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        info.repo, info.file
    );
    eprintln!("litert-lm-examples: downloading {}", info.label);
    eprintln!("from: {url}");
    eprintln!("(one-time download, caching to {})", cache_dir.display());

    let mut buf = Vec::new();
    ureq::get(&url)
        .call()?
        .into_reader()
        .read_to_end(&mut buf)?;

    fs::write(&model_path, &buf)?;
    eprintln!("saved ({} MB)", buf.len() / 1_048_576);
    Ok(model_path)
}
