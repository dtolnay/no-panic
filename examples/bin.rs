//! Usage for no_panic in binaries.
//! Requires:
//! ```toml
//! [profile.dev]
//! opt-level = 1
//! ```
//! Run using:
//! `cargo run --example bin`

use no_panic::no_panic;

/// No link error
#[no_panic]
fn demo(s: &str) -> Option<&str> {
    s.get(1..)
}

// /// Link error
// #[no_panic]
// pub fn demo(s: &str) -> Option<&str> {
//     Some(&s[1..])
// }

fn main() {
    println!("{}", demo("input string").unwrap());
}
