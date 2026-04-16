//! Forwards the libLiteRt directory from `litert-sys` into this crate's
//! rustc-link-arg so downstream tests / examples resolve `@rpath/libLiteRt.*`
//! at runtime without DYLD_LIBRARY_PATH / LD_LIBRARY_PATH.
//!
//! `litert-sys` declares `links = "LiteRt"` and emits `cargo:lib_dir=...`,
//! which Cargo exposes here as `DEP_LITERT_LIB_DIR`.

fn main() {
    println!("cargo:rerun-if-env-changed=DEP_LITERT_LIB_DIR");
    if let Ok(dir) = std::env::var("DEP_LITERT_LIB_DIR") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
}
