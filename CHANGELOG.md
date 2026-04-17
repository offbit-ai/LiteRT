# Changelog

All notable changes are listed here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] — Multimodal vision + streaming fixes

- **Vision inference** via `Conversation::send_raw_stream` with image file
  paths in JSON. Tested with Gemma 4 E2B (2.4 GB) — correctly identifies
  objects and describes scenes from JPEG images.
- **`Input` enum** — `Text`, `Image`, `ImageEnd`, `Audio`, `AudioEnd` for
  `Session::generate_with_inputs`.
- **`Conversation::send_raw_stream`** now public for custom JSON messages
  (multimodal content arrays with image paths).
- Vision encoder runs on CPU by default to avoid absl ODR crash between
  `libLiteRtLmC` and `libLiteRtWebGpuAccelerator` (both statically link
  absl; text generation still uses GPU).
- Log suppression: `litert_lm_set_min_log_level(3)` + `TF_CPP_MIN_LOG_LEVEL`.

## [0.2.0] — LiteRT-LM crates

Adds `litert-lm-sys` and `litert-lm` for on-device LLM inference.
End-to-end verified: Qwen3-0.6B generates text on both CPU and GPU.

### New crates

- **`litert-lm-sys`** — 46 `litert_lm_*` FFI bindings from `c/engine.h`.
  `build.rs` downloads `libLiteRtLmC.{so,dylib}` (built from source via
  Bazel) from our mirrored GitHub release, SHA-256-verified.
- **`litertlm`** — safe API: `Engine`, `EngineSettings` with typed
  `Backend` enum, `Session::generate`, `Conversation::send_message_stream`
  for token-by-token streaming, `SamplerParams` (TopK/TopP/Greedy).

### Highlights

- **GPU inference** works via `CreateAny` factory (Metal/WebGPU).
- **CPU inference** works with TopP sampling and null vision/audio backends.
- **Conversation API** applies model prompt templates correctly for streaming.
- Auto-download of Qwen3-0.6B in the example.

### Known limitations

- WebGPU delegate `I0000` log lines go to stderr (hardcoded, can't suppress).
- CPU TopK sampler not implemented upstream; use TopP.
- Windows and Android not yet supported for `litert-lm-sys`.

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
