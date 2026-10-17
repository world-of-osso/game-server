use super::*;

// Original busy-writer RED from 0ff7a921a, retained before FIFO integration.
#[test]
fn achievement_monthly_snapshot_never_waits_for_a_busy_durable_writer() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 41, "Asyncproof", 1, 1).unwrap();
    let held = db.begin_write().unwrap();
    let blocker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        held.abort().unwrap();
    });
    let started = std::time::Instant::now();
    save_monthly_contribution(&db, 41, character, &PlayerAchievementState::default());
    let elapsed = started.elapsed();
    blocker.join().unwrap();
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "tick waited for writer: {elapsed:?}"
    );
}
