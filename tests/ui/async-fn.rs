use no_panic::no_panic;

#[no_panic]
async fn f() {
    g().await;
}

async fn g() {}

fn main() {}
