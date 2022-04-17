use moro;

#[tokio::main]
pub async fn main() {
    let value = 22;
    let scope = moro::async_scope!(|scope| {
        let future1 = scope.spawn(async {
            let future2 = scope.spawn(async {
                value // access stack values that outlive scope
            });

            let v = future2.await * 2;
            v
        });

        let v = future1.await * 2;
        v
    })
    .infallible();
    let result = scope.await;
    println!("{result:?}");
}
