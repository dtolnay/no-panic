use std::env;
use std::process::Command;
use std::str;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let Some(rustc) = rustc_minor_version() else {
        return;
    };

    if rustc >= 80 {
        println!("cargo:rustc-check-cfg=cfg(exhaustive)");
        println!("cargo:rustc-check-cfg=cfg(no_unsafe_extern_blocks)");
    }

    if rustc < 82 {
        // https://blog.rust-lang.org/2024/10/17/Rust-1.82.0.html#safe-items-with-unsafe-extern
        println!("cargo:rustc-cfg=no_unsafe_extern_blocks");
    }
}

fn rustc_minor_version() -> Option<u32> {
    let rustc = env::var_os("RUSTC").unwrap();
    let output = Command::new(rustc).arg("--version").output().ok()?;
    let version = str::from_utf8(&output.stdout).ok()?;
    let mut pieces = version.split('.');
    if pieces.next() != Some("rustc 1") {
        return None;
    }
    pieces.next()?.parse().ok()
}
