//! One FIFO durable writer; game systems poll receipts without waiting.
//! Reference: TrinityCore a352b1fa DatabaseWorkerPool::AsyncCommitTransaction.
use redb::Database;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

#[cfg(feature = "native-proof-hold-persistence")]
#[path = "native_proof_hold_persistence.rs"]
pub(crate) mod native_proof_hold;

#[cfg(test)]
#[path = "persistence_writer_tests.rs"]
mod tests;

type WriteOperation = Box<dyn FnOnce(&Database, &WriterMetrics) + Send>;

#[derive(Clone)]
pub(crate) struct CommitReceipt<T = ()>(Arc<Mutex<Option<Result<T, String>>>>);

impl<T: Clone> CommitReceipt<T> {
    pub(crate) fn result(&self) -> Option<Result<T, String>> {
        self.0.lock().expect("persistence receipt lock").clone()
    }
}

#[derive(Default)]
pub(crate) struct WriterMetrics {
    pub(crate) submitted: AtomicU64,
    pub(crate) committed: AtomicU64,
    pub(crate) failed: AtomicU64,
}

struct Job {
    operation: WriteOperation,
}

pub(crate) struct PersistenceWriter {
    sender: Option<mpsc::Sender<Job>>,
    worker: Option<JoinHandle<()>>,
    pub(crate) metrics: Arc<WriterMetrics>,
}

impl PersistenceWriter {
    pub(crate) fn start(db: Arc<Database>) -> Self {
        let (sender, receiver) = mpsc::channel::<Job>();
        let metrics = Arc::new(WriterMetrics::default());
        let worker_metrics = Arc::clone(&metrics);
        let worker = std::thread::Builder::new()
            .name("persistence-writer".into())
            .spawn(move || {
                for job in receiver {
                    (job.operation)(&db, &worker_metrics);
                }
            })
            .expect("start FIFO persistence writer");
        Self {
            sender: Some(sender),
            worker: Some(worker),
            metrics,
        }
    }

    pub(crate) fn enqueue(
        &self,
        name: &'static str,
        operation: impl FnOnce(&Database) -> Result<(), String> + Send + 'static,
    ) -> CommitReceipt {
        self.enqueue_result(name, operation)
    }

    pub(crate) fn enqueue_result<T: Clone + Send + 'static>(
        &self,
        name: &'static str,
        operation: impl FnOnce(&Database) -> Result<T, String> + Send + 'static,
    ) -> CommitReceipt<T> {
        let receipt = CommitReceipt(Arc::new(Mutex::new(None)));
        let completion = receipt.clone();
        let job = Job {
            operation: Box::new(move |db, metrics| {
                let result = catch_operation(db, operation);
                record_result(name, &result, metrics);
                *completion.0.lock().expect("persistence receipt lock") = Some(result);
            }),
        };
        self.metrics.submitted.fetch_add(1, Ordering::Relaxed);
        if self
            .sender
            .as_ref()
            .expect("writer sender")
            .send(job)
            .is_err()
        {
            self.metrics.failed.fetch_add(1, Ordering::Relaxed);
            bevy::log::error!(name, "Persistence writer disconnected");
            *receipt.0.lock().expect("persistence receipt lock") =
                Some(Err("persistence writer disconnected".into()));
        }
        receipt
    }
}

fn catch_operation<T>(
    db: &Database,
    operation: impl FnOnce(&Database) -> Result<T, String>,
) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(db))).unwrap_or_else(
        |panic| {
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|message| message.to_string())
                })
                .unwrap_or_else(|| "unknown persistence panic".into());
            Err(message)
        },
    )
}

fn record_result<T>(name: &'static str, result: &Result<T, String>, metrics: &WriterMetrics) {
    match result {
        Ok(_) => {
            metrics.committed.fetch_add(1, Ordering::Relaxed);
        }
        Err(error) => {
            metrics.failed.fetch_add(1, Ordering::Relaxed);
            bevy::log::error!(operation = name, %error, "Persistence commit failed");
        }
    }
}

impl Drop for PersistenceWriter {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            self.metrics.failed.fetch_add(1, Ordering::Relaxed);
            bevy::log::error!("Persistence writer panicked while draining shutdown");
        }
    }
}
