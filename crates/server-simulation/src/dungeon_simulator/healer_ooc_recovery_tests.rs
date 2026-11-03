//! A class follower heals its injured owner between pulls.
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Health;
use shared::protocol::CombatLogKind;

fn run_in_snapshot_subprocess(test: &str) -> bool {
    if std::env::var_os("HEALER_RECOVERY_SNAPSHOT_CHILD").is_some() {
        return false;
    }
    let dir = tempfile::tempdir().unwrap();
    let snapshot = dir.path().join("world.db");
    let source = crate::test_data::world_db_path();
    let db =
        rusqlite::Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    db.execute("VACUUM INTO ?1", [snapshot.to_str().unwrap()])
        .unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("HEALER_RECOVERY_SNAPSHOT_CHILD", "1")
        .env("FOLLOWER_DPS_WORLD_DB", &snapshot)
        .env("DPS_PARITY_WORLD_DB", &snapshot)
        .status()
        .unwrap();
    assert!(status.success(), "snapshot subprocess failed: {status}");
    true
}

#[test]
fn healers_top_up_small_wounds_with_a_persistent_unattackable_combat_reference() {
    if run_in_snapshot_subprocess(concat!(
        module_path!(),
        "::healers_top_up_small_wounds_with_a_persistent_unattackable_combat_reference"
    )) {
        return;
    }
    use shared::components::{CombatStatus, UnitFlags};
    for spec in [65, 105, 256, 257, 264, 270, 1468] {
        let mut run = super::healer_simc_replay::snapshot_party(spec, 70, 1);
        let owner = run.slots[4];
        let healer = run.healer;
        let source = run.sources[0];
        run.app
            .world_mut()
            .entity_mut(source)
            .insert(UnitFlags(0x0200_0000));
        for _ in 0..200 {
            run.app.update();
        }
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), owner, source);
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), healer, source);
        assert!(run.app.world().get::<CombatStatus>(owner).unwrap().0);
        assert!(crate::followers::engaged_targets(run.app.world_mut(), healer).is_empty());
        let max = run.app.world().get::<Health>(owner).unwrap().max;
        run.app
            .world_mut()
            .get_mut::<Health>(owner)
            .unwrap()
            .current = max * 0.985;
        for _ in 0..600 {
            run.app.update();
            if run.app.world().get::<Health>(owner).unwrap().current == max {
                assert!(crate::combat_refs::has_active_reference_to(
                    run.app.world(),
                    owner,
                    source
                ));
                let healed = run
                    .app
                    .world()
                    .resource::<ObservedCombatEvents>()
                    .0
                    .iter()
                    .any(|event| {
                        event.kind == CombatLogKind::Heal
                            && event.source == Some(healer.to_bits())
                            && event.target == Some(owner.to_bits())
                            && event.amount > 0
                    });
                assert!(
                    healed,
                    "spec {spec} must heal the owner, not rely on regeneration"
                );
                break;
            }
        }
        assert_eq!(
            run.app.world().get::<Health>(owner).unwrap().current,
            max,
            "spec {spec} failed to top up the small wound"
        );
    }
}

#[test]
fn restoration_shaman_heals_injured_owner_out_of_combat() {
    if run_in_snapshot_subprocess(concat!(
        module_path!(),
        "::restoration_shaman_heals_injured_owner_out_of_combat"
    )) {
        return;
    }
    let mut run = super::healer_simc_replay::snapshot_party(264, 70, 1);
    let owner = run.slots[4];
    let healer = run.healer;
    let health = run.app.world_mut().get_mut::<Health>(owner).unwrap();
    let initial = health.max * 0.666;
    run.app
        .world_mut()
        .get_mut::<Health>(owner)
        .unwrap()
        .current = initial;

    for _ in 0..600 {
        run.app.update();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        if events.iter().any(|event| {
            event.kind == CombatLogKind::Heal
                && event.source == Some(healer.to_bits())
                && event.target == Some(owner.to_bits())
                && event.amount > 0
        }) {
            assert!(run.app.world().get::<Health>(owner).unwrap().current > initial);
            return;
        }
    }
    panic!("idle Restoration Shaman did not heal its injured owner");
}
