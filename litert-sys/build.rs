//! Build script for litert-sys.
//!
//! Pure-Rust end-user experience: `cargo build` downloads a pinned set of
//! LiteRT prebuilt shared libraries (via Git LFS), verifies their SHA-256
//! against checksums compiled into this binary, caches them in
//! `$XDG_CACHE_HOME/litert-sys/<tag>/<target>/`, and emits the linker
//! directives needed to load them at runtime.
//!
//! Escape hatches (in priority order):
//!   `LITERT_LIB_DIR`       — point at a directory containing libLiteRt.*;
//!                             the download step is skipped entirely.
//!   `LITERT_NO_DOWNLOAD`   — fail hard if the cache is empty (air-gapped CI).
//!   `LITERT_CACHE_DIR`     — override the cache location.
//!
//! Maintainer feature:
//!   `--features generate-bindings` — re-run bindgen on `wrapper.h` against
//!                                     the vendored headers (needs libclang).

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

const LITERT_LM_TAG: &str = "v0.10.2";
const LITERT_MAVEN_VERSION: &str = "2.1.4";

#[cfg(feature = "generate-bindings")]
const LITERT_HEADERS_VERSION: &str = "2.1.4";

/// A single prebuilt file pinned by SHA-256 (which is the Git LFS OID, so the
/// same string serves as both the content address for the download request
/// and the checksum we verify against).
struct Prebuilt {
    name: &'static str,
    oid: &'static str,
    size: u64,
}

/// Where a target's native libraries come from.
enum DistKind {
    /// Desktop targets: Git LFS blobs in `google-ai-edge/litert-lm`, one file
    /// per entry in `files`.
    LiteRtLmLfs {
        upstream_dir: &'static str,
        files: &'static [Prebuilt],
    },
    /// Android targets: a single AAR on Google Maven, from which we extract
    /// `jni/<abi>/<name>` entries.
    MavenAar {
        abi: &'static str,
        outputs: &'static [&'static str],
    },
    /// WASM targets: a tar.gz on the LiteRT-rs GitHub release (built by the
    /// `build-litert-wasm.yml` workflow). Contains static archives (`.a`)
    /// from a CMake+emscripten build of LiteRT v2.1.4. The list of `-lstatic`
    /// directives to emit lives in `WASM32_EMSCRIPTEN_LIBS`.
    WasmTarball {
        url: &'static str,
        oid: &'static str,
        size: u64,
    },
}

struct TargetSpec {
    dist: DistKind,
}

// Pinned Android AAR — same file for every Android target, sourced from
// Google Maven. `com.google.ai.edge.litert:litert:<LITERT_MAVEN_VERSION>`.
const ANDROID_AAR_SHA256: &str = "29ce4fdc362306f3793b910d759d93867a5c2204bb890ac5f9410c62c3f7482a";
const ANDROID_AAR_SIZE: u64 = 13_844_131;
fn android_aar_url() -> String {
    format!(
        "https://dl.google.com/android/maven2/com/google/ai/edge/litert/litert/\
         {LITERT_MAVEN_VERSION}/litert-{LITERT_MAVEN_VERSION}.aar"
    )
}
const ANDROID_OUTPUTS: &[&str] = &["libLiteRt.so", "libLiteRtClGlAccelerator.so"];

const MACOS_ARM64: &[Prebuilt] = &[
    Prebuilt {
        name: "libLiteRt.dylib",
        oid: "382effa82b9830d96f73f9ed545462c9eb64786b5f63d8f3c5affb3a16fd28eb",
        size: 10_535_776,
    },
    Prebuilt {
        name: "libLiteRtMetalAccelerator.dylib",
        oid: "4ff8b68149ac5bc665ae3a1e11473c3a1deebb7af53c657326810867ea937343",
        size: 10_005_920,
    },
    Prebuilt {
        name: "libLiteRtWebGpuAccelerator.dylib",
        oid: "dc3822d1004d502d5e79e8158eb7cf8c919011202fea3f156e94f86ad14f4f71",
        size: 24_015_584,
    },
    Prebuilt {
        name: "libLiteRtTopKWebGpuSampler.dylib",
        oid: "618efc1173194ca6658704a40890e98818cde4320b2e0f9013e959bd2c0268da",
        size: 22_264_864,
    },
    // Required by litert-lm-sys's libLiteRtLmC at runtime.
    Prebuilt {
        name: "libGemmaModelConstraintProvider.dylib",
        oid: "b584d9041af42fec0879593d747ba1bda139c25398f41c5c1c2c8dfa6c457008",
        size: 9_214_976,
    },
];

