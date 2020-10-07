use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use uuid::Uuid;

pub trait Metrics: Debug + Send + Sync + 'static {
    fn task_started(&self, id: Uuid, name: String);
    fn task_stopped(&self, id: Uuid, name: String);
}

impl<M: Metrics + ?Sized> Metrics for Box<M> {
    fn task_started(&self, id: Uuid, name: String) {
        (**self).task_started(id, name)
    }

    fn task_stopped(&self, id: Uuid, name: String) {
        (**self).task_stopped(id, name)
    }
}

impl<M: Metrics + ?Sized> Metrics for Arc<M> {
    fn task_started(&self, id: Uuid, name: String) {
        (**self).task_started(id, name)
    }

    fn task_stopped(&self, id: Uuid, name: String) {
        (**self).task_stopped(id, name)
    }
}

#[derive(Debug)]
pub struct DummyMetrics;

impl Metrics for DummyMetrics {
    fn task_started(&self, _: Uuid, _: String) {}
    fn task_stopped(&self, _: Uuid, _: String) {}
}

impl Metrics for AtomicUsize {
    fn task_started(&self, _: Uuid, _: String) {
        self.fetch_add(1, Ordering::Relaxed);
    }

    fn task_stopped(&self, _: Uuid, _: String) {
        self.fetch_sub(1, Ordering::Relaxed);
    }
}
