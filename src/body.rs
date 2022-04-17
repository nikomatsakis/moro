use std::{convert::Infallible, sync::Arc, task::Poll};

use futures::{future::BoxFuture, Future, FutureExt};

use crate::scope::Scope;

/// The future for a scope's "body".
///
/// It is not considered complete until (a) the body is done and (b) any spawned futures are done.
/// Its result is whatever the body returned.
///
/// # Notes
///
/// The `T: Unpin` requirement seems unfortunate but I am not smart enough at this moment
/// to think about how to avoid it.
///
/// # Unsafe contract
///
/// - `body_future` and `result` will be dropped BEFORE `scope`.
pub(crate) struct Body<'scope, 'env: 'scope, T: 'env + Unpin, C: Send + 'env> {
    body_future: Option<BoxFuture<'scope, T>>,
    result: Option<T>,
    scope: Arc<Scope<'scope, 'env, C>>,
}

impl<'scope, 'env, T, C> Body<'scope, 'env, T, C>
where
    T: Unpin,
    C: Send,
{
    /// # Unsafe contract
    ///
    /// - `future` will be dropped BEFORE `scope`
    pub(crate) fn new(future: BoxFuture<'scope, T>, scope: Arc<Scope<'scope, 'env, C>>) -> Self {
        Self {
            body_future: Some(future),
            result: None,
            scope,
        }
    }

    fn clear(&mut self) {
        self.body_future.take();
        self.result.take();
        self.scope.clear();
    }
}

impl<'scope, 'env, T, C> Drop for Body<'scope, 'env, T, C>
where
    T: Unpin,
    C: Send,
{
    fn drop(&mut self) {
        // Fulfill our unsafe contract and ensure we drop other fields
        // before we drop scope.
        self.clear();
    }
}

impl<'scope, 'env, T, C> Future for Body<'scope, 'env, T, C>
where
    T: Unpin,
    C: Send,
{
    type Output = Result<T, C>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // If the body is not yet finished, poll that. Once it becomes finished,
        // we will update `this.result.
        if let Some(body_future) = &mut this.body_future {
            match body_future.poll_unpin(cx) {
                Poll::Ready(r) => {
                    this.result = Some(r);
                    this.body_future = None;
                }
                Poll::Pending => {}
            }
        }

        // Check if the scope is ready.
        ready!(this.scope.drain(cx))?;

        match this.result.take() {
            None => Poll::Pending,
            Some(v) => Poll::Ready(Ok(v)),
        }
    }
}