const LINUX_X86_64: &[Prebuilt] = &[
    Prebuilt {
        name: "libLiteRt.so",
        oid: "e9844d634dbb69dbeb0bc51a71f7035bb7ba523e876384ff58192955b1da63e4",
        size: 10_050_528,
    },
    Prebuilt {
        name: "libLiteRtWebGpuAccelerator.so",
        oid: "9523c6fd38f661599b904908f87d22448c2ff2c8da54291782e0c23fcf988863",
        size: 17_606_760,
    },
    Prebuilt {
        name: "libLiteRtTopKWebGpuSampler.so",
        oid: "f44b2eaded0a5b2e015c88a4eb6af960811c5a5df140f9101f84d845e8aff0ca",
        size: 4_210_232,
    },
];

const LINUX_ARM64: &[Prebuilt] = &[
    Prebuilt {
        name: "libLiteRt.so",
        oid: "f541933152a7eb651707e610e4e173dba7ee484aee7f1b6f83fb32384a8a5398",
        size: 8_172_696,
    },
    Prebuilt {
        name: "libLiteRtWebGpuAccelerator.so",
        oid: "e09dd392f0f21bd64437f36ca9a28e109cff7b662bafd66ef5b5f2957023d433",
        size: 15_813_112,
    },
    Prebuilt {
        name: "libLiteRtTopKWebGpuSampler.so",
        oid: "90328888d96b4bfedb5ad30a7eecd8b213d25a6a17af44774f213612f47f1a84",
        size: 3_788_400,
    },
];

const WINDOWS_X86_64: &[Prebuilt] = &[
    Prebuilt {
        name: "libLiteRt.dll",
        oid: "d87f00b4161046f23c911fe8f64224bcbdff8399be792bcb62bf95b52cbbccc5",
        size: 11_013_632,
    },
    Prebuilt {
        name: "libLiteRtWebGpuAccelerator.dll",
        oid: "69a8b4671fe5f9f16054b5be211c484d954b95fbf459359d445319c6039b176b",
        size: 21_816_320,
    },
    Prebuilt {
        name: "libLiteRtTopKWebGpuSampler.dll",
        oid: "31cfc379259d553ea4eeb7f4f949f116629b4882feafce343de57d2e837b3903",
        size: 16_971_776,
    },
];

// WASM static-archive bundle produced by `.github/workflows/build-litert-wasm.yml`
// (CMake+emscripten build of LiteRT v2.1.4 + our `wasm-patches/`). Contains
// ~88 `.a` files (litert C/C++ API, runtime, TFLite, XNNPACK, abseil,
// flatbuffers, gemmlowp, cpuinfo, pthreadpool — total ~15 MB extracted).
//
// At link time we glob the cache dir and emit `-l static=<name>` for every
// `lib<name>.a` we find — the upstream build's archive surface evolves over
// time, and a static allowlist would rot. wasm-ld dead-strips unused syms.
//
// TODO(0.3.0-rc): replace placeholder oid/size after the GitHub Actions
// workflow has uploaded the artifact and we've recorded the SHA-256.
const WASM32_EMSCRIPTEN_TARBALL_URL: &str =
    "https://github.com/offbit-ai/LiteRT/releases/download/wasm-prebuilt-v2.1.4/\
     libLiteRt-wasm32-emscripten.tar.gz";
const WASM32_EMSCRIPTEN_OID: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
const WASM32_EMSCRIPTEN_SIZE: u64 = 0;

