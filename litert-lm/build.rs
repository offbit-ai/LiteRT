fn main() {
    // Forward rpath from litert-lm-sys (the LM engine dylib).
    println!("cargo:rerun-if-env-changed=DEP_LITERTLM_LIB_DIR");
    if let Ok(dir) = std::env::var("DEP_LITERTLM_LIB_DIR") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }

    // Forward rpath from litert-sys (the base LiteRT runtime + plugins like
    // libGemmaModelConstraintProvider.dylib that libLiteRtLmC depends on).
    println!("cargo:rerun-if-env-changed=DEP_LITERT_LIB_DIR");
    if let Ok(dir) = std::env::var("DEP_LITERT_LIB_DIR") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
}
