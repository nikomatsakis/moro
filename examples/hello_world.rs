#![feature(async_closure)]

#[tokio::main]
pub async fn main() {
    let value = 22;
    let scope = moro::scope(async |scope| {
        let future1 = scope.spawn(async {
            let future2 = scope.spawn(async {
                value // access stack values that outlive scope
            });

            let v = future2.await * 2;
            v
        });

        let v = future1.await * 2;
        v
    });
    let result = scope.await;
    println!("{result:?}");
    println!("{value:?}");
}
