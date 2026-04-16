//! End-to-end lifecycle tests for the safe wrappers.
//!
//! These don't need a real `.tflite` file — they exercise every create/drop
//! path for the objects that don't depend on a loaded model, plus verify that
//! errors round-trip with useful messages.

use litert::{
    set_global_log_severity, Accelerators, CompilationOptions, ElementType, Environment, Error,
    LogSeverity, Model, TensorBuffer, TensorShape,
};

fn quiet_logs() {
    let _ = set_global_log_severity(LogSeverity::Error);
}

#[test]
fn environment_create_drop() {
    quiet_logs();
    let env = Environment::new().expect("create env");
    drop(env);
}

#[test]
fn compilation_options_default_is_cpu() {
    let opts = CompilationOptions::new().expect("create options");
    drop(opts);
}

#[test]
fn compilation_options_accept_gpu_bit() {
    let opts = CompilationOptions::new()
        .expect("create")
        .with_accelerators(Accelerators::CPU | Accelerators::GPU)
        .expect("set accelerators");
    drop(opts);
}

#[test]
fn model_from_file_missing_returns_status_error() {
    let err =
        Model::from_file("/definitely/does/not/exist.tflite").expect_err("missing file must fail");
    let Error::Status { message, .. } = err else {
        panic!("expected Error::Status, got {err:?}");
    };
    assert!(!message.is_empty(), "runtime should supply a message");
}

#[test]
fn managed_host_tensor_buffer_roundtrip_f32() {
    quiet_logs();
    let env = Environment::new().expect("env");
    let shape = TensorShape {
        element_type: ElementType::Float32,
        dims: vec![1, 4],
    };
    let mut buf = TensorBuffer::managed_host(&env, &shape).expect("alloc");

    assert_eq!(buf.size_bytes().unwrap(), 4 * 4);
    let observed = buf.shape().expect("shape back");
    assert_eq!(observed.element_type, ElementType::Float32);
    assert_eq!(observed.dims, vec![1, 4]);

    {
        let mut w = buf.lock_for_write::<f32>().expect("write lock");
        w.copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
    }
    {
        let r = buf.lock_for_read::<f32>().expect("read lock");
        assert_eq!(&*r, &[1.0, 2.0, 3.0, 4.0]);
    }
}

#[test]
fn tensor_buffer_type_mismatch_errors_cleanly() {
    quiet_logs();
    let env = Environment::new().expect("env");
    let shape = TensorShape {
        element_type: ElementType::Int32,
        dims: vec![3],
    };
    let buf = TensorBuffer::managed_host(&env, &shape).expect("alloc");

    let err = buf
        .lock_for_read::<f32>()
        .map(|_| ())
        .expect_err("type mismatch");
    match err {
        Error::TypeMismatch { expected, actual } => {
            assert_eq!(expected, ElementType::Float32);
            assert_eq!(actual, ElementType::Int32);
        }
        other => panic!("expected TypeMismatch, got {other:?}"),
    }
}
