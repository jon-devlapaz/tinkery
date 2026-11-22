//! Think's model requests: which calls one send makes, and the background job that runs them
//! off the paint thread. The model host itself is [`crate::think::brain_dump::ModelHost`].

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

use serde::Deserialize;

pub use crate::pi::REQUEST_LIMIT;

/// The requests one send may make, in order: the reading, then the decision review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Request {
    /// The shaping call that reads the person's words.
    Shaping,
    /// The review that names decisions the reading settled silently.
    DecisionReview,
}

impl Request {
    /// Every request one send may make, in order.
    pub const ALL: [Request; 2] = [Request::Shaping, Request::DecisionReview];

    /// The name the call budget records it under.
    pub fn label(self) -> &'static str {
        match self {
            Request::Shaping => "shaping",
            Request::DecisionReview => "decision_review",
        }
    }
}

pub(super) struct Job<T> {
    cancelled: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<T, String>>,
    worker: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> Job<T> {
    pub(super) fn launch(
        work: impl FnOnce(&AtomicBool) -> Result<T, String> + Send + 'static,
    ) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancelled.clone();
        let (sender, result) = mpsc::channel();
        let worker = thread::spawn(move || {
            let _ = sender.send(work(&worker_cancel));
        });
        Self {
            cancelled,
            result,
            worker: Some(worker),
        }
    }

    pub(super) fn poll(&self) -> Option<Result<T, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Draft worker stopped unexpectedly.".into()))
            }
        }
    }
}

impl<T> Drop for Job<T> {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
