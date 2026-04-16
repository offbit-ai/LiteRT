# litert

Safe, zero-friction Rust bindings to Google's LiteRT 2.x — the on-device
machine-learning runtime formerly known as TensorFlow Lite.

```toml
[dependencies]
litert = "0.1"
```

```rust
use litert::{CompilationOptions, CompiledModel, Environment, Model, TensorBuffer};

let env      = Environment::new()?;
let model    = Model::from_file("mobilenet.tflite")?;
let options  = CompilationOptions::new()?;
let compiled = CompiledModel::new(env, model, &options)?;
// ... fill input buffers, compiled.run(...), read output buffers ...
# Ok::<(), litert::Error>(())
```

See the [workspace README](https://github.com/offbit-ai/LiteRT#readme) for
quick start, platform support, GPU acceleration examples, and contributor
docs.

## License

Licensed under the Apache License, Version 2.0; see
[`LICENSE`](https://github.com/offbit-ai/LiteRT/blob/main/LICENSE) and
[`NOTICE`](https://github.com/offbit-ai/LiteRT/blob/main/NOTICE) for
upstream attribution.