fn target_spec(target: &str) -> Option<TargetSpec> {
    let dist = match target {
        "aarch64-apple-darwin" => DistKind::LiteRtLmLfs {
            upstream_dir: "macos_arm64",
            files: MACOS_ARM64,
        },
        "x86_64-unknown-linux-gnu" => DistKind::LiteRtLmLfs {
            upstream_dir: "linux_x86_64",
            files: LINUX_X86_64,
        },
        "aarch64-unknown-linux-gnu" => DistKind::LiteRtLmLfs {
            upstream_dir: "linux_arm64",
            files: LINUX_ARM64,
        },
        "x86_64-pc-windows-msvc" => DistKind::LiteRtLmLfs {
            upstream_dir: "windows_x86_64",
            files: WINDOWS_X86_64,
        },
        "aarch64-linux-android" => DistKind::MavenAar {
            abi: "arm64-v8a",
            outputs: ANDROID_OUTPUTS,
        },
        "x86_64-linux-android" => DistKind::MavenAar {
            abi: "x86_64",
            outputs: ANDROID_OUTPUTS,
        },
        "wasm32-unknown-emscripten" => DistKind::WasmTarball {
            url: WASM32_EMSCRIPTEN_TARBALL_URL,
            oid: WASM32_EMSCRIPTEN_OID,
            size: WASM32_EMSCRIPTEN_SIZE,
        },
        _ => return None,
    };
    Some(TargetSpec { dist })
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-env-changed=LITERT_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LITERT_NO_DOWNLOAD");
    println!("cargo:rerun-if-env-changed=LITERT_CACHE_DIR");

    let target = env::var("TARGET").expect("TARGET env var missing");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR env var missing"));

    emit_bindings(&target, &out_dir);
    let lib_dir = locate_library(&target);
    emit_link_directives(&target, lib_dir.as_deref());
}

#[cfg(feature = "generate-bindings")]
fn emit_bindings(_target: &str, out_dir: &Path) {
    generate_bindings(out_dir);
}

#[cfg(not(feature = "generate-bindings"))]
fn emit_bindings(target: &str, out_dir: &Path) {
    let pregenerated = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("bindings")
        .join(format!("{target}.rs"));

    if !pregenerated.exists() {
        panic!(
            "litert-sys: no pre-generated bindings for target `{target}`.\n\
             Run with `--features generate-bindings` (needs libclang) or add \
             src/bindings/{target}.rs.",
        );
    }
    fs::copy(&pregenerated, out_dir.join("bindings.rs")).expect("copy pre-generated bindings");
    println!("cargo:rerun-if-changed={}", pregenerated.display());
}

#[cfg(feature = "generate-bindings")]
fn generate_bindings(out_dir: &Path) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let headers_dir = manifest_dir
        .join("third_party")
        .join(format!("litert-v{LITERT_HEADERS_VERSION}"));
    assert!(
        headers_dir.join("litert/c/litert_common.h").exists(),
        "vendored headers not found at {}",
        headers_dir.display()
    );

    bindgen::Builder::default()
        .header(manifest_dir.join("wrapper.h").to_string_lossy())
        .clang_arg(format!("-I{}", headers_dir.display()))
        // LiteRT uses `typedef enum : int8_t { ... }` (C23 typed enums) in a
        // few headers — e.g. litert_logging.h. Default C dialect rejects it.
        .clang_arg("-std=gnu2x")
        .allowlist_function("LiteRt.*")
        .allowlist_type("LiteRt.*")
        .allowlist_type("kLiteRt.*")
        .allowlist_var("kLiteRt.*")
        .allowlist_var("LITERT_.*")
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

// ---------------------------------------------------------------------------
// Prebuilt library resolution
// ---------------------------------------------------------------------------

fn locate_library(target: &str) -> Option<PathBuf> {
    // 1) Explicit override. Always wins.
    if let Ok(dir) = env::var("LITERT_LIB_DIR") {
        let dir = PathBuf::from(dir);
        assert!(
            dir.is_dir(),
            "LITERT_LIB_DIR={} is not a directory",
            dir.display()
        );
        return Some(dir);
    }

    // 2) Look up the target's pinned prebuilts.
    let spec = target_spec(target).unwrap_or_else(|| {
        panic!(
            "litert-sys: target `{target}` has no pinned prebuilt checksums.\n\
             Set LITERT_LIB_DIR to point at a local libLiteRt build, or file \
             an issue to add this target.",
        );
    });

    let cache_dir = cache_dir_for(target);
    ensure_prebuilts(&spec, &cache_dir);
    Some(cache_dir)
}

