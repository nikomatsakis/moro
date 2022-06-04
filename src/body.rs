use std::{sync::Arc, task::Poll};

use futures::{future::BoxFuture, Future, FutureExt};

use crate::scope::Scope;

/// The future for a scope's "body".
///
/// It is not considered complete until (a) the body is done and (b) any spawned futures are done.
/// Its result is whatever the body returned.
///
/// # Unsafe contract
///
/// - `body_future` and `result` will be dropped BEFORE `scope`.
pub(crate) struct Body<'scope, 'env: 'scope, R: Send + 'env> {
    body_future: Option<BoxFuture<'scope, R>>,
    result: Option<R>,
    scope: Arc<Scope<'scope, 'env, R>>,
}

impl<'scope, 'env, R> Body<'scope, 'env, R>
where
    R: Send,
{
    /// # Unsafe contract
    ///
    /// - `future` will be dropped BEFORE `scope`
    pub(crate) fn new(future: BoxFuture<'scope, R>, scope: Arc<Scope<'scope, 'env, R>>) -> Self {
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

impl<'scope, 'env, R> Drop for Body<'scope, 'env, R>
where
    R: Send,
{
    fn drop(&mut self) {
        // Fulfill our unsafe contract and ensure we drop other fields
        // before we drop scope.
        self.clear();
    }
}

impl<'scope, 'env, R> Future for Body<'scope, 'env, R>
where
    R: Send,
{
    type Output = R;

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
        //
        // If polling the scope returns `Some`, then the scope was early terminated,
        // so forward that result. Otherwise, the `result` from our body future
        // should be available, so return that.
        match ready!(this.scope.poll_jobs(cx)) {
            Some(v) => return Poll::Ready(v),
            None => match this.result.take() {
                None => Poll::Pending,
                Some(v) => Poll::Ready(v),
            },
        }
    }
}

impl<R: Send> Unpin for Body<'_, '_, R> {}
