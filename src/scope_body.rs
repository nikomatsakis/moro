use std::pin::Pin;

use futures::Future;

use crate::body::Body;

pub struct ScopeBody<'env, R: 'env>
where
    R: Send,
{
    body: Body<'env, 'env, R>,
}

impl<'env, R> ScopeBody<'env, R>
where
    R: Send,
{
    pub(crate) fn new(body: Body<'env, 'env, R>) -> Self {
        Self { body }
    }
}

impl<'env, R> Future for ScopeBody<'env, R>
where
    R: Send,
{
    type Output = R;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.get_mut().body).poll(cx)
    }
}

impl<R: Send> Unpin for ScopeBody<'_, R> {}