fn cache_root() -> PathBuf {
    if let Some(dir) = env::var_os("LITERT_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    // Prefer the user-level cache. If we can't actually create it (e.g.,
    // running inside a container where `$HOME` points somewhere read-only,
    // as is the case in some cross-rs images), fall through to OUT_DIR so
    // the build doesn't panic before even attempting a download.
    if let Some(dir) = dirs::cache_dir() {
        if fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }
    PathBuf::from(env::var("OUT_DIR").unwrap()).join("litert-cache")
}

fn cache_dir_for(target: &str) -> PathBuf {
    cache_root()
        .join("litert-sys")
        .join(LITERT_LM_TAG)
        .join(target)
}

fn ensure_prebuilts(spec: &TargetSpec, cache_dir: &Path) {
    fs::create_dir_all(cache_dir).expect("create cache dir");
    match &spec.dist {
        DistKind::LiteRtLmLfs {
            upstream_dir,
            files,
        } => ensure_lfs_prebuilts(upstream_dir, files, cache_dir),
        DistKind::MavenAar { abi, outputs } => ensure_aar_prebuilts(abi, outputs, cache_dir),
        DistKind::WasmTarball { url, oid, size } => ensure_wasm_tarball(url, oid, *size, cache_dir),
    }
}

fn ensure_wasm_tarball(url: &str, oid: &str, size: u64, cache_dir: &Path) {
    if oid.chars().all(|c| c == '0') {
        panic!(
            "litert-sys: wasm32-unknown-emscripten prebuilt not yet published.\n\
             Run the `build-litert-wasm.yml` GitHub Actions workflow, then update\n\
             WASM32_EMSCRIPTEN_OID/SIZE in litert-sys/build.rs from the recorded\n\
             SHA-256. Or set LITERT_LIB_DIR to a local build (see \
             wasm-patches/litert-v2.1.4/).",
        );
    }
    let marker = cache_dir.join(".wasm-tarball.verified");
    if marker.exists() && fs::read_to_string(&marker).ok().as_deref() == Some(oid) {
        return; // Already extracted and verified.
    }
    assert_downloads_allowed(
        cache_dir,
        std::iter::once("libLiteRt-wasm32-emscripten.tar.gz"),
    );

    println!(
        "cargo:warning=litert-sys: downloading WASM static-archive bundle \
         {url} into {} (first build only)",
        cache_dir.display(),
    );

    let mut buf = Vec::with_capacity(size as usize);
    ureq::get(url)
        .call()
        .unwrap_or_else(|e| panic!("GET {url} failed: {e}"))
        .into_reader()
        .read_to_end(&mut buf)
        .unwrap_or_else(|e| panic!("read tarball: {e}"));
    if buf.len() as u64 != size {
        panic!(
            "litert-sys: WASM tarball size mismatch: expected {size}, got {}",
            buf.len()
        );
    }
    let hash = hex(&Sha256::digest(&buf));
    if hash != oid {
        panic!("litert-sys: WASM tarball SHA-256 mismatch: expected {oid}, got {hash}");
    }
    let decoder = flate2::read::GzDecoder::new(buf.as_slice());
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(cache_dir)
        .unwrap_or_else(|e| panic!("extract tarball: {e}"));
    fs::write(&marker, oid).expect("write wasm tarball marker");
}

fn ensure_lfs_prebuilts(upstream_dir: &str, files: &[Prebuilt], cache_dir: &Path) {
    let missing: Vec<&Prebuilt> = files.iter().filter(|p| !file_ok(cache_dir, p)).collect();
    if missing.is_empty() {
        return;
    }
    assert_downloads_allowed(cache_dir, missing.iter().map(|p| p.name));

    println!(
        "cargo:warning=litert-sys: downloading {} file(s) of LiteRT prebuilt \
         {LITERT_LM_TAG} for target `{upstream_dir}` into {} (first build only)",
        missing.len(),
        cache_dir.display(),
    );

    let urls = resolve_lfs_urls(&missing);
    for p in &missing {
        let url = urls
            .get(p.oid)
            .unwrap_or_else(|| panic!("LFS batch response missing URL for oid {}", p.oid));
        download_and_verify(url, p, cache_dir);
    }
}

fn ensure_aar_prebuilts(abi: &str, outputs: &[&str], cache_dir: &Path) {
    let missing: Vec<&str> = outputs
        .iter()
        .copied()
        .filter(|name| !aar_output_ok(cache_dir, name))
        .collect();
    if missing.is_empty() {
        return;
    }
    assert_downloads_allowed(cache_dir, missing.iter().copied());

    let aar_cache = cache_root()
        .join("litert-sys")
        .join(format!("maven-{LITERT_MAVEN_VERSION}"));
    fs::create_dir_all(&aar_cache).expect("create maven cache");
    let aar_path = aar_cache.join(format!("litert-{LITERT_MAVEN_VERSION}.aar"));

    if !aar_is_verified(&aar_path) {
        println!(
            "cargo:warning=litert-sys: downloading LiteRT Maven AAR \
             {LITERT_MAVEN_VERSION} into {} (first build only)",
            aar_path.display()
        );
        let url = android_aar_url();
        let mut buf = Vec::with_capacity(ANDROID_AAR_SIZE as usize);
        ureq::get(&url)
            .call()
            .unwrap_or_else(|e| panic!("GET {url}: {e}"))
            .into_reader()
            .read_to_end(&mut buf)
            .unwrap_or_else(|e| panic!("read AAR: {e}"));
        if buf.len() as u64 != ANDROID_AAR_SIZE {
            panic!(
                "litert-sys: AAR size mismatch: expected {ANDROID_AAR_SIZE}, got {}",
                buf.len()
            );
        }
        let hash = hex(&Sha256::digest(&buf));
        if hash != ANDROID_AAR_SHA256 {
            panic!("litert-sys: AAR SHA-256 mismatch: expected {ANDROID_AAR_SHA256}, got {hash}");
        }
        fs::write(&aar_path, &buf).expect("write AAR");
        fs::write(aar_cache.join(".aar.verified"), ANDROID_AAR_SHA256)
            .expect("write AAR verified marker");
    }

    // Extract each requested file from jni/<abi>/ inside the AAR.
    let aar_bytes = fs::read(&aar_path).expect("read AAR from cache");
    let reader = std::io::Cursor::new(aar_bytes);
    let mut archive = zip::ZipArchive::new(reader).expect("open AAR as zip");
    for name in &missing {
        let member = format!("jni/{abi}/{name}");
        let mut file = archive
            .by_name(&member)
            .unwrap_or_else(|e| panic!("AAR missing entry {member}: {e}"));
        let mut bytes = Vec::with_capacity(file.size() as usize);
        std::io::Read::read_to_end(&mut file, &mut bytes).expect("read AAR entry");
        let dest = cache_dir.join(name);
        fs::write(&dest, &bytes).unwrap_or_else(|e| panic!("write {}: {e}", dest.display()));
        // Record the extracted SHA so we can skip rework on subsequent builds.
        let hash = hex(&Sha256::digest(&bytes));
        fs::write(cache_dir.join(format!("{name}.verified")), hash).expect("write verified marker");
    }
}

fn assert_downloads_allowed<'a>(cache_dir: &Path, missing: impl Iterator<Item = &'a str>) {
    if env::var_os("LITERT_NO_DOWNLOAD").is_some() {
        let names: Vec<&str> = missing.collect();
        panic!(
            "litert-sys: LITERT_NO_DOWNLOAD set but {} file(s) missing from {}: {}",
            names.len(),
            cache_dir.display(),
            names.join(", "),
        );
    }
}

