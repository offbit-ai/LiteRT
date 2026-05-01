# LiteRT-LM v0.10.2 WASM patches

Status: **work in progress (0.4.0 milestone)** — host prebuild stage is now
working, cross-compile stage not yet reached.

Patches needed to compile LiteRT-LM v0.10.2 to `wasm32-unknown-emscripten`
via `emcmake` + the upstream CMake build (`cmake/packages/litert_lm/`).

Apply patches in numeric order:
```bash
git apply 01-cmake-emscripten-support.patch
mkdir -p cmake/scripts
cp cmake/scripts/patch_libpng_pngpriv.cmake \
   /path/to/litert-lm/cmake/scripts/
```

## 01 — Top-level orchestrator + libpng FetchContent

Two changes needed before the build can even *start* under emcmake:

### 1.1 Orchestrator: force native compiler in prebuild stage

The top-level `CMakeLists.txt` runs an `ExternalProject_Add(litert_lm_prebuild ...)`
to build host protoc/flatc. Under `emcmake`, the parent CMake env has
`CC=emcc` / `CMAKE_TOOLCHAIN_FILE=Emscripten.cmake` injected, and
`ExternalProject_Add` inherits them, so the prebuild also tries to build
host tools with emcc — failing with `unsupported option '-arch'` or
similar because emcc doesn't accept Mach-O flags.

Fix: detect a native host compiler at top level, then in the prebuild
ExternalProject's `CONFIGURE_COMMAND`, run cmake via
`cmake -E env --unset=CC --unset=CMAKE_TOOLCHAIN_FILE …` and pass explicit
host compiler paths via `CMAKE_ARGS`. This isolates the prebuild env
from the parent's cross-compile env.

### 1.2 libpng v1.6.40 + modern Apple SDK

libpng v1.6.40's `pngpriv.h` line 517-518 includes `<fp.h>` (Classic Mac OS
header that doesn't exist on modern macOS or Linux/emcc) when
`TARGET_OS_MAC` is defined. Apple's `TargetConditionals.h` defines that
on every modern macOS — so the host prebuild fails before getting to
emcc.

Fix: add a `PATCH_COMMAND` to libpng's `FetchContent_Declare` that runs
`cmake/scripts/patch_libpng_pngpriv.cmake`. The script reads pngpriv.h,
strips ` || defined(TARGET_OS_MAC)` from the conditional, and writes it
back. Same patch unblocks both the host prebuild *and* the emcc
cross-compile (emcc's libc also lacks `<fp.h>`).

## TODO (not yet authored)

The following are surfaced patches expected to be needed once the host
prebuild succeeds and the cross-compile stage runs. They'll be added to
this directory as the build hits each one.

- **Per-dep emcc compatibility**: abseil, flatbuffers, protobuf,
  sentencepiece, tokenizers-cpp (Rust+cxx bridge), antlr4, llguidance
  (Rust target wasm32-unknown-emscripten), TFLite (likely the largest
  patch surface). Each may need:
  - Pthread guards (emcc pthreads need SharedArrayBuffer + COOP/COEP).
  - Atomic intrinsics fallbacks where wasm32 lacks the host's set.
  - File I/O paths that assume `mmap` or `O_DIRECT` (gate to alternative).
- **Skip miniaudio** entirely on emcc (audio inference unsupported on
  WASM v1).
- **Static archive output** target for `libLiteRtLmC.a`. The upstream
  build produces a `cc_binary(linkshared=True)` (Bazel) or shared-lib
  CMake target; we need a sibling `add_library(... STATIC ...)` aggregator
  for Rust linking, similar to LiteRT's `litert_runtime_c_api_static`.
- **Disable threading-required components** for v1 (kv-cache locks,
  sampler thread pool, etc.) or stub them with single-threaded variants.

## Verified

The `01-cmake-emscripten-support.patch` lets the host prebuild stage
configure + start building under emcmake on macOS arm64 (May 2026,
emsdk 5.0.7). The full host prebuild has not yet completed end-to-end as
of this commit; we're iterating in `/tmp/litert-lm-wasm-spike-build`.
