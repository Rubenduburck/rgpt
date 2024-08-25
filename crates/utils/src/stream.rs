use std::pin::Pin;

use tokio_stream::Stream;

use pin_project_lite::pin_project;

pin_project! {
    pub struct StreamAdapter<S, F> {
        #[pin]
        stream: S,
        f: F,
    }
}

impl<S, F, T1, E1, T2, E2> Stream for StreamAdapter<S, F>
where
    S: Stream<Item = Result<T1, E1>>,
    F: Fn(Result<T1, E1>) -> Result<T2, E2>,
{
    type Item = Result<T2, E2>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        this.stream
            .poll_next(cx)
            .map(|opt| opt.map(|res| (this.f)(res)))
    }
}

pub fn adapt_stream<S, F, T1, E1, T2, E2>(stream: S, f: F) -> Pin<Box<dyn Stream<Item = Result<T2, E2>> + Send>>
where
    S: Stream<Item = Result<T1, E1>> + Send + 'static,
    F: Fn(Result<T1, E1>) -> Result<T2, E2> + Send + 'static,
    T1: Send + 'static,
    E1: Send + 'static,
    T2: Send + 'static,
    E2: Send + 'static,
{
    Box::pin(StreamAdapter { stream, f })
}
