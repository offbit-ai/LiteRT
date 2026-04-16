# litert-sys

Raw FFI bindings to Google's LiteRT 2.x C API.

This is the unsafe FFI layer — most users want the safe wrappers in the
[`litert`](https://crates.io/crates/litert) crate instead. Pre-generated
bindings ship for every supported target so no `libclang` is required on
end-user machines.

At build time, `litert-sys/build.rs` downloads the pinned LiteRT prebuilt
shared libraries (Google Git LFS for desktop, Google Maven AAR for Android),
SHA-256-verifies them against in-crate checksums, and caches them under
`$XDG_CACHE_HOME/litert-sys/`. Every target flows through one pure-Rust
download path — no CMake, no Bazel, no shell scripts.

See the [workspace README](https://github.com/offbit-ai/LiteRT#readme) for
the full feature matrix, platform support, and contributor guide.

## License

Licensed under the Apache License, Version 2.0. The vendored C headers and
the downloaded prebuilt binaries are Apache-2.0 © Google LLC; see
[`NOTICE`](https://github.com/offbit-ai/LiteRT/blob/main/NOTICE).
