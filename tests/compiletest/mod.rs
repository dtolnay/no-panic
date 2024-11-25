use std::fs;
use std::process::Command;
use std::sync::Once;

pub fn setup() {
    static BUILD: Once = Once::new();
    BUILD.call_once(|| {
        let status = Command::new("cargo")
            .arg("build")
            .status()
            .expect("failed to build");
        assert!(status.success());
    });
}

pub fn contains_panic(name: &str, code: &str) -> bool {
    let tempdir = tempfile::tempdir().unwrap();

    let prelude = stringify! {
        use no_panic::no_panic;
    };

    let rs = tempdir.path().join(format!("{}.rs", name));
    fs::write(&rs, format!("{}{}", prelude, code)).unwrap();

    let status = Command::new("rustc")
        .arg("--crate-name")
        .arg(name)
        .arg(rs)
        .arg("--edition=2018")
        .arg("-C")
        .arg("opt-level=3")
        .arg("--emit=asm")
        .arg("--out-dir")
        .arg(tempdir.path())
        .arg("--extern")
        .arg(format!(
            "no_panic=target/debug/{prefix}no_panic.{extension}",
            prefix = std::env::consts::DLL_PREFIX,
            extension = std::env::consts::DLL_EXTENSION,
        ))
        .status()
        .expect("failed to execute rustc");
    assert!(status.success());

    let asm = tempdir.path().join(format!("{}.s", name));
    let asm = fs::read_to_string(asm).unwrap();
    asm.contains("detected panic in function")
}

macro_rules! assert_no_panic {
    ($(mod $name:ident { $($content:tt)* })*) => {
        mod no_panic {
            use crate::compiletest;
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
            use crate::compiletest;
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
