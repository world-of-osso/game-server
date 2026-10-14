//! Count every redb commit on the tick thread after its first frame begins.
//! FIFO workers and preceding startup code have no tick-thread accounting context.
use std::cell::Cell;
use std::panic::Location;
use std::time::Instant;

thread_local! {
    static TICK_COMMITS: Cell<Option<u64>> = const { Cell::new(None) };
}

pub(crate) fn begin_tick_frame() {
    if TICK_COMMITS.get().is_none() {
        TICK_COMMITS.set(Some(0));
    }
}

pub(crate) fn read_tick_commit_total() -> u64 {
    TICK_COMMITS.get().unwrap_or(0)
}

#[track_caller]
pub(crate) fn commit(transaction: redb::WriteTransaction) -> Result<(), redb::CommitError> {
    let started = TICK_COMMITS.get().map(|_| Instant::now());
    let result = transaction.commit();
    if let Some(started) = started {
        TICK_COMMITS.set(TICK_COMMITS.get().map(|count| count + 1));
        let caller = Location::caller();
        bevy::log::info!(
            source_file = caller.file(),
            source_line = caller.line(),
            commit_work_ms = started.elapsed().as_secs_f64() * 1000.0,
            succeeded = result.is_ok(),
            "Tick redb commit"
        );
    }
    result
}
