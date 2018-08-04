extern crate tempfile;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Once;

pub fn setup() {
    static BUILD: Once = Once::new();
    BUILD.call_once(|| {
        if !Path::new("target/debug/libno_panic.so").exists() {
            let status = Command::new("cargo")
                .arg("build")
                .status()
                .expect("failed to build");
            assert!(status.success());
        }
    });
}

pub fn contains_panic(name: &str, code: &str) -> bool {
    let tempdir = tempfile::tempdir().unwrap();

    let prelude = stringify! {
        #![feature(use_extern_macros)]
        extern crate no_panic;
        use no_panic::no_panic;
    };

    let rs = tempdir.path().join(format!("{}.rs", name));
    fs::write(&rs, format!("{}{}", prelude, code)).unwrap();

    let status = Command::new("rustc")
        .arg("--crate-name")
        .arg(name)
        .arg(rs)
        .arg("-C")
        .arg("opt-level=3")
        .arg("--emit=asm")
        .arg("--out-dir")
        .arg(tempdir.path())
        .arg("--extern")
        .arg("no_panic=target/debug/libno_panic.so")
        .status()
        .expect("failed to execute rustc");
    assert!(status.success());

    let asm = tempdir.path().join(format!("{}.s", name));
    let asm = fs::read_to_string(asm).unwrap();
    asm.contains("RUST_PANIC_IN_FUNCTION")
}

macro_rules! assert_no_panic {
    ($(mod $name:ident { $($content:tt)* })*) => {
        mod no_panic {
            use compiletest;
            $(
                #[test]
                fn $name() {
                    compiletest::setup();
                    let name = stringify!($name);
                    let content = stringify!($($content)*);
                    assert!(!compiletest::contains_panic(name, content));
                }
            )*
        }
    };
}

macro_rules! assert_link_error {
    ($(mod $name:ident { $($content:tt)* })*) => {
        mod link_error {
            use compiletest;
            $(
                #[test]
                fn $name() {
                    compiletest::setup();
                    let name = stringify!($name);
                    let content = stringify!($($content)*);
                    assert!(compiletest::contains_panic(name, content));
                }
            )*
        }
    };
}
