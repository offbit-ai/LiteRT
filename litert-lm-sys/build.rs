//! Build script for litert-lm-sys.
//!
//! Downloads the pinned `libLiteRtLmC.{so,dylib}` shared library from our
//! mirrored GitHub release, SHA-256-verifies it, caches it, and emits the
//! linker directives. Same pattern as litert-sys.
//!
//! Escape hatches:
//!   `LITERT_LM_LIB_DIR`     — directory containing the shared lib; skip download.
//!   `LITERT_NO_DOWNLOAD`    — fail hard if the cache is empty.
//!   `LITERT_CACHE_DIR`      — override the cache root.

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

const LITERT_LM_VERSION: &str = "0.10.2";

#[cfg(feature = "generate-bindings")]
const LITERT_LM_HEADERS_VERSION: &str = "0.10.2";

const MIRROR_BASE: &str = "https://github.com/offbit-ai/LiteRT/releases/download/litert-lm-v0.10.2";

struct Prebuilt {
    url_filename: &'static str,
    local_name: &'static str,
    sha256: &'static str,
    size: u64,
}

fn prebuilt_for(target: &str) -> Option<Prebuilt> {
    Some(match target {
        "x86_64-unknown-linux-gnu" | "aarch64-unknown-linux-gnu" => Prebuilt {
            url_filename: "libLiteRtLmC.so",
            local_name: "libLiteRtLmC.so",
            sha256: "82a524d6361d15f3b5808549e1d8cf757132cd58282986a240456b7d9f989bc1",
            size: 45_413_552,
        },
        "aarch64-apple-darwin" => Prebuilt {
            url_filename: "libLiteRtLmC.dylib",
            local_name: "libLiteRtLmC.dylib",
            sha256: "616c71d3f52d7b6e7847cba2a3876890aeac993e27ec147dbf1c7de4fd786456",
            size: 28_862_192,
        },
        _ => return None,
    })
}

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
        .allowlist_var("kInput.*")
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

    // 2) Forward litert-sys lib dir so the linker can also find libLiteRt +
    //    its accelerator/plugin dylibs (libGemmaModelConstraintProvider etc.)
    //    which libLiteRtLmC depends on at runtime.
    if let Ok(litert_dir) = env::var("DEP_LITERT_LIB_DIR") {
        println!("cargo:rustc-link-search=native={litert_dir}");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{litert_dir}");
    }

    // 3) Download the mirrored shared lib.
    let pb = match prebuilt_for(target) {
        Some(p) => p,
        None => {
            println!(
                "cargo:warning=litert-lm-sys: no prebuilt for target `{target}`. \
                 Set LITERT_LM_LIB_DIR to a directory containing {lib}.",
                lib = if target.contains("windows") {
                    "LiteRtLmC.dll"
                } else {
                    "libLiteRtLmC.*"
                }
            );
            return None;
        }
    };

    let cache_dir = cache_dir_for(target);
    ensure_prebuilt(&pb, &cache_dir);
    Some(cache_dir)
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

fn ensure_prebuilt(pb: &Prebuilt, cache_dir: &Path) {
    fs::create_dir_all(cache_dir).expect("create cache dir");

    let dest = cache_dir.join(pb.local_name);
    let marker = cache_dir.join(format!("{}.verified", pb.local_name));
    if dest.exists() && marker.exists() {
        return;
    }

    if env::var_os("LITERT_NO_DOWNLOAD").is_some() {
        panic!(
            "litert-lm-sys: LITERT_NO_DOWNLOAD set but {} missing from {}",
            pb.local_name,
            cache_dir.display()
        );
    }

    let url = format!("{MIRROR_BASE}/{}", pb.url_filename);
    println!(
        "cargo:warning=litert-lm-sys: downloading {} ({} bytes) from mirror (first build only)",
        pb.local_name, pb.size,
    );

    let mut buf = Vec::with_capacity(pb.size as usize);
    ureq::get(&url)
        .call()
        .unwrap_or_else(|e| panic!("GET {url}: {e}"))
        .into_reader()
        .read_to_end(&mut buf)
        .unwrap_or_else(|e| panic!("read {}: {e}", pb.local_name));

    if buf.len() as u64 != pb.size {
        panic!(
            "litert-lm-sys: size mismatch for {}: expected {}, got {}",
            pb.local_name,
            pb.size,
            buf.len()
        );
    }

    let hash = hex(&Sha256::digest(&buf));
    if hash != pb.sha256 {
        panic!(
            "litert-lm-sys: SHA-256 mismatch for {}: expected {}, got {hash}",
            pb.local_name, pb.sha256
        );
    }

    fs::write(&dest, &buf).unwrap_or_else(|e| panic!("write {}: {e}", dest.display()));

    // The Bazel-built dylib ships with install_name=bazel-out/.../libLiteRtLmC.*
    // which is a relative path the macOS loader can't resolve. Rewrite to
    // @rpath/ so our -rpath emission works. This is safe because @rpath/ is
    // shorter than the Bazel path, so install_name_tool fits in the existing
    // header padding.
    if dest.extension().is_some_and(|e| e == "dylib") {
        let _ = std::process::Command::new("install_name_tool")
            .args(["-id", &format!("@rpath/{}", pb.local_name)])
            .arg(&dest)
            .status();
    }

    fs::write(&marker, pb.sha256).expect("write verified marker");
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        write!(&mut s, "{b:02x}").unwrap();
    }
    s
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

    println!("cargo:rustc-link-lib=dylib=LiteRtLmC");

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
    if target.contains("android") {
        println!("cargo:rustc-link-lib=dylib=log");
    }
}
