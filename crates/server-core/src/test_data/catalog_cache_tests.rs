//! Real SQLite capabilities and independent source images, not cache implementation shape.
#[path = "catalog_cache.rs"]
mod catalog_cache;

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Condvar, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
struct Catalog(Vec<(u32, String)>);

fn write_catalog(path: &Path) {
    Connection::open(path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE catalog_rows (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
             INSERT INTO catalog_rows VALUES (133, 'Fireball'), (585, 'Smite');",
        )
        .unwrap();
}

fn readonly(path: &Path) -> Connection {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
}

fn read_catalog(db: &Connection) -> rusqlite::Result<Catalog> {
    let mut query = db.prepare("SELECT id, name FROM catalog_rows ORDER BY id")?;
    let rows = query.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(Catalog(rows.collect::<rusqlite::Result<_>>()?))
}

fn deny_catalog_reads(db: &Connection) {
    db.authorizer(Some(|context: AuthContext<'_>| match context.action {
        AuthAction::Read {
            table_name: "catalog_rows",
            ..
        } => Authorization::Deny,
        _ => Authorization::Allow,
    }));
}

#[derive(Default)]
struct ReleaseProbe {
    count: Mutex<usize>,
    changed: Condvar,
}

impl ReleaseProbe {
    fn count(&self) -> usize {
        *self.count.lock().unwrap()
    }

    fn wait_for_drop(&self) -> usize {
        const RELEASE_DEADLINE: Duration = Duration::from_secs(30);
        let count = self.count.lock().unwrap();
        // Another lookup may own the removed slot until its out-of-lock drop runs.
        let (count, _) = self
            .changed
            .wait_timeout_while(count, RELEASE_DEADLINE, |count| *count == 0)
            .unwrap();
        *count
    }
}

struct RetainedCatalog {
    rows: Catalog,
    released: Arc<ReleaseProbe>,
}

impl Drop for RetainedCatalog {
    fn drop(&mut self) {
        *self.released.count.lock().unwrap() += 1;
        self.released.changed.notify_all();
    }
}

#[test]
fn deleted_prepared_input_releases_definitions_after_live_apps_drop() {
    let directory = tempfile::tempdir().unwrap();
    let canonical = directory.path().join("canonical.db");
    let prepared = directory.path().join("prepared.db");
    write_catalog(&canonical);
    std::fs::copy(&canonical, &prepared).unwrap();
    let released = Arc::new(ReleaseProbe::default());
    let definitions = catalog_cache::load(&prepared, || {
        Ok(RetainedCatalog {
            rows: read_catalog(&readonly(&prepared))?,
            released: released.clone(),
        })
    })
    .unwrap();
    let sibling = definitions.clone();
    std::fs::remove_file(&prepared).unwrap();
    let canonical_db = readonly(&canonical);
    let canonical_rows = catalog_cache::load(&canonical, || read_catalog(&canonical_db)).unwrap();
    assert_eq!(
        released.count(),
        0,
        "live Apps must retain their decoded definitions"
    );
    let mut local = definitions.rows.clone();
    local.0[0].1 = "only one App".into();
    local.0.pop();
    drop(definitions);
    assert_eq!(
        sibling.rows.0,
        [(133, "Fireball".into()), (585, "Smite".into())]
    );
    assert_eq!(canonical_rows.0, sibling.rows.0);
    assert_eq!(local.0, [(133, "only one App".into())]);
    assert_eq!(released.count(), 0);
    drop(sibling);
    assert_eq!(
        released.wait_for_drop(),
        1,
        "deleted source must not retain definitions after the last App drops"
    );
    deny_catalog_reads(&canonical_db);
    assert_eq!(
        catalog_cache::load(&canonical, || read_catalog(&canonical_db))
            .unwrap()
            .0,
        canonical_rows.0
    );
}

#[test]
fn deleted_prepared_input_with_no_live_app_releases_on_next_catalog_load() {
    let directory = tempfile::tempdir().unwrap();
    let prepared = directory.path().join("prepared.db");
    let canonical = directory.path().join("canonical.db");
    write_catalog(&prepared);
    write_catalog(&canonical);
    let released = Arc::new(ReleaseProbe::default());
    let definitions = catalog_cache::load(&prepared, || {
        Ok(RetainedCatalog {
            rows: read_catalog(&readonly(&prepared))?,
            released: released.clone(),
        })
    })
    .unwrap();
    drop(definitions);
    assert_eq!(released.count(), 0, "existing input remains reusable");
    std::fs::remove_file(&prepared).unwrap();
    catalog_cache::load(&canonical, || read_catalog(&readonly(&canonical))).unwrap();
    assert_eq!(
        released.wait_for_drop(),
        1,
        "obsolete prepared input must release decoded resources"
    );
}

