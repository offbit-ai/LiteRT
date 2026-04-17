# litertlm

Safe Rust bindings to Google's LiteRT-LM — on-device LLM inference.

```toml
[dependencies]
litertlm = "0.2"
```

```rust
use litertlm::{Backend, Engine, EngineSettings, SamplerParams};

let engine = Engine::new(
    EngineSettings::new("model.litertlm")
        .backend(Backend::Gpu)
        .max_num_tokens(512),
)?;

let mut session = engine.create_session(
    SamplerParams::default().top_k(40).temperature(0.7),
)?;

let response = session.generate("Explain Rust lifetimes")?;
println!("{response}");
# Ok::<(), litert_lm::Error>(())
```

See the [workspace README](https://github.com/offbit-ai/LiteRT#readme) for
the full story.

## License

Apache-2.0; see [NOTICE](https://github.com/offbit-ai/LiteRT/blob/main/NOTICE).
