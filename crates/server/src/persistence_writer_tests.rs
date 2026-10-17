use super::*;
use redb::{ReadableDatabase, TableDefinition};
use std::time::{Duration, Instant};

const ROWS: TableDefinition<u64, &[u8]> = TableDefinition::new("writer_proof");

fn wait<T: Clone>(receipt: &CommitReceipt<T>) -> Result<T, String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = receipt.result() {
            return result;
        }
        assert!(
            Instant::now() < deadline,
            "durability receipt did not complete"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn persist(db: &Database, key: u64, value: &'static [u8]) -> Result<(), String> {
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    txn.open_table(ROWS)
        .map_err(|error| error.to_string())?
        .insert(key, value)
        .map_err(|error| error.to_string())?;
    txn.commit().map_err(|error| error.to_string())
}

#[test]
fn fifo_writer_withholds_durability_receipts_and_drains_owned_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fifo.redb");
    let db = Arc::new(Database::create(&path).unwrap());
    let writer = PersistenceWriter::start(Arc::clone(&db));
    let (release, blocked) = mpsc::channel();
    let (entered, ready) = mpsc::channel();
    let first = writer.enqueue("account41", move |db| {
        entered.send(()).unwrap();
        blocked.recv().unwrap();
        persist(db, 41, b"account-first")
    });
    ready.recv_timeout(Duration::from_secs(3)).unwrap();
    let monthly = writer.enqueue("monthly41", |db| persist(db, 41, b"monthly-final"));
    let other = writer.enqueue("account42", |db| persist(db, 42, b"other-owner"));
    assert!(first.result().is_none());
    assert!(monthly.result().is_none());
    assert!(other.result().is_none());
    release.send(()).unwrap();
    wait(&monthly).unwrap();
    drop(writer); // shutdown must drain the later account42 write.
    assert_eq!(first.result(), Some(Ok(())));
    assert_eq!(other.result(), Some(Ok(())));
    drop(db);
    let reopened = Database::open(&path).unwrap();
    let read = reopened.begin_read().unwrap();
    let rows = read.open_table(ROWS).unwrap();
    assert_eq!(rows.get(41).unwrap().unwrap().value(), b"monthly-final");
    assert_eq!(rows.get(42).unwrap().unwrap().value(), b"other-owner");
}

#[test]
fn fifo_writer_crash_before_flush_loses_only_unacknowledged_work() {
    const CHILD_PATH: &str = "SERVER_WRITER_CRASH_FIXTURE";
    if let Ok(path) = std::env::var(CHILD_PATH) {
        let db = Arc::new(Database::create(&path).unwrap());
        let writer = PersistenceWriter::start(db);
        let acknowledged = writer.enqueue("acknowledged transfer", |db| {
            persist(db, 41, b"acknowledged")
        });
        wait(&acknowledged).unwrap();
        let (entered, ready) = mpsc::channel();
        let (_release, blocked) = mpsc::channel::<()>();
        writer.enqueue("blocked snapshot", move |db| {
            entered.send(()).unwrap();
            blocked.recv().unwrap();
            persist(db, 42, b"unacknowledged")
        });
        ready.recv_timeout(Duration::from_secs(3)).unwrap();
        writer.enqueue("later snapshot", |db| persist(db, 43, b"unacknowledged"));
        std::process::exit(77); // abrupt exit: no Rust destructors or shutdown drain.
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crash.redb");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "persistence_writer::tests::fifo_writer_crash_before_flush_loses_only_unacknowledged_work", "--nocapture"])
        .env(CHILD_PATH, &path).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(77),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reopened = Database::open(&path).unwrap();
    let txn = reopened.begin_read().unwrap();
    let rows = txn.open_table(ROWS).unwrap();
    assert_eq!(rows.get(41).unwrap().unwrap().value(), b"acknowledged");
    assert!(rows.get(42).unwrap().is_none());
    assert!(rows.get(43).unwrap().is_none());
}

#[test]
fn fifo_writer_reports_failed_commit_and_continues_later_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let db = Arc::new(Database::create(dir.path().join("failure.redb")).unwrap());
    let writer = PersistenceWriter::start(Arc::clone(&db));
    let failed = writer.enqueue("rejected purchase", |_| Err("purchase refused".into()));
    let later = writer.enqueue("next owner", |db| persist(db, 42, b"durable"));
    assert_eq!(wait(&failed), Err("purchase refused".into()));
    wait(&later).unwrap();
    assert_eq!(writer.metrics.failed.load(Ordering::Relaxed), 1);
    assert_eq!(writer.metrics.committed.load(Ordering::Relaxed), 1);
    assert_eq!(
        db.begin_read()
            .unwrap()
            .open_table(ROWS)
            .unwrap()
            .get(42)
            .unwrap()
            .unwrap()
            .value(),
        b"durable"
    );
}

#[test]
fn fifo_typed_receipts_withhold_values_and_report_panics_before_continuing() {
    let dir = tempfile::tempdir().unwrap();
    let db = Arc::new(Database::create(dir.path().join("typed.redb")).unwrap());
    let writer = PersistenceWriter::start(Arc::clone(&db));
    let held = db.begin_write().unwrap();
    let value = writer.enqueue_result("owned typed snapshot", |db| {
        persist(db, 41, b"committed typed value")?;
        Ok(41_u64)
    });
    let panic = writer.enqueue_result("typed panic", |_| -> Result<u64, String> {
        panic!("typed write failed");
    });
    let later = writer.enqueue_result("later typed value", |db| {
        persist(db, 42, b"after failure")?;
        Ok(42_u64)
    });
    assert!(value.result().is_none());
    assert!(panic.result().is_none());
    assert!(later.result().is_none());
    held.abort().unwrap();
    assert_eq!(wait(&value), Ok(41));
    assert_eq!(wait(&panic), Err("typed write failed".into()));
    assert_eq!(wait(&later), Ok(42));
    assert_eq!(
        db.begin_read()
            .unwrap()
            .open_table(ROWS)
            .unwrap()
            .get(42)
            .unwrap()
            .unwrap()
            .value(),
        b"after failure"
    );
}
