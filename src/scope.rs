use std::{
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
};

use futures::{future::BoxFuture, stream::FuturesUnordered, Future, Stream};
use tokio::sync::oneshot::channel;

pub struct Scope<'scope, 'env: 'scope, C: Send + 'env> {
    /// Stores the set of futures that have been spawned.
    ///
    /// This is behind a mutex so that multiple concurrent actors can access it.
    /// A `RwLock` seems better, but `FuturesUnordered is not `Sync` in the case.
    /// But in fact it doesn't matter anyway, because all spawned futures execute
    /// CONCURRENTLY and hence there will be no contention.
    futures: Mutex<Pin<Box<FuturesUnordered<BoxFuture<'scope, ()>>>>>,
    enqueued: Mutex<Vec<BoxFuture<'scope, ()>>>,
    cancelled: Mutex<Option<C>>,
    phantom: PhantomData<&'scope &'env ()>,
}

fn is_sync<T: Sync>(t: T) -> T {
    t
}

impl<'scope, 'env, C: Send> Scope<'scope, 'env, C> {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(is_sync(Self {
            futures: Mutex::new(Box::pin(FuturesUnordered::new())),
            enqueued: Default::default(),
            cancelled: Default::default(),
            phantom: Default::default(),
        }))
    }

    pub(crate) fn drain(&self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), C>> {
        let mut futures = self.futures.lock().unwrap();
        'outer: loop {
            // once we are cancelled, we do no more work.
            if let Some(c) = self.cancelled.lock().unwrap().take() {
                return Poll::Ready(Err(c));
            }

            futures.extend(self.enqueued.lock().unwrap().drain(..));

            while let Some(()) = ready!(futures.as_mut().poll_next(cx)) {
                // once we are cancelled, we do no more work.
                if self.cancelled.lock().unwrap().is_some() {
                    continue 'outer;
                }
            }

            if self.enqueued.lock().unwrap().is_empty() {
                return Poll::Ready(Ok(()));
            }
        }
    }

    /// Clear out all pending tasks. This is used when dropping the
    /// scope body to ensure that any possible references to `Scope`
    /// are removed before we drop it.
    ///
    /// # Unsafe contract
    ///
    /// Once this returns, there are no more pending tasks.
    pub(crate) fn clear(&self) {
        self.futures.lock().unwrap().clear();
        self.enqueued.lock().unwrap().clear();
    }

    /// Cancel the scope immediately -- all existing tasks will stop at their next await point
    /// and never wake up again. Anything on their stacks will be dropped.
    ///
    /// This returns a future that you can await, but it will never complete (because you will never be reawoken).
    pub fn cancel<T>(&self, value: C) -> impl Future<Output = T> + 'scope
    where
        T: std::fmt::Debug + Send + 'scope,
    {
        let mut lock = self.cancelled.lock().unwrap();
        if lock.is_none() {
            *lock = Some(value);
        }
        std::mem::drop(lock);

        // The code below will never run
        self.spawn(async { panic!() })
    }

    /// Spawn a subjob. This will run concurrently with everything else in the scope.
    /// It may access stack fields defined outside the scope.
    /// The scope will not terminate until this task completes or the scope is cancelled.
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
