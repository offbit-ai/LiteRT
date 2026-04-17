# Changelog

All notable changes are listed here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### In-progress: litert-lm (not yet published)

`litert-lm-sys` and `litert-lm` are scaffolded in the workspace with
bindings, safe API, and a mirrored shared library — but NOT published to
crates.io. Text generation has not been verified end-to-end yet:

- GPU backend: Metal shader compilation exceeds 10 min on first run for
  600MB models. Likely resolves with a warm cache but not yet confirmed.
- CPU backend: upstream `engine.cc` fatally requires a vision encoder
  section even on text-only models when the main backend is CPU.

These crates remain `publish = false` until actual text output is observed.

## [0.1.0] — initial release

Initial public release covering `litert-sys` (raw FFI) and `litert` (safe
wrappers) for Google's LiteRT 2.x on-device ML runtime.

### Published crates

- **`litert-sys 0.1.0`** — raw `LiteRt*` C API bindings, pre-generated for
  every supported target. `build.rs` downloads pinned prebuilt shared
  libraries (desktop via Git LFS, Android via Google Maven AAR) and
  verifies them against in-crate SHA-256 checksums before linking.
- **`litert 0.1.0`** — safe Rust wrappers: `Environment`, `Model`,
  `CompilationOptions`, `CompiledModel`, `TensorBuffer` with RAII lock
  guards, `Signature` introspection, `LogSeverity` control via runtime
  `libloading` dlsym.

### Not published (deferred to 0.2.0)

- `litert-lm-sys`, `litert-lm` — LiteRT-LM's C API is a static library
  with heavy dependencies (abseil, protobuf, flatbuffers, nlohmann_json);
  shipping requires standing up a CMake build-and-mirror pipeline first.

### Supported targets

| Rust target                   | CPU | GPU accelerator(s)       |
|-------------------------------|-----|--------------------------|
| `aarch64-apple-darwin`        | ✅  | Metal, WebGPU            |
| `x86_64-unknown-linux-gnu`    | ✅  | WebGPU                   |
| `aarch64-unknown-linux-gnu`   | ✅  | WebGPU                   |
| `x86_64-pc-windows-msvc`      | ✅  | WebGPU                   |
| `aarch64-linux-android`       | ✅  | OpenCL/GL                |
| `x86_64-linux-android`        | ✅  | OpenCL/GL                |

iOS (`aarch64-apple-ios`) is deferred — no prebuilt exists upstream and
source-building requires CocoaPods / XCFramework packaging we haven't
wired up yet.

### Pinned upstream versions

- LiteRT **v2.1.4** — C API headers vendored under `third_party/`.
- LiteRT-LM **v0.10.2** — source of the prebuilt `libLiteRt.*` shared
  libraries downloaded at build time.
- LiteRT Maven AAR **2.1.4** — Android `libLiteRt.so` + `libLiteRtClGlAccelerator.so`.

[Unreleased]: https://github.com/offbit-ai/LiteRT/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/offbit-ai/LiteRT/releases/tag/v0.1.0
