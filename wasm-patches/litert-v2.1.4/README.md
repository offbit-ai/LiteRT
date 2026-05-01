# LiteRT v2.1.4 WASM patches

Patches applied to `google-ai-edge/LiteRT` v2.1.4 source tree before
cross-compiling to `wasm32-unknown-emscripten` via `emcmake`. The CI
workflow `.github/workflows/build-litert-wasm.yml` applies these
automatically.

## 01-cmake-emscripten-support.patch

Two related changes to `litert/CMakeLists.txt`:

1. **FetchContent OpenCL headers** when targeting emscripten. The runtime's
   `open_cl_memory.h`/`open_cl_memory.cc` unconditionally `#include <CL/cl.h>`,
   even though the actual OpenCL impl is gated behind
   `LITERT_HAS_OPENCL_SUPPORT` and never executes on WASM. Without these
   headers the compile fails. Khronos pin: `v2024.05.08`.

2. **`FLATBUFFERS_LOCALE_INDEPENDENT=0` globally** for emscripten. Flatbuffers
   v25.9.23's CMake disables `LOCALE_INDEPENDENT` because emcc lacks
   `strtof_l`, so its util.cpp doesn't define `ClassicLocale::instance_`.
   But `flatbuffers/base.h` auto-detects `__unix__` (which emcc does set)
   and tries to use the locale path, causing a link error in TFLite/LiteRT
   `.cc` files that include flatbuffers headers. Forcing `=0` everywhere
   keeps the consumer code aligned with the library.

## How to apply locally

```bash
git clone --depth=1 --branch=v2.1.4 https://github.com/google-ai-edge/LiteRT.git
cd LiteRT
git apply /path/to/wasm-patches/litert-v2.1.4/01-cmake-emscripten-support.patch
emcmake cmake -S litert -B build-wasm \
  -DLITERT_ENABLE_GPU=OFF -DLITERT_ENABLE_NPU=OFF \
  -DLITERT_DISABLE_KLEIDIAI=ON -DLITERT_BUILD_TESTS=OFF \
  -DCMAKE_BUILD_TYPE=Release
emmake cmake --build build-wasm --target litert_runtime_c_api_shared_lib -j 8
```

Output: ~120 static archives in `build-wasm/`, totaling ~15 MB.

## Verified

End-to-end smoke test on macOS arm64 host (May 2026, emsdk 5.0.7,
clang 21):

- `cargo build -p litert --example add_wasm --target wasm32-unknown-emscripten`
  produces a 12 MB `.wasm` + 177 KB Emscripten JS glue.
- All linker symbols resolve cleanly via wasm-ld.

## Upstream PR candidates

Both changes are minimal and behave identically on non-emscripten targets
(due to the `if(EMSCRIPTEN)` guard). Worth submitting upstream as
`feat: support cross-compile to wasm32-unknown-emscripten`.
