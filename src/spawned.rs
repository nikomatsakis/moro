use std::pin::Pin;

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
