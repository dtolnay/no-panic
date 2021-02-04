//! Usage for no_panic in libraries.
//! Requires:
//! ```toml
//! [profile.test]
//! opt-level = 1
//! ```
//! Run using:
//! `cargo test --example lib`

use no_panic::no_panic;

/// No link error
#[no_panic]
pub fn demo(s: &str) -> Option<&str> {
    s.get(1..)
}

// /// Link error
// #[no_panic]
// pub fn demo(s: &str) -> Option<&str> {
//     Some(&s[1..])
// }

#[test]
fn no_panic() {
    let _ = demo("aaa");
}
