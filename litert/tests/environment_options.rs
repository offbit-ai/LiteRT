use litert::{Accelerators, Environment, EnvironmentOptions};

#[test]
fn cpu_registration_accepts_owned_library_paths() {
    let directory = std::env::temp_dir();
    let options = EnvironmentOptions::default()
        .with_accelerators(Accelerators::CPU)
        .with_dispatch_library_dir(&directory)
        .unwrap()
        .with_compiler_plugin_library_dir(&directory)
        .unwrap();
    let environment = Environment::with_options(options).unwrap();
    drop(directory);
    drop(environment);
}

#[test]
fn library_paths_reject_nul_bytes() {
    assert!(EnvironmentOptions::default()
        .with_dispatch_library_dir("bad\0path")
        .is_err());
    assert!(EnvironmentOptions::default()
        .with_compiler_plugin_library_dir("bad\0path")
        .is_err());
}

#[test]
fn model_inference_survives_temporary_library_path_arguments() {
    use litert::{CompilationOptions, CompiledModel, Model, TensorBuffer};
    use std::path::PathBuf;

    let environment = {
        let directory = std::env::temp_dir().join("temporary runtime path");
        let options = EnvironmentOptions::default()
            .with_accelerators(Accelerators::CPU)
            .with_dispatch_library_dir(&directory)
            .unwrap()
            .with_compiler_plugin_library_dir(&directory)
            .unwrap();
        Environment::with_options(options).unwrap()
    };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/add_10x10.tflite");
    let model = Model::from_file(path).unwrap();
    let signature = model.signature(0).unwrap();
    let mut inputs: Vec<_> = (0..2)
        .map(|index| {
            TensorBuffer::managed_host(&environment, &signature.input_shape(index).unwrap())
                .unwrap()
        })
        .collect();
    let mut output =
        TensorBuffer::managed_host(&environment, &signature.output_shape(0).unwrap()).unwrap();
    inputs[0].lock_for_write::<f32>().unwrap().fill(1.0);
    inputs[1].lock_for_write::<f32>().unwrap().fill(2.0);
    let compiled =
        CompiledModel::new(environment, model, &CompilationOptions::new().unwrap()).unwrap();
    compiled
        .run(&mut inputs, std::slice::from_mut(&mut output))
        .unwrap();
    assert!(output
        .lock_for_read::<f32>()
        .unwrap()
        .iter()
        .all(|value| *value == 3.0));
}

#[cfg(unix)]
#[test]
fn library_paths_reject_non_utf8_bytes() {
    use std::os::unix::ffi::OsStringExt;
    let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![0xff]));
    assert!(matches!(
        EnvironmentOptions::default().with_dispatch_library_dir(&path),
        Err(litert::Error::InvalidPath(_))
    ));
    assert!(matches!(
        EnvironmentOptions::default().with_compiler_plugin_library_dir(&path),
        Err(litert::Error::InvalidPath(_))
    ));
}

#[cfg(windows)]
#[test]
fn library_paths_reject_unpaired_utf16_surrogates() {
    use std::os::windows::ffi::OsStringExt;
    let path = std::path::PathBuf::from(std::ffi::OsString::from_wide(&[0xd800]));
    assert!(matches!(
        EnvironmentOptions::default().with_dispatch_library_dir(&path),
        Err(litert::Error::InvalidPath(_))
    ));
    assert!(matches!(
        EnvironmentOptions::default().with_compiler_plugin_library_dir(&path),
        Err(litert::Error::InvalidPath(_))
    ));
}
