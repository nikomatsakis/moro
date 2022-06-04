use std::future::Future;

use futures::future::BoxFuture;

use crate::{prelude::UnwrapOrCancel, Scope};

pub trait OrCancel: Send + Sized {
    type Ok: Send;
    type Err: Send;

    fn or_cancel<'scope, 'env, T>(
        self,
        scope: &'scope Scope<'scope, 'env, Result<T, Self::Err>>,
    ) -> BoxFuture<'scope, Self::Ok>
    where
        Self: 'scope,
        Self::Ok: 'scope,
        T: Send;
}

impl<F, O, E> OrCancel for F
where
    F: Future<Output = Result<O, E>> + Send,
    O: Send,
    E: Send,
{
    type Ok = O;
    type Err = E;

    fn or_cancel<'scope, 'env, T>(
        self,
        scope: &'scope Scope<'scope, 'env, Result<T, Self::Err>>,
    ) -> BoxFuture<'scope, Self::Ok>
    where
        F: 'scope,
        Self::Ok: 'scope,
        T: Send,
    {
        Box::pin(scope.spawn(async { self.await.unwrap_or_cancel(scope).await }))
    }
}
