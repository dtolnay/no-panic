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

    mod test_ref_argument {
        #[no_panic]
        fn demo(ref i: i32) -> i32 {
            *i
        }

        fn main() {
            println!("{}", demo(0));
        }
    }

    mod test_mut_argument {
        #[no_panic]
        fn demo(mut i: i32) -> i32 {
            i += 1;
            i
        }

        fn main() {
            println!("{}", demo(0));
        }
    }

    mod test_ref_mut_argument {
        #[no_panic]
        fn demo(ref mut i: i32) -> i32 {
            *i += 1;
            *i
        }

        fn main() {
            println!("{}", demo(0));
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
