#[async_rt::main]
async fn main() {
    let val = async_rt::task::spawn(async { "Alice" }).await.unwrap();

    println!("Hello, world, {}", val);
}
