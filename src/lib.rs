use std::future::Future;

use futures::future::BoxFuture;

#[macro_use]
mod macros;
mod body;
mod scope;

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
