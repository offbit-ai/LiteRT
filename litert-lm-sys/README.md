# litert-lm-sys

Raw FFI bindings to Google's LiteRT-LM C engine API.

Most users want the safe wrappers in [`litertlm`](https://crates.io/crates/litertlm).
At build time, `build.rs` downloads a pinned `libLiteRtLmC.{so,dylib}` from
our mirrored GitHub release and SHA-256-verifies it.

See the [workspace README](https://github.com/offbit-ai/LiteRT#readme).

## License

Apache-2.0; see [NOTICE](https://github.com/offbit-ai/LiteRT/blob/main/NOTICE).
