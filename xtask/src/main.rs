//! Workspace automation entry point. Invoked as `cargo xtask <subcommand>`.
//!
//! The subcommands that require a foreign target use `cross` under the hood,
//! which needs Docker (or Podman via `CROSS_CONTAINER_ENGINE=podman`).
//!
//! Subcommands:
//!   regen-bindings [--target TRIPLE]   Rebuild the committed litert-sys
//!                                      bindings for a target (default: all
//!                                      supported targets).
//!   build-all                          Cross-build litert-sys + litert for
//!                                      every supported target (smoke test).
//!   targets                            Print the supported target list.
//!   help                               Show this message.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

/// Every Rust target we publish bindings for. Keep this in sync with
/// litert-sys/build.rs `target_spec()`.
const TARGETS: &[&str] = &[
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
    "aarch64-linux-android",
    "x86_64-linux-android",
];

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let rest = &args[args.len().min(1)..];

    let result = match cmd {
        "regen-bindings" => regen_bindings(rest),
        "build-all" => build_all(rest),
        "targets" => {
            for t in TARGETS {
                println!("{t}");
            }
            Ok(())
        }
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        other => Err(format!(
            "unknown subcommand `{other}`; try `cargo xtask help`"
        )),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    eprintln!(
        "cargo xtask <subcommand>\n\
         \n\
         Subcommands:\n  \
           regen-bindings [--target TRIPLE]   Rebuild committed litert-sys bindings\n  \
           build-all                          Cross-build for every supported target\n  \
           targets                            List supported Rust target triples\n  \
           help                               Show this message\n\
         \n\
         Cross-target subcommands require `cross` plus a running container engine\n\
         (Docker or Podman). Host-target runs use the native cargo toolchain."
    );
}

// --------------------------------------------------------------------------
// regen-bindings
// --------------------------------------------------------------------------

fn regen_bindings(args: &[String]) -> Result<(), String> {
    let targets = parse_target_arg(args)?;
    for target in targets {
        println!("== regen-bindings {target} ==");
        regen_one(target)?;
    }
    Ok(())
}

fn regen_one(target: &str) -> Result<(), String> {
    let repo = repo_root();
    let host = host_target();

    let mut cmd = if target == host {
        let mut c = Command::new("cargo");
        c.current_dir(&repo)
            .args([
                "build",
                "-p",
                "litert-sys",
                "--features",
                "generate-bindings",
            ])
            .args(["--target", target]);
        if let Ok(libclang) = find_libclang() {
            c.env("LIBCLANG_PATH", libclang);
        }
        c
    } else {
        let mut c = Command::new("cross");
        c.current_dir(&repo)
            .args([
                "build",
                "-p",
                "litert-sys",
                "--features",
                "generate-bindings",
            ])
            .args(["--target", target]);
        c
    };

    run(&mut cmd).map_err(|e| format!("build failed for {target}: {e}"))?;

    // Locate the generated bindings.rs (last-modified under
    // target/<triple>/debug/build/litert-sys-*/out/bindings.rs).
    let build_root = repo.join("target").join(target).join("debug").join("build");
    let generated = newest_bindings_rs(&build_root).ok_or_else(|| {
        format!(
            "could not find generated bindings.rs under {}",
            build_root.display()
        )
    })?;

    let dest = repo
        .join("litert-sys")
        .join("src")
        .join("bindings")
        .join(format!("{target}.rs"));
    fs::copy(&generated, &dest)
        .map_err(|e| format!("copy {} → {}: {e}", generated.display(), dest.display()))?;
    println!("wrote {}", dest.display());
    Ok(())
}

fn newest_bindings_rs(root: &Path) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let walker = fs::read_dir(root).ok()?;
    for entry in walker.flatten() {
        if !entry
            .file_name()
            .to_string_lossy()
            .starts_with("litert-sys-")
        {
            continue;
        }
        let candidate = entry.path().join("out").join("bindings.rs");
        let Ok(meta) = fs::metadata(&candidate) else {
            continue;
        };
        let Ok(mtime) = meta.modified() else { continue };
        match best {
            Some((t, _)) if t >= mtime => {}
            _ => best = Some((mtime, candidate)),
        }
    }
    best.map(|(_, p)| p)
}

// --------------------------------------------------------------------------
// build-all
// --------------------------------------------------------------------------

fn build_all(args: &[String]) -> Result<(), String> {
    let targets = parse_target_arg(args)?;
    for target in targets {
        println!("== build-all {target} ==");
        let host = host_target();
        let tool = if target == host { "cargo" } else { "cross" };
        let mut cmd = Command::new(tool);
        cmd.current_dir(repo_root())
            .args(["build", "--workspace", "--exclude", "xtask"])
            .args(["--target", target]);
        run(&mut cmd).map_err(|e| format!("{tool} build --target {target}: {e}"))?;
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Argument parsing helpers
// --------------------------------------------------------------------------

fn parse_target_arg(args: &[String]) -> Result<Vec<&'static str>, String> {
    let mut iter = args.iter();
    let mut selected: Option<&str> = None;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--target" | "-t" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--target requires a value".to_string())?;
                selected = Some(v.as_str());
            }
            other => return Err(format!("unexpected argument `{other}`")),
        }
    }
    match selected {
        Some(t) => {
            let matched = TARGETS.iter().find(|&&known| known == t).ok_or_else(|| {
                format!(
                    "unsupported target `{t}`; supported: {}",
                    TARGETS.join(", ")
                )
            })?;
            Ok(vec![*matched])
        }
        None => Ok(TARGETS.to_vec()),
    }
}

// --------------------------------------------------------------------------
// Environment utilities
// --------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is xtask/; its parent is the workspace.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn host_target() -> &'static str {
    // Build-time constant — pick the matching triple from TARGETS if possible.
    // Falls back to the CARGO_CFG_* pair at runtime for portability.
    if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_arch = "aarch64", target_os = "linux")) {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(all(target_arch = "x86_64", target_os = "windows")) {
        "x86_64-pc-windows-msvc"
    } else {
        // Unknown host — treat every target as foreign and route through cross.
        "unknown"
    }
}

fn find_libclang() -> Result<String, env::VarError> {
    if let Ok(v) = env::var("LIBCLANG_PATH") {
        return Ok(v);
    }
    for guess in [
        "/opt/homebrew/opt/llvm/lib",
        "/usr/local/opt/llvm/lib",
        "/Library/Developer/CommandLineTools/usr/lib",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib64",
    ] {
        if Path::new(guess).join("libclang.dylib").exists()
            || Path::new(guess).join("libclang.so").exists()
            || Path::new(guess).join("libclang.so.1").exists()
        {
            return Ok(guess.to_string());
        }
    }
    Err(env::VarError::NotPresent)
}

fn run(cmd: &mut Command) -> Result<(), String> {
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let status = cmd.status().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("{cmd:?} exited with {status}"));
    }
    Ok(())
}
