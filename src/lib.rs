use futures::future::BoxFuture;
use scope_data::ScopeData;

#[macro_use]
mod macros;

mod body;
pub mod prelude;
mod result_ext;
mod scope;
mod scope_body;
mod scope_data;
mod spawned;

/// Creates an async scope within which you can spawn jobs.
/// This works much like the stdlib's
/// [`scope`](https://doc.rust-lang.org/std/thread/fn.scope.html)
/// function.
///
/// The scope is not considered to be complete until all jobs
/// within the scope have completed. Jobs spawned within the scope
/// can refer to stack variables that are defined outside
/// the scope.
///
/// # Cancellable vs infallible scopes
///
/// By default, moro scopes support *cancellation*,
/// which means that you can cancel the entire scope by invoking
/// `scope.cancel(v)`. Cancellable scopes return a [`Result`] value
/// whose error type is the type of `v`. If your scope does not use `cancel`,
/// you will get compilation errors because the error type cannot be inferred!
///
/// To avoid this, use the `infallible` method on the scope to convert it
/// into a non-cancellable scope:
///
/// ```rust
/// # futures::executor::block_on(async {
/// let scope = moro::async_scope!(|scope| {/* ... */}).await;
/// # });
/// ```
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
///
/// # Examples
///
/// ## Hello, world
///
/// The following scope spawns one concurrent task which iterates over
/// the vector `v` and sums its values; the [`infallible`][`Scope::infallible`]
/// method is used to indicate that the scope is never cancelled:
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
/// ## Specifying the result type
///
/// You can use the `->` notation to specify the type of value
/// returned by the scope. This is useful to guide type inference
/// sometimes:
///
/// ```rust
/// # futures::executor::block_on(async {
/// let scope = moro::async_scope!(|scope| -> Result<(), u32> {
///     Err(22) // Ok type would otherwise be unconstrained
/// });
/// let result = scope.await;
/// assert_eq!(result, Err(22));
/// # });
/// ```
///
/// ## More
///
/// For more examples, see the [examples] directory in the
/// repository.
///
/// [examples]: https://github.com/nikomatsakis/structtastic/tree/main/examples
///
#[macro_export]
macro_rules! async_scope {
    (|$scope:ident| -> $result:ty { $($body:tt)* }) => {{
        $crate::scope_fn::<$result, _>(|$scope| {
            let future = async { $($body)* };
            Box::pin(future)
        })
    }};
    (|$scope:ident| $body:expr) => {{
        $crate::scope_fn(|$scope| {
            let future = async { $body };
            Box::pin(future)
        })
    }};
}

pub use self::scope::Scope;
pub use self::scope_body::ScopeBody;
pub use self::spawned::Spawned;

/// Creates a new moro scope. Normally, you invoke this through `moro::async_scope!`.
pub fn scope_fn<'env, R, B>(body: B) -> ScopeBody<'env, R>
where
    R: Send + 'env,
    B: for<'scope> FnOnce(Scope<'scope, 'env, R>) -> BoxFuture<'scope, R>,
{
    let scope_data = ScopeData::new();

    // Unsafe: We are letting the body use the `Arc<Scope>` without reference
    // counting. The reference is held by `Body` below. `Body` will not drop
    // the `Arc` until the body_future is dropped, and the output `T` has to outlive
    // `'env` so it can't reference `scope`, so this should be ok.
    let scope_data_ref: *const ScopeData<'_, 'env, R> = &*scope_data;
    let scope: Scope<'_, 'env, R> = Scope::new(unsafe { &*scope_data_ref });

    let body_future = body(scope);

    ScopeBody::new(body::Body::new(body_future, scope_data))
}
