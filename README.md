# moro

Experiments with structured concurrency in Rust

## Desired usage patterns

### Stack access, core scope API

```
let value = 22;
let result = moro::scope(|scope| async {
    let future1 = scope.spawn(async {
        let future2 = scope.spawn(async {
            value // access stack values that outlive scope
        });

        future2.await * 2
    });

    future1.await * 2
});
let result = result.await;
println!("{result}"); // prints 88
```

when the "main future" completes, the scope will wait.

what we can really get today might be more like


```
let value = 22;
let result = moro::scope!(|scope| {
    let future1 = scope.spawn(async {
        let future2 = scope.spawn(async {
            value // access stack values that outlive scope
        });

        future2.await * 2
    });

    future1.await * 2
});
let result = result.await;
println!("{result}"); // prints 88
```


### Integrating with async iterators

Define a

```rust
trait AsyncIter {
    type Item;
    async fn next(&mut self) -> Option<Self::Item>;
}
```

### Cancellation

* Assuming async-drop capabilities for now
    * Modeled by an explicit `async fn cancel(self)` on the scope
    * Panic if the scope is neither awaited nor canceled?
    * Or synchronously block?
    * Or just drop all nested futures for now?

### Handles



### Panics

* 

