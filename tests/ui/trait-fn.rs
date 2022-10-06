use no_panic::no_panic;

trait Trait {
    #[no_panic]
    fn f();
}

impl Trait for i32 {
    fn f() {}
}

fn main() {}
