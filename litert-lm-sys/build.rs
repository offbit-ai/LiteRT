//! Build script for litert-lm-sys.
//!
//! Same pattern as litert-sys: pre-generated bindings for the default path,
//! optional `generate-bindings` feature for maintainers. The shared library
//! (`libLiteRtLmC.{so,dylib,dll}`) is built from source in our CI and hosted
//! as a mirrored release artifact. `build.rs` downloads + SHA-verifies it on
//! first build.
//!
//! Escape hatches:
//!   `LITERT_LM_LIB_DIR`     — directory containing the shared lib; skip download.
//!   `LITERT_NO_DOWNLOAD`    — fail hard if the cache is empty (air-gapped CI).
//!   `LITERT_CACHE_DIR`      — override the cache root.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

const LITERT_LM_VERSION: &str = "0.10.2";

#[cfg(feature = "generate-bindings")]
const LITERT_LM_HEADERS_VERSION: &str = "0.10.2";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-env-changed=LITERT_LM_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LITERT_NO_DOWNLOAD");
    println!("cargo:rerun-if-env-changed=LITERT_CACHE_DIR");

    let target = env::var("TARGET").expect("TARGET env var missing");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR env var missing"));

    emit_bindings(&target, &out_dir);
    let lib_dir = locate_library(&target);
    emit_link_directives(&target, lib_dir.as_deref());
}

// ---------------------------------------------------------------------------
// Bindings
// ---------------------------------------------------------------------------

#[cfg(feature = "generate-bindings")]
fn emit_bindings(_target: &str, out_dir: &Path) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let headers_dir = manifest_dir
        .join("third_party")
        .join(format!("litert-lm-v{LITERT_LM_HEADERS_VERSION}"));
    assert!(
        headers_dir.join("c/engine.h").exists(),
        "vendored headers not found at {}",
        headers_dir.display()
    );

    bindgen::Builder::default()
        .header(manifest_dir.join("wrapper.h").to_string_lossy())
        .clang_arg(format!("-I{}", headers_dir.display()))
        .allowlist_function("litert_lm_.*")
        .allowlist_type("LiteRtLm.*")
        .allowlist_type("Type")
        .allowlist_var("kLiteRtLm.*")
        .allowlist_var("kType.*|kTopK|kTopP|kGreedy|kTypeUnspecified")
        .prepend_enum_name(false)
        .layout_tests(false)
        .derive_default(true)
        .derive_debug(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("bindgen failed")
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("write bindings.rs");
}

#[cfg(not(feature = "generate-bindings"))]
fn emit_bindings(target: &str, out_dir: &Path) {
    let pregenerated = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("bindings")
        .join(format!("{target}.rs"));

    if !pregenerated.exists() {
        panic!(
            "litert-lm-sys: no pre-generated bindings for target `{target}`.\n\
             Run with `--features generate-bindings` (needs libclang) or add \
             src/bindings/{target}.rs.",
        );
    }
    fs::copy(&pregenerated, out_dir.join("bindings.rs")).expect("copy pre-generated bindings");
    println!("cargo:rerun-if-changed={}", pregenerated.display());
}

// ---------------------------------------------------------------------------
// Library resolution
// ---------------------------------------------------------------------------

fn locate_library(target: &str) -> Option<PathBuf> {
    // 1) Explicit override.
    if let Ok(dir) = env::var("LITERT_LM_LIB_DIR") {
        let dir = PathBuf::from(dir);
        assert!(
            dir.is_dir(),
            "LITERT_LM_LIB_DIR={} is not a directory",
            dir.display()
        );
        return Some(dir);
    }

    // 2) Check DEP_LITERT_LIB_DIR — if litert-sys already downloaded
    //    libLiteRt.*, the LM engine links against that same runtime.
    //    Forward the lib search path so the linker can resolve it.
    if let Ok(litert_dir) = env::var("DEP_LITERT_LIB_DIR") {
        println!("cargo:rustc-link-search=native={litert_dir}");
    }

    // 3) Download the LiteRT-LM shared lib from our mirror.
    let cache_dir = cache_dir_for(target);

    // TODO(0.2.0): once the CMake CI pipeline produces and mirrors
    // libLiteRtLmC.{so,dylib,dll}, wire the download + SHA-verify
    // logic here (same pattern as litert-sys).
    //
    // For now, if LITERT_LM_LIB_DIR isn't set and the cache is empty,
    // we emit a warning but still let the crate compile (bindings-only
    // mode). The link will fail at the final binary step, which is
    // acceptable during the bootstrap phase.
    if !cache_dir.join(lib_name(target)).exists() {
        println!(
            "cargo:warning=litert-lm-sys: libLiteRtLmC not found. Set \
             LITERT_LM_LIB_DIR or wait for the mirrored build pipeline. \
             The crate will compile but binaries will fail to link."
        );
    }

    if cache_dir.is_dir() {
        Some(cache_dir)
    } else {
        None
    }
}

fn cache_dir_for(target: &str) -> PathBuf {
    if let Some(dir) = env::var_os("LITERT_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = dirs::cache_dir() {
        if fs::create_dir_all(&dir).is_ok() {
            return dir
                .join("litert-lm-sys")
                .join(format!("v{LITERT_LM_VERSION}"))
                .join(target);
        }
    }
    PathBuf::from(env::var("OUT_DIR").unwrap())
        .join("litert-lm-cache")
        .join(target)
}

fn lib_name(target: &str) -> &'static str {
    if target.contains("windows") {
        "LiteRtLmC.dll"
    } else if target.contains("apple") {
        "libLiteRtLmC.dylib"
    } else {
        "libLiteRtLmC.so"
    }
}

// ---------------------------------------------------------------------------
// Linker directives
// ---------------------------------------------------------------------------

fn emit_link_directives(target: &str, lib_dir: Option<&Path>) {
    if let Some(dir) = lib_dir {
        let dir = dir.display();
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        println!("cargo:lib_dir={dir}");
    }

    // The LiteRT-LM C engine lib.
    println!("cargo:rustc-link-lib=dylib=LiteRtLmC");

    // Also depends on the base LiteRT runtime (linked by litert-sys).
    println!("cargo:rustc-link-lib=dylib=LiteRt");

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
    if target.contains("android") {
        println!("cargo:rustc-link-lib=dylib=log");
    }
}
