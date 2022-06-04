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

## Frequently asked questions

### Where does the name `moro` come from?

It's the Greek word for "baby". The popular ["trio"](https://trio.readthedocs.io/en/stable/) library uses the term "nursery" to refer to a scope, so I wanted to honor that lineage.

### Why do moro spawns only run concurrently, not parallel?

Parallel moro tasks cannot, with Rust as it is today, be done safely. The full details are in the next question, but the tl;dr is that when a moro scope yields to its caller, the scope is "giving up control" to its caller, and that caller can -- if it chooses -- just forget the scope entirely and stop executing it. This means that if the moro scope has started parallel threads, those threads will go on accessing the caller's data, which can create data races. Not good.

### Isn't running concurrently a huge limitation?

Sort of? Parallel would definitely be nice, but for many async servers, you get parallelism between connections and you don't need to have parallelism *within* a connection. You can also use other mechanisms to get parallelism, but `'static` bounds are required.

### OK, but why do moro spawns only run concurrently, not parallel? Give me the details!

The [`Future::poll`](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll) method permits safe code to "partially advance" a future and then, because a future is an ordinary Rust value, "forget" it (e.g., via [`std::mem::forget`], though there are other ways. This would allow you to create a scope, execute it a few times, and then discard it without running any destructor:

```rust
async fn method() {
    let data = vec![1, 2, 3];
    let some_future = moro::async_scope!(|scope| {
        scope.spawn(async { 
            for d in &data {
                tokio::task::yield_now().await;
            }
        });
    });

    // pseudo-code, we'd have to gin up a context etc:
    std::future::Future::poll(some_future);
    std::future::Future::poll(some_future);
    std::mem::forget(some_future);
    return;
}
```

If moro tasks were running in parallel, there would be no way for us to ensure that the parallel threads spawned inside the scope are stopped before `method` returns. As a result, they would go on accessing the data from `data` even after the stack frame was popped and the data was freed. Bad.

But because moro is limited to concurrency, this is fine. Tasks in the scope only advance when they are polled (they're not parallel) -- so when you "forget" the scope, you simply stop executing the tasks too.

Note that this problem doesn't occur in libraries like [rayon](https://crates.io/crates/rayon). This is because sync code has a capability that async code lacks: a sync function can block its caller. But in async code, under Rust's current model, so long as you "await" something, you are giving up control to your caller and they are free to never poll you again. This means, I believe, that it is not possible to have a "scope" like moro's that safely refers to data *outside* of the scope, since that data is owned by your callers, and you cannot force them not to return. Put another way, async code *can*, by cooperating with the executor, ensure that some future runs to completion. Any call to `tokio::spawn` will do that. But you cannot ensure that your future is *embedded in something else* that runs to completion.

I do not believe parallel execution can be safely enabled without modifying the `Future` trait or Rust in some way. There are various proposals to change the `Future` trait to permit moro to support parallel execution (those same proposals would help for supporting io-uring, DMA, and other features), but the exact path forward hasn't been settled.

