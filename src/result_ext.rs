use crate::Scope;

#[async_trait::async_trait]
pub trait OrCancel: Send + Sized {
    type Ok: Send;
    type Err: Send;

    async fn or_cancel<'scope, 'env, T>(
        self,
        scope: &'scope Scope<'scope, 'env, Result<T, Self::Err>>,
    ) -> Self::Ok
    where
        T: Send,
        Self: 'env;
}

#[async_trait::async_trait]
impl<O, E> OrCancel for Result<O, E>
where
    O: Send,
    E: Send,
{
    type Ok = O;
    type Err = E;

    async fn or_cancel<'scope, 'env, T>(self, scope: &'scope Scope<'scope, 'env, Result<T, E>>) -> O
    where
        T: Send,
        Self: 'env,
    {
        match self {
            Ok(o) => o,
            Err(e) => scope.terminate(Err(e)).await,
        }
    }
}
