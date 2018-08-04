#[macro_use]
mod compiletest;

assert_no_panic! {
    mod test_readme {
        #[no_panic]
        fn demo(s: &str) -> &str {
            &s[1..]
        }

        fn main() {
            println!("{}", demo("input string"));
        }
    }
}

assert_link_error! {
    mod test_readme {
        #[no_panic]
        fn demo(s: &str) -> &str {
            &s[1..]
        }

        fn main() {
            println!("{}", demo("\u{1f980}input string"));
        }
    }
}
