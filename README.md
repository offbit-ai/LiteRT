# LiteRT-rs

[![CI](https://github.com/offbit-ai/LiteRT/actions/workflows/ci.yml/badge.svg)](https://github.com/offbit-ai/LiteRT/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/litert.svg?label=litert)](https://crates.io/crates/litert)
[![crates.io](https://img.shields.io/crates/v/litert-sys.svg?label=litert-sys)](https://crates.io/crates/litert-sys)
[![docs.rs](https://img.shields.io/docsrs/litert?label=docs.rs%2Flitert)](https://docs.rs/litert)
[![MSRV](https://img.shields.io/badge/rustc-1.75%2B-blue.svg)](https://releases.rs/docs/1.75.0/)
[![License](https://img.shields.io/badge/license-Apache--2.0-informational.svg)](LICENSE)
[![LiteRT](https://img.shields.io/badge/LiteRT-2.1.4-informational.svg)](https://github.com/google-ai-edge/LiteRT)

Safe, zero-friction Rust bindings for [Google LiteRT] 2.x — the on-device
machine-learning runtime formerly known as TensorFlow Lite. Add the crate to
your `Cargo.toml` and `cargo build`. No Bazel, no CMake, no `libclang` on
user machines.

[Google LiteRT]: https://ai.google.dev/edge/litert

```toml
[dependencies]
litert = "0.1"
```

```rust
use litert::{CompilationOptions, CompiledModel, Environment, Model, TensorBuffer};

let env = Environment::new()?;
let model = Model::from_file("add_10x10.tflite")?;

let sig = model.signature(0)?;
let mut inputs: Vec<_> = (0..sig.input_count()?)
    .map(|i| TensorBuffer::managed_host(&env, &sig.input_shape(i)?))
    .collect::<Result<_, _>>()?;
let mut outputs = vec![TensorBuffer::managed_host(&env, &sig.output_shape(0)?)?];

{
    let mut w = inputs[0].lock_for_write::<f32>()?;
    w.copy_from_slice(&[1.0, 2.0, /* … */]);
}

let compiled = CompiledModel::new(env, model, &CompilationOptions::new()?)?;
compiled.run(&mut inputs, &mut outputs)?;

let out = outputs[0].lock_for_read::<f32>()?;
# Ok::<(), litert::Error>(())
```

## Why

| Other options                      | Friction                                                            |
|------------------------------------|---------------------------------------------------------------------|
| Build LiteRT from source via CMake | Bazel or CMake + protoc + flatc + abseil + Android NDK on your box  |
| Invoke via Python (`ai-edge-litert`) | Python interpreter + wheel dependency graph                         |
| Hand-roll FFI against TFLite C API | Maintain a sysroot per target + track header drift manually         |

`litert-rs` takes the same upstream runtime binaries Google publishes, pins
each by SHA-256, and downloads them into a user-level cache the first time
`cargo build` runs. Your app links against that cached `libLiteRt.{so,dylib,dll}`.

## Crates in the workspace

| Crate           | What it is                                   | Status           |
|-----------------|----------------------------------------------|------------------|
| `litert-sys`    | Raw FFI against the LiteRT 2.x C API         | 0.1.x            |
| `litert`        | Idiomatic safe wrappers                      | 0.1.x            |
| `litert-lm-sys` | Raw FFI against LiteRT-LM (`c/engine.h`)     | stubbed, 0.2.0   |
| `litertlm`      | Safe LLM-inference API                       | 0.2.x            |

The LiteRT-LM side requires building a static C++ library from source with
heavy transitive dependencies (abseil, protobuf, flatbuffers, nlohmann_json);
it's deferred until a mirrored build-and-package pipeline exists.

## Platform support

| Rust target                     | CPU | GPU accelerator(s) shipped        | Source                         |
|---------------------------------|-----|-----------------------------------|--------------------------------|
| `aarch64-apple-darwin`          | ✅  | Metal, WebGPU                     | litert-lm prebuilt (Git LFS)   |
| `x86_64-unknown-linux-gnu`      | ✅  | WebGPU                            | litert-lm prebuilt (Git LFS)   |
| `aarch64-unknown-linux-gnu`     | ✅  | WebGPU                            | litert-lm prebuilt (Git LFS)   |
| `x86_64-pc-windows-msvc`        | ✅  | WebGPU                            | litert-lm prebuilt (Git LFS)   |
| `aarch64-linux-android`         | ✅  | OpenCL/GL (via `ClGlAccelerator`) | LiteRT Maven AAR               |
| `x86_64-linux-android`          | ✅  | OpenCL/GL                         | LiteRT Maven AAR               |
| `aarch64-apple-ios`             | ⏳  | —                                 | deferred to 0.2.x (no upstream prebuilt) |

## Environment variables (escape hatches)

| Variable              | Effect                                                                     |
|-----------------------|----------------------------------------------------------------------------|
| `LITERT_LIB_DIR`      | Directory containing `libLiteRt.{so,dylib,dll}`. Bypasses the downloader. |
| `LITERT_NO_DOWNLOAD`  | Fail the build if any prebuilt is missing from cache (air-gapped CI).     |
| `LITERT_CACHE_DIR`    | Override the cache root. Default: `$XDG_CACHE_HOME/litert-sys`.           |

## macOS downstream binaries (one-time setup)

The prebuilt `libLiteRt.dylib` Google ships has `install_name=@rpath/libLiteRt.dylib`
and wasn't linked with `-headerpad_max_install_names`, so we can't rewrite that
identifier to an absolute path post-download. `litert-sys`' build script emits
an `-rpath` flag for its own tests and examples, but Cargo's `rustc-link-arg`
does **not** propagate to downstream consumer binaries. Without action, the
binaries your crate produces on macOS will fail at launch with:

    dyld: Library not loaded: @rpath/libLiteRt.dylib

Fix it once per downstream crate — add this tiny [build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
next to your `Cargo.toml`:

```rust
// build.rs
fn main() {
    // `litert-sys` declares `links = "LiteRt"` and publishes its cache
    // directory as `DEP_LITERT_LIB_DIR`. Embedding it as an rpath makes
    // dyld find libLiteRt.dylib without DYLD_LIBRARY_PATH.
    if let Ok(dir) = std::env::var("DEP_LITERT_LIB_DIR") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
}
```

Alternatively, prefix individual invocations with
`DYLD_LIBRARY_PATH=$(cargo xtask cache-dir)`, or link with
`RUSTFLAGS="-C link-arg=-Wl,-rpath,/path/to/cache"`.

Linux, Windows, and Android are unaffected.

## Cross-platform development

End users only need `cargo`. The sections below are for contributors who want
to regenerate bindings or build for foreign targets locally.

### Tooling prerequisites (contributors only)

* `rustup` with stable + any target triples you want to exercise.
* A container engine for foreign-target builds: **Docker** or **Podman**.
  * macOS: `brew install podman && podman machine init && podman machine start`.
  * Linux: your distro's Docker/Podman packages.
* `cross` ≥ 0.2.5: `cargo install cross --locked`.
* If you're using Podman: `export CROSS_CONTAINER_ENGINE=podman`.

macOS and Windows target toolchains run **natively**, not through `cross`.
`cross` is only invoked for Linux + Android targets.

### Workspace automation

All cross-target chores flow through a single `xtask` binary.

```bash
cargo xtask targets            # list every supported Rust target triple
cargo xtask regen-bindings     # rebuild litert-sys bindings for every target
cargo xtask regen-bindings --target aarch64-apple-darwin   # single target
cargo xtask build-all          # cross-build the workspace for every target
```

`regen-bindings` dispatches automatically:

* **Host target** → native `cargo build -p litert-sys --features generate-bindings`.
* **Foreign target** → `cross build …`, which runs `bindgen` inside a
  container image that already has `libclang` + the target sysroot installed
  (see `Cross.toml`).

### CI

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs three matrices on
every push and PR:

1. **`native`** — macOS arm64 (tests + build), Linux x86_64 (tests + `fmt --check` + `clippy -D warnings`), Windows x86_64 (build).
2. **`cross`** — Linux arm64, Android arm64, Android x86_64 (build only).
3. **`bindings-drift`** — regenerates all 6 target binding files via `cargo xtask regen-bindings` and fails on any `git diff`. If drift is detected the regenerated files are uploaded as a build artifact so they can be inspected or accepted in a PR.

Drift is the authoritative check: if the CI-generated bindings for a target
differ from what's committed, that's the signal to update the committed file.

### First-build download

On the first `cargo build` for a given target, `litert-sys/build.rs` emits a
one-time warning while it fetches the pinned prebuilt libraries:

```
warning: litert-sys: downloading 4 file(s) of LiteRT prebuilt v0.10.2
         for target `macos_arm64` into
         /Users/you/Library/Caches/litert-sys/v0.10.2/aarch64-apple-darwin
         (first build only)
```

Subsequent builds hit the cache (with a size check; a SHA-256 verified marker
file short-circuits rehashing). Deleting the cache directory or bumping the
pinned upstream version triggers a fresh download + re-verification.

## Regenerating bindings for a new target

```bash
# 1. Add the triple to:
#      Cargo.toml workspace members (if needed)
#      Cross.toml (pre-build apt-get for libclang)
#      litert-sys/build.rs target_spec()  — with pinned checksums
#      xtask/src/main.rs TARGETS
#
# 2. Regenerate:
cargo xtask regen-bindings --target <new-target>
#
# 3. Commit litert-sys/src/bindings/<new-target>.rs and push.
```

## Credits

This project is a binding, not a fork. The runtime that does the work —
model loading, graph compilation, kernel execution, GPU/NPU delegation — is
Google's [LiteRT] and [LiteRT-LM]:

* **LiteRT** (Apache-2.0, © 2024–2026 Google LLC) — C API headers vendored
  under `third_party/litert-v2.1.4/`, source:
  <https://github.com/google-ai-edge/LiteRT>.
* **LiteRT-LM** (Apache-2.0, © 2024–2026 Google LLC) — source of the
  prebuilt `libLiteRt.*` + accelerator plugins we download:
  <https://github.com/google-ai-edge/litert-lm>.
* **TensorFlow Lite**, **XNNPACK**, **abseil-cpp**, **flatbuffers**,
  **protobuf** and the rest of the transitive open-source stack that LiteRT
  itself is built on.

See [NOTICE](NOTICE) for the full attribution and
[third_party/litert-v2.1.4/LICENSE](third_party/litert-v2.1.4/LICENSE) for
the upstream LiteRT license text.

[LiteRT]: https://github.com/google-ai-edge/LiteRT
[LiteRT-LM]: https://github.com/google-ai-edge/litert-lm

## License

Licensed under the [Apache License, Version 2.0](LICENSE). By contributing
you agree that your contribution is licensed under the same terms.
