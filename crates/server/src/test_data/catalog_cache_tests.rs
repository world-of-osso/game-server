//! Real SQLite capabilities and independent source images, not cache implementation shape.
#[path = "catalog_cache.rs"]
mod catalog_cache;

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

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

#[test]
fn warm_immutable_input_survives_revoked_database_read_capability() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    write_catalog(&path);
    let db = readonly(&path);
    let original = catalog_cache::load(&path, || read_catalog(&db)).unwrap();
    deny_catalog_reads(&db);
    let reused = catalog_cache::load(&path, || read_catalog(&db))
        .expect("unchanged immutable definitions must not query SQLite again");
    assert_eq!(reused.0, [(133, "Fireball".into()), (585, "Smite".into())]);
    let mut app_owned = original.as_ref().clone();
    app_owned.0[0].1 = "only the first App".into();
    app_owned.0.pop();
    assert_eq!(reused.0.len(), 2);
    assert_eq!(reused.0[0].1, "Fireball");
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
