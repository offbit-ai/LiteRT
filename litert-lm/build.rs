fn main() {
    println!("cargo:rerun-if-env-changed=DEP_LITERTLM_LIB_DIR");
    if let Ok(dir) = std::env::var("DEP_LITERTLM_LIB_DIR") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
}
