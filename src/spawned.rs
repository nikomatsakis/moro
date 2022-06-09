use std::pin::Pin;

use crate::prelude::*;
use crate::Scope;
use futures::Future;

pub struct Spawned<F> {
    f: F,
}

impl<F> Spawned<F> {
    pub(crate) fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Future for Spawned<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        unsafe {
            let f = Pin::new_unchecked(&mut self.get_unchecked_mut().f);
            F::poll(f, cx)
        }
    }
}

impl<F, O, E> Spawned<F>
where
    F: Future<Output = Result<O, E>> + Send,
    O: Send,
    E: Send,
{
    pub fn or_cancel<'scope, 'env, T>(
        self,
        scope: Scope<'scope, 'env, Result<T, E>>,
    ) -> impl Future<Output = O> + 'scope
    where
        T: Send,
        O: 'scope,
        F: 'scope,
    {
        scope.spawn(async move { self.await.unwrap_or_cancel(scope).await })
    }
}
