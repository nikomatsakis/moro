use std::pin::Pin;

use futures::Future;
use pin_project::pin_project;

use crate::body::Body;

#[pin_project]
pub struct ScopeBody<'env, R: 'env, F>
where
    R: Send,
    F: Future<Output = R>,
{
    #[pin]
    body: Body<'env, 'env, R, F>,
}

impl<'env, R, F> ScopeBody<'env, R, F>
where
    R: Send,
    F: Future<Output = R>,
{
    pub(crate) fn new(body: Body<'env, 'env, R, F>) -> Self {
        Self { body }
    }
}

impl<'env, R, F> Future for ScopeBody<'env, R, F>
where
    R: Send,
    F: Future<Output = R>,
{
    type Output = R;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.project().body).poll(cx)
    }
}
