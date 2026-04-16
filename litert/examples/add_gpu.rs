//! GPU-accelerated inference example (with CPU fallback).
//!
//! Same model as `add_cpu`, but the compilation options enable both GPU and
//! CPU. On Apple Silicon this lights up the WebGPU/Metal delegate; on Linux
//! desktops with a GPU it uses the WebGPU backend; on Android it routes
//! through OpenCL/GL. If no GPU accelerator is registered for your platform,
//! LiteRT transparently falls back to the CPU reference backend.
//!
//! Run with:
//!     cargo run --example add_gpu

use std::{error::Error, path::PathBuf};

use litert::{
    set_global_log_severity, Accelerators, CompilationOptions, CompiledModel, Environment,
    LogSeverity, Model, TensorBuffer,
};

fn main() -> Result<(), Box<dyn Error>> {
    set_global_log_severity(LogSeverity::Error)?;

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

    {
        let mut g = inputs[0].lock_for_write::<f32>()?;
        for (i, v) in g.iter_mut().enumerate() {
            *v = i as f32;
        }
    }
    {
        let mut g = inputs[1].lock_for_write::<f32>()?;
        for (i, v) in g.iter_mut().enumerate() {
            *v = 100.0 + i as f32;
        }
    }

    let options =
        CompilationOptions::new()?.with_accelerators(Accelerators::GPU | Accelerators::CPU)?;
    let compiled = CompiledModel::new(env, model, &options)?;

    match compiled.is_fully_accelerated() {
        Ok(true) => println!("graph was fully placed on a GPU accelerator"),
        Ok(false) => println!("at least one op fell back to the CPU reference backend"),
        Err(e) => eprintln!("could not query acceleration status: {e}"),
    }

    compiled.run(&mut inputs, &mut outputs)?;

    let out = outputs[0].lock_for_read::<f32>()?;
    println!("first 5 outputs: {:?}", &out[..5]);
    println!("last 5 outputs:  {:?}", &out[out.len() - 5..]);
    Ok(())
}
