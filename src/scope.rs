use std::{
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
};

use futures::{future::BoxFuture, stream::FuturesUnordered, Future, Stream};
use tokio::sync::oneshot::channel;

pub struct Scope<'scope, 'env: 'scope> {
    /// Stores the set of futures that have been spawned.
    ///
    /// This is behind a mutex so that multiple concurrent actors can access it.
    /// A `RwLock` seems better, but `FuturesUnordered is not `Sync` in the case.
    /// But in fact it doesn't matter anyway, because all spawned futures execute
    /// CONCURRENTLY and hence there will be no contention.
    futures: Mutex<Pin<Box<FuturesUnordered<BoxFuture<'scope, ()>>>>>,
    enqueued: Mutex<Vec<BoxFuture<'scope, ()>>>,
    phantom: PhantomData<&'scope &'env ()>,
}

fn is_sync<T: Sync>() {}
fn check_sync<'scope, 'env: 'scope>() {
    if false {
        is_sync::<Scope<'scope, 'env>>();
    }
}

impl<'scope, 'env> Scope<'scope, 'env> {
    pub(crate) fn new() -> Arc<Self> {
        check_sync();
        Arc::new(Self {
            futures: Mutex::new(Box::pin(FuturesUnordered::new())),
            enqueued: Default::default(),
            phantom: Default::default(),
        })
    }

    pub(crate) fn drain(&self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        let mut futures = self.futures.lock().unwrap();
        loop {
            futures.extend(self.enqueued.lock().unwrap().drain(..));

            while let Some(()) = ready!(futures.as_mut().poll_next(cx)) {}

            if self.enqueued.lock().unwrap().is_empty() {
                return Poll::Ready(());
            }
        }
    }

    pub fn spawn<T>(
        &self,
        future: impl Future<Output = T> + Send + 'scope,
    ) -> impl Future<Output = T> + Send + 'scope
    where
        T: std::fmt::Debug + Send + 'scope,
    {
        // Use a tokio channel to communicate result from the *actual* future
        // (which lives in the futures-unordered) and the caller.
        // This is kind of crappy because, ideally, the caller expressing interest
        // in the result of the future would let it run, but that would require
        // more clever coding and I'm just trying to stand something up quickly
        // here. What will happen when caller expresses an interest in result
        // now is that caller will block which should (eventually) allow the
        // futures-unordered to be polled and make progress. Good enough.

        let (tx, rx) = channel();

        self.enqueued.lock().unwrap().push(Box::pin(async {
            let v = future.await;
            let _ = tx.send(v);
        }));

        async move {
            match rx.await {
                Ok(v) => v,
                Err(e) => panic!("unexpected error: {e:?}"),
            }
        }
    }
}
