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

enum Prebuilt {
    /// Desktop targets: a single dynamic library downloaded from the mirror.
    Dylib {
        url_filename: &'static str,
        local_name: &'static str,
        sha256: &'static str,
        size: u64,
    },
    /// WASM target: a tar.gz of static archives produced by the
    /// `build-litertlm-wasm.yml` GitHub Actions workflow (CMake+emscripten
    /// build of LiteRT-LM v0.10.2 + our `wasm-patches/litert-lm-v0.10.2/`
    /// CMake patches). Contains libLiteRtLmC.a plus its full transitive
    /// closure (TFLite, abseil, sentencepiece, tokenizers-cpp, antlr4,
    /// llguidance Rust glue, libpng, kissfft, zlib, minizip, minja).
    /// build.rs globs lib*.a from the cache and emits -lstatic for each.
    WasmTarball {
        url: &'static str,
        sha256: &'static str,
        size: u64,
    },
}

fn prebuilt_for(target: &str) -> Option<Prebuilt> {
    Some(match target {
        "x86_64-unknown-linux-gnu" | "aarch64-unknown-linux-gnu" => Prebuilt::Dylib {
            url_filename: "libLiteRtLmC.so",
            local_name: "libLiteRtLmC.so",
            sha256: "82a524d6361d15f3b5808549e1d8cf757132cd58282986a240456b7d9f989bc1",
            size: 45_413_552,
        },
        "aarch64-apple-darwin" => Prebuilt::Dylib {
            url_filename: "libLiteRtLmC.dylib",
            local_name: "libLiteRtLmC.dylib",
            sha256: "616c71d3f52d7b6e7847cba2a3876890aeac993e27ec147dbf1c7de4fd786456",
            size: 28_862_192,
        },
        "wasm32-unknown-emscripten" => Prebuilt::WasmTarball {
            url: "https://github.com/offbit-ai/LiteRT/releases/download/\
                  litert-lm-wasm-v0.10.2/libLiteRtLmC-wasm32-emscripten.tar.gz",
            // TODO(0.4.0-rc): replace placeholder after `build-litertlm-wasm.yml`
            // uploads the artifact and we record the SHA-256.
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
            size: 0,
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
    match pb {
        Prebuilt::Dylib {
            url_filename,
            local_name,
            sha256,
            size,
        } => ensure_dylib(url_filename, local_name, sha256, *size, cache_dir),
        Prebuilt::WasmTarball { url, sha256, size } => {
            ensure_wasm_tarball(url, sha256, *size, cache_dir)
        }
    }
}

fn ensure_dylib(
    url_filename: &str,
    local_name: &str,
    sha256: &str,
    size: u64,
    cache_dir: &Path,
) {
    let dest = cache_dir.join(local_name);
    let marker = cache_dir.join(format!("{local_name}.verified"));
    if dest.exists() && marker.exists() {
        return;
    }

    if env::var_os("LITERT_NO_DOWNLOAD").is_some() {
        panic!(
            "litert-lm-sys: LITERT_NO_DOWNLOAD set but {local_name} missing from {}",
            cache_dir.display()
        );
    }

    let url = format!("{MIRROR_BASE}/{url_filename}");
    println!(
        "cargo:warning=litert-lm-sys: downloading {local_name} ({size} bytes) from mirror (first build only)",
    );

    let mut buf = Vec::with_capacity(size as usize);
    ureq::get(&url)
        .call()
        .unwrap_or_else(|e| panic!("GET {url}: {e}"))
        .into_reader()
        .read_to_end(&mut buf)
        .unwrap_or_else(|e| panic!("read {local_name}: {e}"));

    if buf.len() as u64 != size {
        panic!(
            "litert-lm-sys: size mismatch for {local_name}: expected {size}, got {}",
            buf.len()
        );
    }

    let hash = hex(&Sha256::digest(&buf));
    if hash != sha256 {
        panic!("litert-lm-sys: SHA-256 mismatch for {local_name}: expected {sha256}, got {hash}");
    }

    fs::write(&dest, &buf).unwrap_or_else(|e| panic!("write {}: {e}", dest.display()));

    // The Bazel-built dylib ships with install_name=bazel-out/.../libLiteRtLmC.*
    // which is a relative path the macOS loader can't resolve. Rewrite to
    // @rpath/ so our -rpath emission works. This is safe because @rpath/ is
    // shorter than the Bazel path, so install_name_tool fits in the existing
    // header padding.
    if dest.extension().is_some_and(|e| e == "dylib") {
        let _ = std::process::Command::new("install_name_tool")
            .args(["-id", &format!("@rpath/{local_name}")])
            .arg(&dest)
            .status();
    }

    fs::write(&marker, sha256).expect("write verified marker");
}

fn ensure_wasm_tarball(url: &str, sha256: &str, size: u64, cache_dir: &Path) {
    if sha256.chars().all(|c| c == '0') {
        panic!(
            "litert-lm-sys: wasm32-unknown-emscripten prebuilt not yet published.\n\
             Run the `build-litertlm-wasm.yml` GitHub Actions workflow, then update\n\
             the SHA-256 placeholder in litert-lm-sys/build.rs. Or set\n\
             LITERT_LM_LIB_DIR to a directory of `.a` files from a local build\n\
             (see wasm-patches/litert-lm-v0.10.2/).",
        );
    }
    let marker = cache_dir.join(".wasm-tarball.verified");
    if marker.exists() && fs::read_to_string(&marker).ok().as_deref() == Some(sha256) {
        return;
    }
    if env::var_os("LITERT_NO_DOWNLOAD").is_some() {
        panic!(
            "litert-lm-sys: LITERT_NO_DOWNLOAD set but WASM tarball missing from {}",
            cache_dir.display()
        );
    }

    println!(
        "cargo:warning=litert-lm-sys: downloading WASM static-archive bundle \
         {url} into {} (first build only)",
        cache_dir.display(),
    );

    let mut buf = Vec::with_capacity(size as usize);
    ureq::get(url)
        .call()
        .unwrap_or_else(|e| panic!("GET {url}: {e}"))
        .into_reader()
        .read_to_end(&mut buf)
        .unwrap_or_else(|e| panic!("read tarball: {e}"));
    if buf.len() as u64 != size {
        panic!(
            "litert-lm-sys: WASM tarball size mismatch: expected {size}, got {}",
            buf.len()
        );
    }
    let hash = hex(&Sha256::digest(&buf));
    if hash != sha256 {
        panic!("litert-lm-sys: WASM tarball SHA-256 mismatch: expected {sha256}, got {hash}");
    }
    let decoder = flate2::read::GzDecoder::new(buf.as_slice());
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(cache_dir)
        .unwrap_or_else(|e| panic!("extract tarball: {e}"));
    fs::write(&marker, sha256).expect("write wasm tarball marker");
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
        if target != "wasm32-unknown-emscripten" {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        }
        println!("cargo:lib_dir={dir}");
    }

    if target == "wasm32-unknown-emscripten" {
        // Glob lib*.a from the cache and emit -lstatic for each, same pattern
        // as litert-sys. wasm-ld dead-strips unreferenced syms.
        if let Some(dir) = lib_dir {
            let mut libs: Vec<String> = fs::read_dir(dir)
                .expect("read lib_dir for wasm")
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    name.strip_prefix("lib")
                        .and_then(|n| n.strip_suffix(".a"))
                        .map(str::to_owned)
                })
                .collect();
            // Roots first (litert_lm, litert) so the linker resolves them
            // before transitive deps.
            libs.sort_by_key(|n| {
                if n.starts_with("litert_lm") {
                    0
                } else if n.starts_with("litert") {
                    1
                } else {
                    2
                }
            });
            for lib in libs {
                println!("cargo:rustc-link-lib=static={lib}");
            }
        }
        println!("cargo:rustc-link-lib=c++");
        return;
    }

    println!("cargo:rustc-link-lib=dylib=LiteRtLmC");

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
    if target.contains("android") {
        println!("cargo:rustc-link-lib=dylib=log");
    }
}
