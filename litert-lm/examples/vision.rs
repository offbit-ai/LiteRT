//! Multimodal vision inference — send an image + text prompt to a vision LLM.
//!
//! Downloads Gemma 4 E2B (~2.4 GB, public) on first run.
//!
//! Usage:
//!     cargo run --example vision --release -- photo.jpg "What's in this image?"
//!     cargo run --example vision --release -- --cpu photo.jpg "Describe this"

use std::{error::Error, fs, io::Read, path::PathBuf};

use litertlm::{Backend, Engine, EngineSettings, Input, SamplerParams};

const MODEL_REPO: &str = "litert-community/gemma-4-E2B-it-litert-lm";
const MODEL_FILE: &str = "gemma-4-E2B-it.litertlm";

fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("TF_CPP_MIN_LOG_LEVEL", "3");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let use_cpu = args.iter().any(|a| a == "--cpu");
    let positional: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect();

    let (image_path, prompt) = match positional.len() {
        2 => (positional[0], positional[1]),
        1 => (positional[0], "Describe this image in detail."),
        _ => {
            eprintln!("usage: vision [--cpu] <image.jpg> [prompt]");
            std::process::exit(1);
        }
    };

    let image_bytes = fs::read(image_path)?;
    eprintln!("image: {} ({} KB)", image_path, image_bytes.len() / 1024);

    let model_path = match std::env::var("LITERT_LM_MODEL") {
        Ok(p) => PathBuf::from(p),
        Err(_) => ensure_model()?,
    };

    let backend = if use_cpu { Backend::Cpu } else { Backend::Gpu };
    let cache_dir = std::env::temp_dir().join("litert-lm-cache");
    fs::create_dir_all(&cache_dir)?;

    eprintln!("loading {MODEL_FILE} on {backend:?}...");
    let engine = Engine::new(
        EngineSettings::new(&model_path)
            .backend(backend)
            .vision_backend(backend)
            .max_num_tokens(512)
            .cache_dir(&cache_dir),
    )?;

    // Multimodal uses the Session API (InputData carries raw image bytes).
    // The Conversation API only accepts JSON strings — no binary data.
    let mut session =
        engine.create_session(SamplerParams::default().top_p(0.95).temperature(0.7))?;

    eprintln!("prompt: {prompt}");
    let response = session.generate_with_inputs(&[
        Input::image(&image_bytes),
        Input::ImageEnd,
        Input::text(prompt),
    ])?;
    println!("{response}");
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
    eprintln!("downloading {MODEL_FILE} (~2.4 GB, one-time)");

    let mut buf = Vec::new();
    ureq::get(&url)
        .call()?
        .into_reader()
        .read_to_end(&mut buf)?;

    fs::write(&model_path, &buf)?;
    eprintln!("cached to {}", model_path.display());
    Ok(model_path)
}
