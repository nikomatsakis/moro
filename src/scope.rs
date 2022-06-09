use std::{
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
};

use futures::{future::BoxFuture, stream::FuturesUnordered, Future, Stream};

use crate::{scope_data::ScopeData, Spawned};

/// Represents a moro "async scope". See the [`async_scope`][crate::async_scope] macro for details.
pub struct Scope<'scope, 'env: 'scope, R: Send + 'env> {
    data: &'scope ScopeData<'scope, 'env, R>,
}

impl<'scope, 'env, R> Copy for Scope<'scope, 'env, R> where R: Send {}

impl<'scope, 'env, R> Clone for Scope<'scope, 'env, R>
where
    R: Send,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'scope, 'env, R> Scope<'scope, 'env, R>
where
    R: Send,
{
    /// Create a `Scope` from a `ScopeData` that is valid for `'scope`.
    pub(crate) fn new(data: &'scope ScopeData<'scope, 'env, R>) -> Self {
        Self { data }
    }

    /// Terminate the scope immediately -- all existing jobs will stop at their next await point
    /// and never wake up again. Anything on their stacks will be dropped. This is most useful
    /// for propagating errors, but it can be used to propagate any kind of final value (e.g.,
    /// perhaps you are searching for something and want to stop once you find it.)
    ///
    /// This returns a future that you should await, but it will never complete
    /// (because you will never be reawoken). Since termination takes effect at the next
    /// await point, awaiting the returned future ensures that your current future stops
    /// immediately.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # futures::executor::block_on(async {
    /// let result = moro::async_scope!(|scope| {
    ///     scope.spawn(async { /* ... */ });
    ///
    ///     // Calling `scope.terminate` here will terminate the async
    ///     // scope and use the string `"cancellation-value"` as
    ///     // the final value.
    ///     let result: () = scope.terminate("cancellation-value").await;
    ///     unreachable!() // this code never executes
    /// }).await;
    ///
    /// assert_eq!(result, "cancellation-value");
    /// # });
    /// ```
    pub fn terminate<T>(self, value: R) -> impl Future<Output = T> + 'scope
    where
        T: 'scope + Send,
    {
        self.data.terminate(value)
    }

    /// Spawn a job that will run concurrently with everything else in the scope.
    /// The job may access stack fields defined outside the scope.
    /// The scope will not terminate until this job completes or the scope is cancelled.
    pub fn spawn<T>(
        self,
        future: impl Future<Output = T> + Send + 'scope,
    ) -> Spawned<impl Future<Output = T> + Send>
    where
        T: 'scope + Send,
    {
        self.data.spawn(future)
    }
}
