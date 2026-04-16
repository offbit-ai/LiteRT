//! Minimal CPU inference example.
//!
//! Loads the bundled `add_10x10.tflite` fixture (two `f32[10, 10]` inputs,
//! one output of the same shape computing their element-wise sum) and runs
//! it end-to-end on the CPU reference backend.
//!
//! Run with:
//!     cargo run --example add_cpu

use std::{error::Error, path::PathBuf};

use litert::{
    set_global_log_severity, CompilationOptions, CompiledModel, Environment, LogSeverity, Model,
    TensorBuffer,
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

    // Allocate buffers matching the shapes the model declares.
    let mut inputs: Vec<TensorBuffer> = (0..sig.input_count()?)
        .map(|i| {
            sig.input_shape(i)
                .and_then(|s| TensorBuffer::managed_host(&env, &s))
        })
        .collect::<Result<_, _>>()?;
    let output_shape = sig.output_shape(0)?;
    let mut outputs = vec![TensorBuffer::managed_host(&env, &output_shape)?];

    // lhs[i] = i, rhs[i] = 100 + i  →  expected output[i] = 100 + 2i
    fill(&mut inputs[0], |i| i as f32)?;
    fill(&mut inputs[1], |i| 100.0 + i as f32)?;

    let options = CompilationOptions::new()?;
    let compiled = CompiledModel::new(env, model, &options)?;
    compiled.run(&mut inputs, &mut outputs)?;

    let out = outputs[0].lock_for_read::<f32>()?;
    println!("add_10x10.tflite — CPU inference");
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
