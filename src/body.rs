use std::{future::Future, pin::Pin, sync::Arc, task::Poll};

use pin_project::{pin_project, pinned_drop};

use crate::scope::Scope;

/// The future for a scope's "body".
///
/// It is not considered complete until (a) the body is done and (b) any spawned futures are done.
/// Its result is whatever the body returned.
///
/// # Unsafe contract
///
/// - `body_future` and `result` will be dropped BEFORE `scope`.
#[pin_project(PinnedDrop)]
pub(crate) struct Body<'scope, 'env: 'scope, R, F>
where
    R: Send,
    R: 'env,
{
    #[pin]
    body_future: Option<F>,
    result: Option<R>,
    scope: Arc<Scope<'scope, 'env, R>>,
}

impl<'scope, 'env, R, F> Body<'scope, 'env, R, F>
where
    R: Send,
{
    /// # Unsafe contract
    ///
    /// - `future` will be dropped BEFORE `scope`
    pub(crate) fn new(future: F, scope: Arc<Scope<'scope, 'env, R>>) -> Self {
        Self {
            body_future: Some(future),
            result: None,
            scope,
        }
    }

    fn clear(self: Pin<&mut Self>) {
        let mut this = self.project();
        this.body_future.set(None);
        this.result.take();
        this.scope.clear();
    }
}

#[pinned_drop]
impl<'scope, 'env, R, F> PinnedDrop for Body<'scope, 'env, R, F>
where
    R: Send,
{
    fn drop(self: Pin<&mut Self>) {
        // Fulfill our unsafe contract and ensure we drop other fields
        // before we drop scope.
        self.clear();
    }
}

impl<'scope, 'env, R, F> Future for Body<'scope, 'env, R, F>
where
    R: Send,
    F: Future<Output = R>,
{
    type Output = R;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        // If the body is not yet finished, poll that. Once it becomes finished,
        // we will update `this.result.
        if let Some(body_future) = this.body_future.as_mut().as_pin_mut() {
            match body_future.poll(cx) {
                Poll::Ready(r) => {
                    *this.result = Some(r);
                    this.body_future.set(None);
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
