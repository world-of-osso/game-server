//! TDB1210 Porung20923 SmartAI; TC a352b1fa header identity; native70448 bit0.
use super::*;

const PORUNG: u32 = 20923;
const SHATTERED_HALLS: u32 = 540;
const PORUNG_BIT: u32 = 0;

fn porung_copy(difficulty: u32) -> (Harness, InMap) {
    let (mut h, _, map) = encounter_fixture(
        PORUNG,
        SHATTERED_HALLS,
        PORUNG_BIT,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
    let directory = crate::ground::ground_data_dir();
    let wdt = h.app.world().resource::<MapStore>().maps[&SHATTERED_HALLS].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &directory,
        std::collections::HashMap::from([(SHATTERED_HALLS, wdt)]),
    ));
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&directory.join("los"))
            .expect("authentic eye tables"),
    );
    (h, map)
}

fn listen_and_approach(h: &mut Harness, boss: Entity) {
    // HARNESS SETUP: proximity begins after the authentic Heroic copy is ready.
    let position = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
}

#[test]
#[ignore = "RED: Porung primary20923/1935bit0 not published"]
fn encounter_coverage_porung_real_heroic_engage_starts_native_encounter() {
    let (mut h, map) = porung_copy(2);
    let bosses = h.creatures_of(PORUNG);
    assert_eq!(bosses.len(), 1, "source Heroic-only spawn34038");
    let boss = bosses[0];
    listen_and_approach(&mut h, boss);
    h.engage(boss);
    h.advance(0.2);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, PORUNG_BIT),
        Some(EncounterState::InProgress),
        "real SmartAI engage must drive1935bit0"
    );
    assert!(
        h.inbox(0)
            .starts
            .iter()
            .any(|start| start.encounter_id == 1935 && start.difficulty_id == 2)
    );
    assert!(h.inbox(1).starts.is_empty());
}
