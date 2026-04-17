#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    improper_ctypes,
    clippy::useless_transmute,
    clippy::transmute_int_to_bool,
    clippy::missing_safety_doc
)]
#![doc = "Raw FFI bindings to LiteRT-LM C API. Use the safe `litert-lm` crate for idiomatic Rust."]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
