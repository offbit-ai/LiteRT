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

### 1.3 — 1.6 CoreFoundation framework on Apple host

protobuf v6.31.1 + sentencepiece + tflite + litert all build CLI tools
that link `libabsl_time_zone.a`, which references `CFTimeZoneGetName`,
`CFStringGetMaximumSizeForEncoding`, etc. on Apple. None of those CMake
files add `-framework CoreFoundation` to their executables' link command,
so the link fails with "Undefined symbols for architecture arm64".

Fix: in each affected `cmake/packages/<dep>/<dep>.cmake`, add to
`ExternalProject_Add`'s `CMAKE_ARGS`:
```cmake
"$<$<PLATFORM_ID:Darwin>:-DCMAKE_EXE_LINKER_FLAGS=-framework CoreFoundation>"
```
The generator expression evaluates to empty on non-Apple, so the patch
is a no-op on Linux/Windows.

Applied to: protobuf, sentencepiece, tflite, litert.

## TODO (next walls expected, not yet patched)

After the above patches the host prebuild progresses to ~46% (sentencepiece
done, working on tokenizers-cpp). Next surfaced wall:

- **tokenizers-cpp's Rust onig_sys env leak**: the Rust crate's `cc-rs`
  wrapper picks up `CC=emcc` from the inherited env, then fails with
  "stdlib.h not found" because emcc's clang doesn't have a host sysroot.
  Fix: explicitly set `CC_aarch64_apple_darwin` /
  `CC_x86_64_unknown_linux_gnu` (per-target Rust env vars) to native
  clang in the orchestrator's BUILD_COMMAND env wrapper. Or pass
  `CARGO_TARGET_<TARGET>_LINKER` and friends.

Beyond that, expected (untested):

- **Cross-compile stage (emcc)**: every C++ dep needs to compile cleanly
  under emcc — pthread-dependent code, atomic intrinsics, mmap-based
  file I/O all common pain points. Likely 10-20 more incremental patches
  before all 22 deps build clean to wasm32-unknown-emscripten.
- **`miniaudio`**: skip entirely on emcc (audio inference is unsupported
  on WASM v1; miniaudio's Web/Emscripten path adds complexity we don't
  need yet).
- **Static archive aggregator** for `libLiteRtLmC.a` — upstream produces
  a shared-lib CMake target; we need an `add_library(... STATIC ...)`
  sibling for Rust linking, mirroring LiteRT's
  `litert_runtime_c_api_static`.
- **Threading**: disable thread-pool dispatch in samplers / kv-cache for
  single-threaded WASM v1, or wrap behind `#ifdef __EMSCRIPTEN__`
  fallbacks.

## Verified (status: ~93% through host prebuild, 0.4.0 spike)

After 9 distinct fixes consolidated into `01-cmake-emscripten-support.patch`
+ 2 helper scripts (libpng pngpriv.h fix, litert_cc_options restore),
the host prebuild progresses through:

```
✅ kissfft, opencl_headers, zlib, libpng16, antlr4-runtime
✅ absl_external, flatbuffers_external, gtest_external, re2_external
✅ protobuf_external (protoc, protoc-gen-upb*) — needs CF + UPB toolchain unset
✅ sentencepiece_external (all spm_* CLI tools) — needs CF
✅ tokenizers-cpp_external (Rust onig_sys + cxx bridge) — needs CC_<triple>
✅ tflite_external — needs CF + XNNPACK_ENABLE_KLEIDIAI=OFF
✅ litert_external compile — needs CF + cc/options/CMakeLists.txt restore
❌ litert_external link of libLiteRt.dylib — DUPLICATE SYMBOL ERRORS
```

The duplicate-symbol failure is the 10th distinct wall. Stopping the
auto-spike here pending dedicated investigation. Symptoms:

```
duplicate symbol 'google::protobuf::internal::TcParser::FastV32R1' in:
    libprotobuf-lite.a[generated_message_tctable_lite.cc.o]
    libprotobuf.a[generated_message_tctable_lite.cc.o]
duplicate symbol 'tflite::profiling::Node::Node(...)' in:
    libmodel_runtime_info_proto.a[model_runtime_info.pb.cc.o]
    libtflite_profiling.a[model_runtime_info.pb.cc.o]
```

Both libprotobuf-lite + libprotobuf are getting linked into libLiteRt.dylib
even though libprotobuf-lite is supposed to be a subset of libprotobuf
(should pick one). Same for tflite_profiling vs the locally-generated
model_runtime_info_proto. Likely a `target_link_libraries()` chain in
the litert subproject's CMakeLists redundantly references both.

Working around with `-Wl,-multiply_defined,suppress` is no longer
viable on modern macOS (it's marked obsolete by ld). The fix needs
careful pruning of the link graph in `litert/c/CMakeLists.txt` /
`litert/cc/CMakeLists.txt` / `litert/runtime/CMakeLists.txt`.

Investigation deferred to a dedicated multi-day session.

Tested with emsdk 5.0.7, macOS arm64, May 2026.

Build command pattern at checkpoint:
```bash
git apply 01-cmake-emscripten-support.patch
mkdir -p cmake/scripts && cp .../patch_libpng_pngpriv.cmake cmake/scripts/
emcmake cmake -B build-wasm -S . \
    -DCMAKE_BUILD_TYPE=Release \
    -DLITERTLM_TOOLCHAIN_ARGS="-DCMAKE_BUILD_TYPE=Release"
emmake cmake --build build-wasm -j 8
# → fails with `ld: library 'litert_cc_options' not found` during
#   litert_external link stage at ~93% of overall progress
```

## Realistic estimate to finish 0.4.0

Based on the iteration rate observed in this spike (~5-10 minutes per
"author fix → verify it didn't break" cycle, plus ~10-15 minutes per
build iteration to surface the next wall):

- **Remaining host prebuild fixes**: 1-3 more walls expected after
  litert_cc_options resolves. Estimate ~1-2 days.
- **Cross-compile (emcc) stage**: not yet started. Each of the 22
  C++/Rust deps may need pthread/atomic/file-I/O patches. Estimate
  ~2-3 weeks.
- **`Conversation::send_raw_stream` async rewrite** (`Mutex`+`Condvar`
  blocking wait deadlocks JS event loop): ~3-5 days.
- **Browser harness** (HTML+JS, IndexedDB model cache, fetch streaming
  to DOM): ~3-5 days.
- **Qwen smoke test + debugging**: ~1 week.

**Total realistic remaining**: 4-6 weeks of focused work *after* this
checkpoint. The patches in this directory are durable progress —
re-applying them on the same upstream commit reproduces the same
~70-75% prebuild state.
