//! End-to-end inference on the bundled `add_10x10.tflite` fixture.
//!
//! The model takes two f32 tensors shaped `[10, 10]` and returns their
//! element-wise sum. Asserting each output element matches the sum of the
//! two corresponding inputs validates that every layer of the stack — the
//! FFI, the bindgen-generated C API, the safe wrappers, and the prebuilt
//! libLiteRt runtime — agree on layout and semantics.

use std::path::PathBuf;

use litert::{
    set_global_log_severity, CompilationOptions, CompiledModel, ElementType, Environment,
    LogSeverity, Model, TensorBuffer,
};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("add_10x10.tflite")
}

#[test]
fn add_10x10_cpu_elementwise_sum() {
    let _ = set_global_log_severity(LogSeverity::Error);
    let env = Environment::new().expect("environment");
    let model = Model::from_file(fixture_path()).expect("load tflite");

    // Sanity-check the model's declared shape through the signature API.
    let sig = model.signature(0).expect("signature 0");
    let inputs = sig.input_count().unwrap();
    let outputs = sig.output_count().unwrap();
    assert_eq!(inputs, 2, "add_10x10 should take two inputs");
    assert_eq!(outputs, 1, "add_10x10 should produce one output");

    let input_shapes: Vec<_> = (0..inputs).map(|i| sig.input_shape(i).unwrap()).collect();
    let output_shape = sig.output_shape(0).unwrap();
    for s in &input_shapes {
        assert_eq!(s.element_type, ElementType::Float32);
        assert_eq!(s.dims, vec![10, 10]);
    }
    assert_eq!(output_shape.element_type, ElementType::Float32);
    assert_eq!(output_shape.dims, vec![10, 10]);

    // Allocate host-memory buffers matching the declared shapes.
    let mut in_bufs: Vec<TensorBuffer> = input_shapes
        .iter()
        .map(|s| TensorBuffer::managed_host(&env, s).expect("alloc input"))
        .collect();
    let mut out_bufs: Vec<TensorBuffer> =
        vec![TensorBuffer::managed_host(&env, &output_shape).expect("alloc output")];

    // Deterministic inputs: lhs[i] = i as f32, rhs[i] = 100 + i as f32.
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

    // Compile for CPU and run.
    let options = CompilationOptions::new().expect("options");
    let compiled = CompiledModel::new(env, model, &options).expect("compile");
    compiled
        .run(&mut in_bufs, &mut out_bufs)
        .expect("inference");

    // Verify outputs: lhs + rhs == 100 + 2i for every element.
    let out = out_bufs[0].lock_for_read::<f32>().unwrap();
    assert_eq!(out.len(), 100);
    for (i, &v) in out.iter().enumerate() {
        let expected = 100.0 + 2.0 * (i as f32);
        assert!(
            (v - expected).abs() < 1e-6,
            "element {i}: got {v}, expected {expected}",
        );
    }
}
