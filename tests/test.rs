#[macro_use]
mod compiletest;

#[rustversion::attr(not(nightly), ignore)]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

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

    mod test_receiver_lifetime_elision {
        struct Buffer {
            bytes: [u8; 24],
        }

        impl Buffer {
            #[no_panic]
            fn demo(&mut self, _s: &str) -> &[u8] {
                &self.bytes[..]
            }
        }

        fn main() {
            let mut buffer = Buffer {
                bytes: [0u8; 24],
            };
            println!("{:?}", buffer.demo(""));
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

    mod test_borrow_from_mut_self {
        struct S {
            data: usize,
        }

        impl S {
            #[no_panic]
            fn get_mut(&mut self) -> &mut usize {
                &mut self.data
            }
        }

        fn main() {
            let mut s = S { data: 0 };
            println!("{}", s.get_mut());
        }
    }

    mod test_self_in_macro {
        struct S {
            data: usize,
        }

        macro_rules! id {
            ($expr:expr) => {
                $expr
            };
        }

        impl S {
            #[no_panic]
            fn get_mut(&mut self) -> usize {
                id![self.data]
            }
        }

        fn main() {
            let mut s = S { data: 0 };
            println!("{}", s.get_mut());
        }
    }

    mod test_self_in_macro_containing_fn {
        pub struct S {
            data: usize,
        }

        macro_rules! emit {
            ($($tt:tt)*) => {
                $($tt)*
            };
        }

        impl S {
            #[no_panic]
            fn get_mut(&mut self) -> usize {
                let _ = emit!({
                    impl S {
                        pub fn f(self) {}
                    }
                });
                self.data
            }
        }

        fn main() {
            let mut s = S { data: 0 };
            println!("{}", s.get_mut());
        }
    }

    mod test_self_with_std_pin {
        use std::pin::Pin;

        pub struct S;

        impl S {
            #[no_panic]
            fn f(mut self: Pin<&mut Self>) {
                let _ = self.as_mut();
            }
        }

        fn main() {}
    }

    mod test_deref_coercion {
        #[no_panic]
        pub fn f(s: &str) -> &str {
            &s
        }

        fn main() {}
    }

    mod test_return_impl_trait {
        use std::io;

        #[no_panic]
        pub fn f() -> io::Result<impl io::Write> {
            Ok(Vec::new())
        }

        fn main() {}
    }

    mod test_conditional_return {
        #[no_panic]
        pub fn f(i: i32) {
            if i < 0 {
                return;
            }
        }

        fn main() {
            println!("{:?}", f(-1));
        }
    }

    mod test_conditional_return_macro {
        macro_rules! return_if_negative {
            ($e:expr) => {
                if $e < 0 {
                    return;
                }
            }
        }

        #[no_panic]
        pub fn f(i: i32) {
            return_if_negative!(i);
        }

        fn main() {
            println!("{:?}", f(-1));
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
