//! GPU-accelerated end-to-end inference. Mirrors `inference.rs` but compiles
//! `add_10x10.tflite` with the GPU backend enabled.
//!
//! On targets where a GPU accelerator is registered (Metal/WebGPU on macOS,
//! WebGPU on desktop Linux, OpenCL/GL on Android) this should run entirely
//! on the GPU. When no GPU accelerator is available LiteRT falls back to the
//! CPU reference backend — the test still passes but `is_fully_accelerated`
//! returns `false`. This is expected and doesn't fail the build; we just
//! verify the numerical result is correct either way.

use std::path::PathBuf;

use litert::{
    set_global_log_severity, Accelerators, CompilationOptions, CompiledModel, ElementType,
    Environment, LogSeverity, Model, TensorBuffer,
};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("add_10x10.tflite")
}

#[test]
fn add_10x10_gpu_or_cpu_fallback() {
    let _ = set_global_log_severity(LogSeverity::Error);

    let env = Environment::new().expect("environment");
    let model = Model::from_file(fixture_path()).expect("load tflite");

    let sig = model.signature(0).expect("signature 0");
    assert_eq!(sig.input_count().unwrap(), 2);
    assert_eq!(sig.output_count().unwrap(), 1);

    let input_shapes: Vec<_> = (0..2).map(|i| sig.input_shape(i).unwrap()).collect();
    let output_shape = sig.output_shape(0).unwrap();
    for s in &input_shapes {
        assert_eq!(s.element_type, ElementType::Float32);
        assert_eq!(s.dims, vec![10, 10]);
    }

    let mut in_bufs: Vec<TensorBuffer> = input_shapes
        .iter()
        .map(|s| TensorBuffer::managed_host(&env, s).expect("alloc input"))
        .collect();
    let mut out_bufs = vec![TensorBuffer::managed_host(&env, &output_shape).expect("alloc output")];

    {
        let mut g = in_bufs[0].lock_for_write::<f32>().unwrap();
        for (i, v) in g.iter_mut().enumerate() {
            *v = i as f32;
        }
    }
    {
        let mut g = in_bufs[1].lock_for_write::<f32>().unwrap();
        for (i, v) in g.iter_mut().enumerate() {
            *v = 100.0 + i as f32;
        }
    }

    // Ask for GPU but also allow CPU fallback so the test is portable across
    // runners with and without a real GPU.
    let options = CompilationOptions::new()
        .expect("options")
        .with_accelerators(Accelerators::GPU | Accelerators::CPU)
        .expect("set accelerators");

    let compiled = CompiledModel::new(env, model, &options).expect("compile");
    let accelerated = compiled.is_fully_accelerated().unwrap_or(false);
    eprintln!("add_10x10 fully accelerated on GPU: {accelerated}");

    compiled
        .run(&mut in_bufs, &mut out_bufs)
        .expect("inference");

    let out = out_bufs[0].lock_for_read::<f32>().unwrap();
    assert_eq!(out.len(), 100);
    for (i, &v) in out.iter().enumerate() {
        let expected = 100.0 + 2.0 * (i as f32);
        assert!(
            (v - expected).abs() < 1e-5,
            "element {i}: got {v}, expected {expected}",
        );
    }
}
