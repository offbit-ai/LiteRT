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
