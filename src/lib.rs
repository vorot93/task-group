use crossbeam::queue::SegQueue;
use futures::future::abortable;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Clone, Debug)]
pub enum Shutdown {
    Runtime,
    TaskGroup,
}

pub struct TaskHandle<T>(Pin<Box<dyn Future<Output = Result<T, Shutdown>> + Send + 'static>>);

impl<T> Future for TaskHandle<T>
where
    T: Send + 'static,
{
    type Output = Result<T, Shutdown>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

#[derive(Default, Debug)]
pub struct TaskGroup(SegQueue<futures::future::AbortHandle>);

impl TaskGroup {
    pub fn spawn<Fut, T>(&self, future: Fut) -> TaskHandle<T>
    where
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (t, handle) = abortable(future);
        self.0.push(handle);
        let spawned_handle = tokio::spawn(t);
        TaskHandle(Box::pin(async move {
            Ok(spawned_handle
                .await
                .map_err(|_| Shutdown::Runtime)?
                .map_err(|_| Shutdown::TaskGroup)?)
        }))
    }
}

impl Drop for TaskGroup {
    fn drop(&mut self) {
        while let Ok(handle) = self.0.pop() {
            handle.abort();
        }
    }
}
