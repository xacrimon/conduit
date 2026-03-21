use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use pin_project_lite::pin_project;

pin_project! {
    pub struct CancellableStream<Stream, Signal> {
        #[pin]
        stream: Stream,
        #[pin]
        signal: Signal,
    }
}

impl<Stream, Signal> CancellableStream<Stream, Signal> {
    pub fn new(stream: Stream, signal: Signal) -> Self {
        Self { stream, signal }
    }
}

impl<T, Str, Si> Stream for CancellableStream<Str, Si>
where
    Str: Stream<Item = T>,
    Si: Future<Output = ()>,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let signal_poll = this.signal.poll(cx);
        if signal_poll.is_ready() {
            return Poll::Ready(None);
        }

        this.stream.poll_next(cx)
    }
}
