use std::future::Future;

use futures::future::BoxFuture;

#[macro_use]
mod macros;
mod body;
mod scope;

/// Creates an async scope within which you can spawn jobs.
///
/// The scope is not considered to be complete until all jobs
/// within the scope have completed. Jobs spawned within the scope
/// can refer to stack variables that are defined outside
/// the scope.
///
/// # Example
///
/// ```rust
/// # futures::executor::block_on(async {
/// let v = vec![1, 2, 3, 5];
/// let scope = moro::async_scope!(|scope| {
///     let job = scope.spawn(async {
///         let r: i32 = v.iter().sum();
///         r
///     });
///     job.await * 2
/// });
/// let result = scope.await;
/// assert_eq!(result, 22);
/// # });
/// ```
///
/// For more examples, see the [examples] directory in the
/// repository.
///
/// [examples]: https://github.com/nikomatsakis/structtastic/tree/main/examples
///
/// # Access to stack variables
///
/// Futures spawned inside an async scope can refer to
/// stack variables defined outside the scope:
///
/// ```rust
/// # futures::executor::block_on(async {
/// let r = 22;
/// let scope = moro::async_scope!(|scope| {
///     // OK to refer to `r` here
///     scope.spawn(async { r }).await
/// });
/// let result = scope.await;
/// assert_eq!(result, 22);
/// # });
/// ```
///
/// but when you spawn a future, that future cannot refer to
/// stack variables defined *inside* the scope (except its own
/// variables, of course):
///
/// ```rust,compile_fail,E0373
/// # futures::executor::block_on(async {
/// let scope = moro::async_scope!(|scope| {
///     let r = 22;
//
///     // NOT ok to refer to `r` now, because `r`
///     // is defined inside the scope
///     scope.spawn(async { r }).await
/// });
/// let result = scope.await;
/// assert_eq!(result, 22);
/// # });
/// ```
#[macro_export]
macro_rules! async_scope {
    (|$scope:ident| $body:expr) => {{
        $crate::scope_fn(|$scope| {
            let future = async { $body };
            Box::pin(future)
        })
    }};
}

pub use self::scope::Scope;

/// Creates a new moro scope. Normally, you invoke this through `moro::async_scope!`.
pub fn scope_fn<'env, T>(
    body: impl for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> BoxFuture<'scope, T>,
) -> impl Future<Output = T> + 'env
where
    T: Unpin + 'env,
{
    let scope = Scope::new();

    // Unsafe: We are letting the body use the `Arc<Scope>` without reference
    // counting. The reference is held by `Body` below. `Body` will not drop
    // the `Arc` until the body_future is dropped, and the output `T` has to outlive
    // `'env` so it can't reference `scope`, so this should be ok.
    let scope_ref: *const Scope = &*scope;
    let body_future = body(unsafe { &*scope_ref });

    body::Body::new(body_future, scope)
}
