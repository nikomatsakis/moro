use crate::{AsyncIterator, IntoAsyncIter, Scope};

pub trait Stream: IntoAsyncIter {
    fn filter(self, op: impl AsyncFnMut(&Self::Item) -> bool) -> impl Stream<Item = Self::Item>
    where
        Self: Sized,
    {
        Filter {
            stream: self,
            filter_op: op,
        }
    }

    async fn for_each(&mut self, mut op: impl AsyncFnMut(Self::Item))
    where
        Self: Sized,
    {
        self.fold((), async |(), item| op(item).await).await
    }

    async fn fold<R>(&mut self, start: R, op: impl AsyncFnMut(R, Self::Item) -> R) -> R;
}

struct Filter<S, O>
where
    S: Stream,
    O: AsyncFnMut(&S::Item) -> bool,
{
    stream: S,
    filter_op: O,
}

impl<S, O> Stream for Filter<S, O>
where
    S: Stream,
    O: AsyncFnMut(&S::Item) -> bool,
{
    async fn fold<R>(&mut self, start: R, mut op: impl AsyncFnMut(R, Self::Item) -> R) -> R {
        self.stream
            .fold(start, async |acc, item| {
                if (self.filter_op)(&item).await {
                    op(acc, item).await
                } else {
                    acc
                }
            })
            .await
    }
}

impl<S, O> IntoAsyncIter for Filter<S, O>
where
    S: Stream,
    O: AsyncFnMut(&S::Item) -> bool,
{
    type Item = S::Item;

    fn into_async_iter<R: Send>(
        self,
        scope: &Scope<'_, '_, R>,
    ) -> impl AsyncIterator<Item = Self::Item> {
        let iter = self.stream.into_async_iter(scope);
        iter.filter(self.filter_op)
    }
}