fn aar_output_ok(cache_dir: &Path, name: &str) -> bool {
    cache_dir.join(name).exists() && cache_dir.join(format!("{name}.verified")).exists()
}

fn aar_is_verified(aar_path: &Path) -> bool {
    let Ok(meta) = fs::metadata(aar_path) else {
        return false;
    };
    if meta.len() != ANDROID_AAR_SIZE {
        return false;
    }
    aar_path
        .parent()
        .map(|p| p.join(".aar.verified").exists())
        .unwrap_or(false)
}

fn file_ok(dir: &Path, p: &Prebuilt) -> bool {
    // A `.verified` marker is written only after the bytes matched the
    // pinned SHA-256. Its presence (alongside the file) is authoritative —
    // we don't re-check file length here because post-download mutations
    // like install_name_tool may legitimately change the byte count on
    // macOS. Absent the marker, the file is treated as missing and
    // re-downloaded from scratch.
    dir.join(p.name).exists() && dir.join(format!("{}.verified", p.name)).exists()
}

fn download_and_verify(url: &str, p: &Prebuilt, dir: &Path) {
    let mut buf = Vec::with_capacity(p.size as usize);
    ureq::get(url)
        .call()
        .unwrap_or_else(|e| panic!("GET {} failed: {e}", p.name))
        .into_reader()
        .read_to_end(&mut buf)
        .unwrap_or_else(|e| panic!("read {} failed: {e}", p.name));

    if buf.len() as u64 != p.size {
        panic!(
            "litert-sys: size mismatch for {}: expected {}, got {}",
            p.name,
            p.size,
            buf.len()
        );
    }
    let hash = hex(&Sha256::digest(&buf));
    if hash != p.oid {
        panic!(
            "litert-sys: SHA-256 mismatch for {}: expected {}, got {}",
            p.name, p.oid, hash
        );
    }

    let out = dir.join(p.name);
    fs::write(&out, &buf).unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
    fs::write(dir.join(format!("{}.verified", p.name)), p.oid).expect("write verified marker");
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
// Git LFS batch API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LfsBatchResponse {
    objects: Vec<LfsObject>,
}

