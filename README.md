\#\[no\_panic\]
===============

[![Build Status](https://api.travis-ci.org/dtolnay/no-panic.svg?branch=master)](https://travis-ci.org/dtolnay/no-panic)
[![Latest Version](https://img.shields.io/crates/v/no-panic.svg)](https://crates.io/crates/no-panic)

A Rust attribute macro to require that the compiler prove a function can't ever
panic.

```toml
[dependencies]
no-panic = "0.1"
```

```rust
use no_panic::no_panic;

#[no_panic]
fn demo(s: &str) -> &str {
    &s[1..]
}

fn main() {
    println!("{}", demo("input string"));
}
```

If the function does panic (or the compiler fails to prove that the function
cannot panic), the program fails to compile with a linker error that identifies
the function name. Let's trigger that by passing a string that cannot be sliced
at the first byte:

```rust
fn main() {
    println!("{}", demo("\u{1f980}input string"));
}
```

```console
   Compiling no-panic-demo v0.0.1
error: linking with `cc` failed: exit code: 1
  |
  = note: /no-panic-demo/target/release/deps/no_panic_demo-7170785b672ae322.no_p
anic_demo1-cba7f4b666ccdbcbbf02b7348e5df1b2.rs.rcgu.o: In function `_$LT$no_pani
c_demo..demo..__NoPanic$u20$as$u20$core..ops..drop..Drop$GT$::drop::h72f8f423002
b8d9f':
          no_panic_demo1-cba7f4b666ccdbcbbf02b7348e5df1b2.rs:(.text._ZN72_$LT$no
_panic_demo..demo..__NoPanic$u20$as$u20$core..ops..drop..Drop$GT$4drop17h72f8f42
3002b8d9fE+0x2): undefined reference to `

          ERROR[no-panic]: detected panic in function `demo`
          '
          collect2: error: ld returned 1 exit status
```

The error is not stellar but notice the ERROR\[no-panic\] part at the end that
provides the name of the offending function.

*Compiler support: requires rustc 1.31+*

<br>

### Caveats

- Functions that require some amount of optimization to prove that they do not
  panic may no longer compile in debug mode after being marked `#[no_panic]`.

- Panic detection happens at link time across the entire dependency graph, so
  any Cargo commands that do not invoke a linker will not trigger panic
  detection. This includes `cargo build` of library crates and `cargo check` of
  binary and library crates.

- The attribute is useless in code built with `panic = "abort"`.

If you find that code requires optimization to pass `#[no_panic]`, either make
no-panic an optional dependency that you only enable in release builds, or add a
section like the following to Cargo.toml to enable very basic optimization in
debug builds.

```toml
[profile.dev]
opt-level = 1
```

<br>

### Acknowledgments

The linker error technique is based on [Kixunil]'s crate [`dont_panic`]. Check
out that crate for other convenient ways to require absence of panics.

[Kixunil]: https://github.com/Kixunil
[`dont_panic`]: https://github.com/Kixunil/dont_panic

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
