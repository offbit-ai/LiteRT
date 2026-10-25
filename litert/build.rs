//! Forwards the libLiteRt directory from `litert-sys` into this crate's
//! rustc-link-arg so downstream tests / examples resolve `@rpath/libLiteRt.*`
//! at runtime without DYLD_LIBRARY_PATH / LD_LIBRARY_PATH.
//!
//! `litert-sys` declares `links = "LiteRt"` and emits `cargo:lib_dir=...`,
//! which Cargo exposes here as `DEP_LITERT_LIB_DIR`.
//!
//! MSVC's link.exe has no rpath: on Windows the DLLs are found through PATH,
//! which Cargo extends with `litert-sys`'s link directory (see its build.rs).

fn main() {
    println!("cargo:rerun-if-env-changed=DEP_LITERT_LIB_DIR");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if let (Ok(dir), false) = (std::env::var("DEP_LITERT_LIB_DIR"), msvc) {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
}