#[derive(Deserialize)]
struct LfsObject {
    oid: String,
    actions: Option<LfsActions>,
    error: Option<LfsError>,
}

#[derive(Deserialize)]
struct LfsActions {
    download: LfsAction,
}

#[derive(Deserialize)]
struct LfsAction {
    href: String,
}

#[derive(Deserialize, Debug)]
struct LfsError {
    #[allow(dead_code)]
    code: i32,
    message: String,
}

fn resolve_lfs_urls(files: &[&Prebuilt]) -> std::collections::HashMap<String, String> {
    let body = serde_json::json!({
        "operation": "download",
        "transfers": ["basic"],
        "objects": files.iter().map(|p| serde_json::json!({
            "oid": p.oid,
            "size": p.size,
        })).collect::<Vec<_>>(),
    })
    .to_string();

    let resp = ureq::post("https://github.com/google-ai-edge/litert-lm.git/info/lfs/objects/batch")
        .set("Accept", "application/vnd.git-lfs+json")
        .set("Content-Type", "application/vnd.git-lfs+json")
        .send_string(&body)
        .unwrap_or_else(|e| panic!("LFS batch request failed: {e}"));

    let parsed: LfsBatchResponse =
        serde_json::from_reader(resp.into_reader()).expect("parse LFS batch response");

    let mut out = std::collections::HashMap::new();
    for obj in parsed.objects {
        if let Some(err) = obj.error {
            panic!("LFS server refused oid {}: {}", obj.oid, err.message);
        }
        let action = obj
            .actions
            .unwrap_or_else(|| panic!("LFS object {} has no actions", obj.oid));
        out.insert(obj.oid, action.download.href);
    }
    out
}

// ---------------------------------------------------------------------------
// Linker directives
// ---------------------------------------------------------------------------

fn emit_link_directives(target: &str, lib_dir: Option<&Path>) {
    if let Some(dir) = lib_dir {
        let dir = dir.display();
        println!("cargo:rustc-link-search=native={dir}");
        if target != "wasm32-unknown-emscripten" {
            // wasm-ld doesn't understand -rpath; emcc rejects it as well.
            println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        }
        println!("cargo:lib_dir={dir}");
    }

    if target == "wasm32-unknown-emscripten" {
        // Glob lib*.a under the lib_dir (or LITERT_LIB_DIR) and emit a
        // -lstatic for each. wasm-ld will dead-strip unreferenced syms, so
        // listing all archives is harmless. Roots first (litert) helps the
        // linker resolve faster but isn't required for correctness.
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
            // Ordering: ensure litert_* roots come first so wasm-ld
            // touches them before transitive deps.
            libs.sort_by_key(|n| !n.starts_with("litert"));
            for lib in libs {
                println!("cargo:rustc-link-lib=static={lib}");
            }
        }
        // emscripten ships libc++ in its sysroot.
        println!("cargo:rustc-link-lib=c++");
        return;
    }

    println!("cargo:rustc-link-lib=dylib=LiteRt");

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
    if target.contains("android") {
        println!("cargo:rustc-link-lib=dylib=log");
    }
}
