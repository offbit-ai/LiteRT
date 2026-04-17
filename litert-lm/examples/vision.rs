//! Multimodal vision inference — send an image + text prompt to a vision LLM.
//!
//! Downloads Gemma 4 E2B (~2.4 GB, public) on first run.
//!
//! Usage:
//!     cargo run --example vision --release -- photo.jpg "What's in this image?"
//!     cargo run --example vision --release -- --cpu photo.jpg "Describe this"

use std::{error::Error, fs, io::Read, path::PathBuf};

use litertlm::{Backend, Engine, EngineSettings, SamplerParams};

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

    let image_size = fs::metadata(image_path)?.len();
    eprintln!("image: {} ({} KB)", image_path, image_size / 1024);

    let model_path = match std::env::var("LITERT_LM_MODEL") {
        Ok(p) => PathBuf::from(p),
        Err(_) => ensure_model()?,
    };

    let backend = if use_cpu { Backend::Cpu } else { Backend::Gpu };
    let cache_dir = std::env::temp_dir().join("litert-lm-cache");
    fs::create_dir_all(&cache_dir)?;

    eprintln!("loading {MODEL_FILE} (text: {backend:?}, vision: Cpu)...");
    let engine = Engine::new(
        EngineSettings::new(&model_path)
            .backend(backend)
            // Vision encoder runs on CPU to avoid ODR conflict between
            // libLiteRtLmC.dylib and libLiteRtWebGpuAccelerator.dylib
            // (both statically link absl → BlockingCounter crash on GPU).
            .vision_backend(Backend::Cpu)
            .max_num_tokens(512)
            .cache_dir(&cache_dir),
    )?;

    let mut conv =
        engine.create_conversation(SamplerParams::default().top_p(0.95).temperature(0.7))?;

    // Pass image as file path in the Conversation JSON content array.
    // The engine's vision pipeline reads and decodes the file internally.
    let message_json = format!(
        r#"{{"role":"user","content":[{{"type":"image","path":{}}},{{"type":"text","text":{}}}]}}"#,
        json_escape(image_path),
        json_escape(prompt),
    );

    eprintln!("prompt: {prompt}");
    conv.send_raw_stream(&message_json, |chunk| {
        print!("{chunk}");
        use std::io::Write;
        std::io::stdout().flush().ok();
    })?;
    println!();
    Ok(())
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
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
