# Changelog

All notable changes are listed here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — LiteRT-LM crates

Adds `litert-lm-sys` (raw FFI) and `litert-lm` (safe API) for on-device
LLM inference via Google's LiteRT-LM engine.

### New crates

- **`litert-lm-sys 0.2.0`** — raw `litert_lm_*` C API bindings (46
  functions from `c/engine.h`). `build.rs` downloads a pinned
  `libLiteRtLmC.{so,dylib}` from our mirrored GitHub release, built from
  source via Bazel `cc_binary(linkshared=True)`.
- **`litert-lm 0.2.0`** — safe wrappers: `Engine` + `EngineSettings`
  builder with typed `Backend` enum (Cpu/Gpu/Npu), `Session::generate`,
  `SamplerParams` (TopK/TopP/Greedy).

### Safe API

```rust
use litert_lm::{Backend, Engine, EngineSettings, SamplerParams};

let engine = Engine::new(
    EngineSettings::new("model.litertlm")
        .backend(Backend::Gpu)
        .max_num_tokens(512),
)?;
let mut session = engine.create_session(SamplerParams::default())?;
let response = session.generate("Hello")?;
```

### Known limitations

- GPU (Metal/WebGPU) shader compilation takes 10+ minutes on first run
  for 600MB+ models; cached on subsequent runs.
- CPU backend on text-only models triggers an upstream "vision encoder not
  found" error; GPU backend handles missing modalities gracefully.
- Windows and Android targets not yet supported for `litert-lm-sys`.

### Build pipeline

`libLiteRtLmC` is built from `google-ai-edge/litert-lm` v0.10.2 via a
GitHub Actions workflow using Bazel, producing Linux x86_64 (.so, 45 MB)
and macOS arm64 (.dylib, 29 MB) artifacts mirrored to a GitHub release.

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
