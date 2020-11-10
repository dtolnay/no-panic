//! Usage for no_panic in libraries.
//! Requires:
//! ```toml
//! [profile.test]
//! opt-level = 1
//! ```
//! Run using:
//! `cargo test --example lib`

use no_panic::{no_panic, may_panic};

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

#[may_panic]
fn demo_may_panic(s: &str) -> &str {
    &s[1..]
}

#[test]
fn no_panic() {
    let _ = demo("aaa");
    let _ = demo_may_panic("aaa");
}
