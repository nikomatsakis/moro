use crate::Scope;

pub trait AsyncIterator {
    type Item;

    // fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>)
    //     -> std::task::Poll<Option<Self::Item>>;
    //
    // how do you do this?
    //
    // fn next(&mut self) -> impl Future<Output = Option<Self::Item>> {
    //     pin!(self);
    //     std::future::poll_fn(|cx| this.poll_next(cx))
    // }

    async fn next(&mut self) -> Option<Self::Item>;

    fn filter(
        self,
        op: impl async FnMut(&Self::Item) -> bool,
    ) -> impl AsyncIterator<Item = Self::Item>
    where
        Self: Sized,
    {
        Filter {
            iter: self,
            filter_op: op,
        }
    }
}

pub trait IntoAsyncIter {
    type Item;

    // FIXME: The Scope type needs to not carry R.
    fn into_async_iter<R: Send>(
        self,
        scope: &Scope<'_, '_, R>,
    ) -> impl AsyncIterator<Item = Self::Item>;
}

impl<T: AsyncIterator> IntoAsyncIter for T {
    type Item = T::Item;

    fn into_async_iter<R: Send>(
        self,
        _scope: &Scope<'_, '_, R>,
    ) -> impl AsyncIterator<Item = Self::Item> {
        self
    }
}

struct Filter<I, O>
where
    I: AsyncIterator,
    O: async FnMut(&I::Item) -> bool,
{
    iter: I,
    filter_op: O,
}

impl<I, O> AsyncIterator for Filter<I, O>
where
    I: AsyncIterator,
    O: async FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    async fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.iter.next().await?;
            if (self.filter_op)(&item).await {
                return Some(item);
            }
        }
    }
}
