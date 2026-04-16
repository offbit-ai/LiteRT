//! Real-world image classification end-to-end.
//!
//! Downloads Google's MobileNet v1 (0.25 depth, 224x224, INT8-quantized)
//! from the public TensorFlow CDN on first run and caches it under
//! `$TMPDIR/litert-examples/`. Subsequent runs reuse the cache.
//!
//! The input is a deterministic synthetic pattern rather than a real photo
//! — keeping the example free of image-decoding dependencies. The goal is
//! to show the shape of a "real model" pipeline: dynamic shape introspection,
//! UINT8 I/O, top-k extraction.
//!
//! Run with:
//!     cargo run --example image_classification --release

use std::{error::Error, fs, io::Read, path::PathBuf};

use litert::{
    set_global_log_severity, CompilationOptions, CompiledModel, ElementType, Environment,
    LogSeverity, Model, TensorBuffer,
};

const ARCHIVE_URL: &str = "https://storage.googleapis.com/download.tensorflow.org/\
                           models/mobilenet_v1_2018_08_02/mobilenet_v1_0.25_224_quant.tgz";
const MODEL_NAME: &str = "mobilenet_v1_0.25_224_quant.tflite";

fn main() -> Result<(), Box<dyn Error>> {
    let _ = set_global_log_severity(LogSeverity::Error);

    let model_path = ensure_model()?;

    let env = Environment::new()?;
    let model = Model::from_file(&model_path)?;
    let sig = model.signature(0)?;

    // Allocate I/O buffers matching the model's declared shapes.
    let mut inputs: Vec<TensorBuffer> = (0..sig.input_count()?)
        .map(|i| {
            let shape = sig.input_shape(i)?;
            TensorBuffer::managed_host(&env, &shape)
        })
        .collect::<Result<_, _>>()?;
    let mut outputs: Vec<TensorBuffer> = (0..sig.output_count()?)
        .map(|i| {
            let shape = sig.output_shape(i)?;
            TensorBuffer::managed_host(&env, &shape)
        })
        .collect::<Result<_, _>>()?;

    // Confirm we got what we expected before filling in anything.
    let in_shape = sig.input_shape(0)?;
    let out_shape = sig.output_shape(0)?;
    println!(
        "model: {}\n  input  [{}] {:?} {:?}\n  output [{}] {:?} {:?}",
        model_path.file_name().unwrap().to_string_lossy(),
        0,
        in_shape.dims,
        in_shape.element_type,
        0,
        out_shape.dims,
        out_shape.element_type,
    );
    assert!(matches!(in_shape.element_type, ElementType::UInt8));

    // Write a deterministic synthetic pattern into the input. Three channels
    // (R, G, B) × 224 × 224 with a slow gradient so predictions don't get
    // clamped to one class by degenerate stats.
    fill_synthetic(&mut inputs[0])?;

    let options = CompilationOptions::new()?;
    let compiled = CompiledModel::new(env, model, &options)?;
    compiled.run(&mut inputs, &mut outputs)?;

    // Output is a 1001-element UINT8 vector of quantized class probabilities.
    // Largest value ≈ most confident class. Pull the top 5.
    let out = outputs[0].lock_for_read::<u8>()?;
    let mut scored: Vec<(usize, u8)> = out.iter().copied().enumerate().collect();
    scored.sort_by_key(|&(_, s)| std::cmp::Reverse(s));

    println!("\ntop-5 predictions for synthetic gradient input:");
    println!("  (class indices only — labels omitted, see ImageNet-1001 for names)");
    for (rank, (idx, score)) in scored.iter().take(5).enumerate() {
        println!("  {rank}. class #{idx:4}  score {score:3}/255");
    }
    Ok(())
}

// -------------------------------------------------------------------------
// Asset handling
// -------------------------------------------------------------------------

fn ensure_model() -> Result<PathBuf, Box<dyn Error>> {
    let cache_dir = std::env::temp_dir().join("litert-examples");
    fs::create_dir_all(&cache_dir)?;

    let model_path = cache_dir.join(MODEL_NAME);
    if model_path.exists() {
        return Ok(model_path);
    }

    eprintln!("litert-examples: fetching {ARCHIVE_URL}");
    let mut archive_bytes = Vec::new();
    ureq::get(ARCHIVE_URL)
        .call()?
        .into_reader()
        .read_to_end(&mut archive_bytes)?;

    // Decompress gzip and scan the tar stream for the .tflite entry.
    let cursor = std::io::Cursor::new(archive_bytes);
    let decoder = flate2::read::GzDecoder::new(cursor);
    let mut tar = tar::Archive::new(decoder);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let name = entry.path()?.display().to_string();
        if name.ends_with(MODEL_NAME) {
            let mut out = fs::File::create(&model_path)?;
            std::io::copy(&mut entry, &mut out)?;
            return Ok(model_path);
        }
    }
    Err(format!("{MODEL_NAME} not found in tarball").into())
}

// -------------------------------------------------------------------------
// Synthetic input fill
// -------------------------------------------------------------------------

fn fill_synthetic(buf: &mut TensorBuffer) -> Result<(), litert::Error> {
    let shape = buf.shape()?;
    let mut guard = buf.lock_for_write::<u8>()?;
    // Expected shape is [1, 224, 224, 3] for a quant MobileNet. Generate a
    // pseudo-colour gradient so each channel carries distinct information.
    let [_, h, w, c] = [
        shape.dims.first().copied().unwrap_or(1) as usize,
        shape.dims.get(1).copied().unwrap_or(224) as usize,
        shape.dims.get(2).copied().unwrap_or(224) as usize,
        shape.dims.get(3).copied().unwrap_or(3) as usize,
    ];
    for y in 0..h {
        for x in 0..w {
            for k in 0..c {
                let idx = ((y * w + x) * c) + k;
                if idx < guard.len() {
                    let base = match k {
                        0 => x * 255 / w.max(1),        // red: horizontal gradient
                        1 => y * 255 / h.max(1),        // green: vertical
                        _ => ((x + y) * 255) / (w + h), // blue: diagonal
                    };
                    guard[idx] = base as u8;
                }
            }
        }
    }
    Ok(())
}
