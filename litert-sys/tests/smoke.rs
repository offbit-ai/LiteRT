//! FFI smoke test. Verifies that libLiteRt is loadable and that calling the
//! simplest possible exported function returns sensible data.

use std::ffi::CStr;

#[test]
fn status_string_for_ok_is_non_empty() {
    // LiteRtGetStatusString takes a LiteRtStatus enum value and returns a
    // static C string describing it. It allocates nothing and has no
    // side effects, making it an ideal liveness probe for the dynamic load
    // path.
    let ptr = unsafe { litert_sys::LiteRtGetStatusString(litert_sys::kLiteRtStatusOk) };
    assert!(!ptr.is_null(), "LiteRtGetStatusString returned null");
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .expect("status string is not valid UTF-8");
    assert!(!s.is_empty(), "status string is empty");
    eprintln!("kLiteRtStatusOk → {s:?}");
}

#[test]
fn api_version_compare_reflexive() {
    let v = litert_sys::LiteRtApiVersion {
        major: litert_sys::LITERT_API_VERSION_MAJOR as i32,
        minor: litert_sys::LITERT_API_VERSION_MINOR as i32,
        patch: litert_sys::LITERT_API_VERSION_PATCH as i32,
    };
    let cmp = unsafe { litert_sys::LiteRtCompareApiVersion(v, v) };
    assert_eq!(cmp, 0, "version should compare equal to itself");
}
