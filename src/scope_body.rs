use std::{convert::Infallible, pin::Pin};

use futures::Future;

use crate::body::Body;

pub struct ScopeBody<'env, T: 'env, C: 'env>
where
    C: Send,
{
    body: Body<'env, 'env, T, C>,
}

impl<'env, T, C> ScopeBody<'env, T, C>
where
    C: Send,
{
    pub(crate) fn new(body: Body<'env, 'env, T, C>) -> Self {
        Self { body }
    }
}

impl<'env, T> ScopeBody<'env, T, Infallible> {
    pub async fn infallible(self) -> T {
        match self.body.await {
            Ok(v) => v,
            Err(_) => unreachable!(),
        }
    }
}

impl<'env, T, C> Future for ScopeBody<'env, T, C>
where
    C: Send,
{
    type Output = Result<T, C>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.get_mut().body).poll(cx)
    }
}

impl<T, C: Send> Unpin for ScopeBody<'_, T, C> {}
