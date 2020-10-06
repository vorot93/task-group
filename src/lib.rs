use futures::future::{abortable, AbortHandle};
use parking_lot::Mutex;
use std::{
    collections::{hash_map::Entry, HashMap},
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum Shutdown {
    /// Runtime shutdown.
    Runtime,
    /// Task group was dropped.
    TaskGroup,
}

/// This can be awaited on to get task output.
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
struct Inner {
    tasks: Mutex<HashMap<Uuid, AbortHandle>>,
}

impl Inner {
    fn insert(&self, item: AbortHandle) -> Uuid {
        let mut tasks = self.tasks.lock();
        loop {
            let id = Uuid::new_v4();
            if let Entry::Vacant(vacant) = tasks.entry(id) {
                vacant.insert(item);
                return id;
            }
        }
    }

    fn remove(&self, id: Uuid) -> Option<AbortHandle> {
        self.tasks.lock().remove(&id)
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        for (_, handle) in self.tasks.lock().drain() {
            handle.abort();
        }
    }
}

/// This is a holding structure that "owns" tasks. That is, when this struct is dropped, tasks are cancelled and eventually dropped from the Tokio runtime.
#[derive(Default, Debug)]
pub struct TaskGroup(Arc<Inner>);

impl TaskGroup {
    /// Spawn task on this task group.
    pub fn spawn<Fut, T>(&self, future: Fut) -> TaskHandle<T>
    where
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (t, handle) = abortable(future);
        let id = self.0.insert(handle);
        let spawned_handle = tokio::spawn({
            let inner = Arc::downgrade(&self.0);
            async move {
                let res = t.await;
                if let Some(inner) = inner.upgrade() {
                    inner.remove(id);
                }
                res
            }
        });
        TaskHandle(Box::pin(async move {
            Ok(spawned_handle
                .await
                .map_err(|_| Shutdown::Runtime)?
                .map_err(|_| Shutdown::TaskGroup)?)
        }))
    }

    /// Returns the number of tasks that are currently active in this task group.
    pub fn len(&self) -> usize {
        self.0.tasks.lock().len()
    }

    /// Returns `true` if no tasks are active in this task group.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
