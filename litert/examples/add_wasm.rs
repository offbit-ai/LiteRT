//! WASM CPU inference example.
//!
//! Same model + computation as `add_cpu.rs`, but compiled to
//! `wasm32-unknown-emscripten` so it can run under Node.js, wasmtime, or in a
//! browser via the emscripten JS shim.
//!
//! The model is read from `add_10x10.tflite` in the host filesystem (when run
//! under Node/wasmtime) or via emscripten's preload-file (when run in a
//! browser bundle).
//!
//! ## Build
//!
//! ```bash
//! source $EMSDK/emsdk_env.sh
//! # NODERAWFS=1 lets the WASM module read host files (model.tflite) under
//! # node/wasmtime. ALLOW_MEMORY_GROWTH=1 lets the heap grow past the default
//! # 16 MB so larger models load.
//! LITERT_LIB_DIR=/path/to/wasm/static/archives \
//!   RUSTFLAGS="-C link-arg=-sNODERAWFS=1 -C link-arg=-sALLOW_MEMORY_GROWTH=1" \
//!   cargo build --target wasm32-unknown-emscripten \
//!                --example add_wasm --release
//! ```
//!
//! ## Run (Node.js)
//!
//! ```bash
//! node target/wasm32-unknown-emscripten/release/examples/add_wasm.js
//! ```
//!
//! Verified end-to-end: model loads, inference runs, outputs are correct
//! (sum of element-wise inputs).
//!
//! ## Browser
//!
//! Drop NODERAWFS (no host fs in browser) and instead either:
//! - Embed the model with `--preload-file model.tflite` (bundled into the
//!   `.wasm` data section).
//! - Fetch via JS and write to MEMFS before calling `Model::from_file`.
//! - Or use `Model::from_bytes` with a `Vec<u8>` you `fetch()`'d.
//!
//! Bundle the produced `.wasm` + `.js` glue alongside a small HTML loader.

use std::{error::Error, path::PathBuf};

use litert::{CompilationOptions, CompiledModel, Environment, Model, TensorBuffer};

fn main() -> Result<(), Box<dyn Error>> {
    // Note: set_global_log_severity returns Error::Unsupported on WASM.
    // libLiteRt.a doesn't export the logger-control symbols. Skip it.

    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("add_10x10.tflite");

    let env = Environment::new()?;
    let model = Model::from_file(&model_path)?;
    let sig = model.signature(0)?;

    let mut inputs: Vec<TensorBuffer> = (0..sig.input_count()?)
        .map(|i| {
            sig.input_shape(i)
                .and_then(|s| TensorBuffer::managed_host(&env, &s))
        })
        .collect::<Result<_, _>>()?;
    let output_shape = sig.output_shape(0)?;
    let mut outputs = vec![TensorBuffer::managed_host(&env, &output_shape)?];

    fill(&mut inputs[0], |i| i as f32)?;
    fill(&mut inputs[1], |i| 100.0 + i as f32)?;

    let options = CompilationOptions::new()?;
    let compiled = CompiledModel::new(env, model, &options)?;
    compiled.run(&mut inputs, &mut outputs)?;

    let out = outputs[0].lock_for_read::<f32>()?;
    println!("add_10x10.tflite — WASM CPU inference");
    println!("first 5 outputs: {:?}", &out[..5]);
    println!("last 5 outputs:  {:?}", &out[out.len() - 5..]);
    Ok(())
}

fn fill(buf: &mut TensorBuffer, f: impl Fn(usize) -> f32) -> Result<(), litert::Error> {
    let mut guard = buf.lock_for_write::<f32>()?;
    for (i, v) in guard.iter_mut().enumerate() {
        *v = f(i);
    }
    Ok(())
}
