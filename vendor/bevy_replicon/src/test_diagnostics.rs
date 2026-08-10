use core::sync::atomic::{AtomicU64, Ordering};

static ROOT_CANDIDATES: AtomicU64 = AtomicU64::new(0);
static REPLICATE_ENTITY_VISITS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Snapshot {
    pub root_candidates: u64,
    pub replicate_entity_visits: u64,
}

pub fn reset() {
    ROOT_CANDIDATES.store(0, Ordering::Relaxed);
    REPLICATE_ENTITY_VISITS.store(0, Ordering::Relaxed);
}

pub fn snapshot() -> Snapshot {
    Snapshot {
        root_candidates: ROOT_CANDIDATES.load(Ordering::Relaxed),
        replicate_entity_visits: REPLICATE_ENTITY_VISITS.load(Ordering::Relaxed),
    }
}

pub(crate) fn count_root_candidate() {
    ROOT_CANDIDATES.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn count_replicate_entity_visit() {
    REPLICATE_ENTITY_VISITS.fetch_add(1, Ordering::Relaxed);
}
