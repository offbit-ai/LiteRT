//! FFI smoke test. Verifies that libLiteRt is loadable and that calls to
//! universally-exported entry points round-trip successfully.
//!
//! We avoid platform-specific symbols here (`LiteRtGetStatusString`,
//! `LiteRtCompareApiVersion`, the logger family) because the Linux + Android
//! builds of `libLiteRt` strip them via a linker script while the macOS build
//! keeps them.

#[test]
fn options_lifecycle() {
    let mut opts: litert_sys::LiteRtOptions = std::ptr::null_mut();
    let status = unsafe { litert_sys::LiteRtCreateOptions(&mut opts) };
    assert_eq!(status, litert_sys::kLiteRtStatusOk, "CreateOptions status");
    assert!(!opts.is_null(), "CreateOptions returned null handle");

    let hw_status = unsafe {
        litert_sys::LiteRtSetOptionsHardwareAccelerators(opts, litert_sys::kLiteRtHwAcceleratorCpu)
    };
    assert_eq!(hw_status, litert_sys::kLiteRtStatusOk);

    unsafe { litert_sys::LiteRtDestroyOptions(opts) };
}

#[test]
fn environment_lifecycle() {
    let mut env: litert_sys::LiteRtEnvironment = std::ptr::null_mut();
    let status = unsafe { litert_sys::LiteRtCreateEnvironment(0, std::ptr::null(), &mut env) };
    assert_eq!(
        status,
        litert_sys::kLiteRtStatusOk,
        "CreateEnvironment status"
    );
    assert!(!env.is_null(), "CreateEnvironment returned null handle");
    unsafe { litert_sys::LiteRtDestroyEnvironment(env) };
}
