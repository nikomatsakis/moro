# moro

Experiments with structured concurrency in Rust

## TL;DR

Similar to rayon, moro let's you create a *scope* using a `moro::async_scope!` macro.
Within this scope, you can spawn jobs that can access stack data defined outside the scope:

```rust
let value = 22;
let result = moro::async_scope!(|scope| {
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
.infallible()
.await;
eprintln!("{result}"); // prints 88
```

## Stack access, core scope API

Invoke `moro::async_scope!(|scope| ...)` and you get back a future
for the overall scope that you can await to start up the scope.

Within the scope body (`...`) you can invoke `scope.spawn(async { ... })` to spawn a job. 
This job must terminate before the scope itself is considered completed. 
The result of `scope.spawn` is a future whose result is the result returned by the job.

## Cancellation

The `async_scope!` macro itself optionally supports *cancellation*.
When you cancel a scope, all the work within that scope immediately
terminates and all futures are dropped (in the future, we'll support
async drop to allow them to do work). 

To reflect cancellation, the future you get back from `async_scope`
has type `Result<T, C>`, where `T` is the type your scope body returns
and `C` is the *cancellation* type. You cancel a scope by invoking
`scope.cancel(c)`.

If your scope is never cancelled, you can use the `infallible` method
to create an infallible async scope (whose result is just `T`).
An example is shown in the [hello world](examples/hello_world.rs) example.

An example that uses cancellation is shown in [monitor](examples/monitor.rs) --
in this example, several jobs are spawned which all examine one integer from
the input. If any integers are negative, the entire scope is canceled.

## Future work: Integrating with rayon-like iterators

I want to do this. :) 