#[test]
fn warm_immutable_input_survives_revoked_database_read_capability() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    write_catalog(&path);
    let db = readonly(&path);
    let original = catalog_cache::load_connection(&db, || read_catalog(&db)).unwrap();
    deny_catalog_reads(&db);
    let reused = catalog_cache::load_connection(&db, || read_catalog(&db))
        .expect("unchanged immutable definitions must not query SQLite again");
    assert_eq!(reused.0, [(133, "Fireball".into()), (585, "Smite".into())]);
    let mut app_owned = original.clone();
    app_owned.0[0].1 = "only the first App".into();
    app_owned.0.pop();
    assert_eq!(reused.0.len(), 2);
    assert_eq!(reused.0[0].1, "Fireball");
}

#[test]
fn writable_and_in_memory_connections_observe_uncheckpointed_edits() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    write_catalog(&path);
    for db in [
        Connection::open(&path).unwrap(),
        Connection::open_in_memory().unwrap(),
    ] {
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS catalog_rows (id INTEGER PRIMARY KEY, name TEXT);
            INSERT OR REPLACE INTO catalog_rows VALUES (133, 'Fireball'), (585, 'Smite');",
        )
        .unwrap();
        let before = catalog_cache::load_connection(&db, || read_catalog(&db)).unwrap();
        db.execute_batch("BEGIN; UPDATE catalog_rows SET name='edited locally' WHERE id=133;")
            .unwrap();
        let after = catalog_cache::load_connection(&db, || read_catalog(&db)).unwrap();
        assert_eq!(before.0[0].1, "Fireball");
        assert_eq!(after.0[0].1, "edited locally");
        db.execute_batch("ROLLBACK").unwrap();
    }
}

#[test]
fn concurrent_apps_need_only_one_authorized_sqlite_reader() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    write_catalog(&path);
    let start = Arc::new(Barrier::new(4));
    let authorized = Arc::new(AtomicUsize::new(0));
    let workers: Vec<_> = (1..=4)
        .map(|id| {
            let path = path.clone();
            let start = start.clone();
            let authorized = authorized.clone();
            std::thread::spawn(move || {
                let db = readonly(&path);
                db.authorizer(Some(move |context: AuthContext<'_>| {
                    if let AuthAction::Read {
                        table_name: "catalog_rows",
                        ..
                    } = context.action
                    {
                        let reader = match authorized.compare_exchange(
                            0,
                            id,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => id,
                            Err(reader) => reader,
                        };
                        return if reader == id {
                            Authorization::Allow
                        } else {
                            Authorization::Deny
                        };
                    }
                    Authorization::Allow
                }));
                start.wait();
                catalog_cache::load(&path, || read_catalog(&db))
            })
        })
        .collect();
    for worker in workers {
        let definitions = worker
            .join()
            .unwrap()
            .expect("one authorized source reader must serve every concurrent App");
        assert_eq!(definitions.0[1], (585, "Smite".into()));
    }
}

#[test]
fn identical_bytes_at_another_path_do_not_inherit_database_authority() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("canonical.db");
    let prepared = directory.path().join("prepared.db");
    write_catalog(&first);
    std::fs::copy(&first, &prepared).unwrap();
    let canonical_db = readonly(&first);
    let canonical = catalog_cache::load(&first, || read_catalog(&canonical_db)).unwrap();
    let prepared_db = readonly(&prepared);
    deny_catalog_reads(&prepared_db);
    assert!(catalog_cache::load(&prepared, || read_catalog(&prepared_db)).is_err());
    assert_eq!(canonical.0[0].1, "Fireball");
}

#[test]
fn same_length_rewrite_with_preserved_mtime_loads_new_content() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    write_catalog(&path);
    let original_mtime = path.metadata().unwrap().modified().unwrap();
    let before = catalog_cache::load(&path, || read_catalog(&readonly(&path))).unwrap();
    Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE catalog_rows SET name = 'Firebolt' WHERE id = 133",
            [],
        )
        .unwrap();
    std::fs::File::open(&path)
        .unwrap()
        .set_modified(original_mtime)
        .unwrap();
    let after = catalog_cache::load(&path, || read_catalog(&readonly(&path))).unwrap();
    assert_eq!(before.0[0].1, "Fireball");
    assert_eq!(after.0[0].1, "Firebolt");
}

#[test]
fn new_content_at_the_same_path_preserves_old_apps_and_loads_new_rows() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    write_catalog(&path);
    let before = catalog_cache::load(&path, || read_catalog(&readonly(&path))).unwrap();
    Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE catalog_rows SET name = 'Prepared Fireball' WHERE id = 133",
            [],
        )
        .unwrap();
    let after = catalog_cache::load(&path, || read_catalog(&readonly(&path))).unwrap();
    assert_eq!(before.0[0].1, "Fireball");
    assert_eq!(after.0[0].1, "Prepared Fireball");
    assert_eq!(after.0[1], (585, "Smite".into()));
}
