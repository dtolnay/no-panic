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

    mod test_method_in_impl {
        struct S;

        impl S {
            #[no_panic]
            fn demo(self) -> &'static str {
                "test"
            }
        }

        fn main() {
            println!("{}", S.demo());
        }
    }

    mod test_lifetime_elision {
        struct Buffer {
            bytes: [u8; 24],
        }

        #[no_panic]
        fn demo(buffer: &mut Buffer) -> &[u8] {
            &buffer.bytes[..]
        }

        fn main() {
            let mut buffer = Buffer {
                bytes: [0u8; 24],
            };
            println!("{:?}", demo(&mut buffer));
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
