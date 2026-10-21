//! TC a352b1fa boss_warchief_kargath_bladefist.cpp; native70448/map540 bit3.
use super::*;

const KARGATH: u32 = 16808;
const SHATTERED_HALLS: u32 = 540;
const KARGATH_BIT: u32 = 3;

fn kargath_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, _, map) = encounter_fixture(
        KARGATH,
        SHATTERED_HALLS,
        KARGATH_BIT,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
    let boss = h.creatures_of(KARGATH)[0];
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
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn stand_near_kargath(h: &mut Harness, boss: Entity) {
    // HARNESS SETUP: isolated entrant approaches only after the source copy is ready.
    let position = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
}

fn kargath_yells(h: &Harness, boss: Entity, group: u8) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&KARGATH][&group];
    h.inbox(0)
        .chat
        .iter()
        .filter(|line| {
            line.channel == shared::protocol::ChatType::MonsterYell(boss.to_bits())
                && texts
                    .iter()
                    .any(|text| line.content == text.male_text || line.content == text.female_text)
        })
        .count()
}

#[test]
#[ignore = "RED: Kargath source script not ported"]
fn encounter_coverage_kargath_aggro_speech_and_native_encounter() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = kargath_copy(difficulty);
        stand_near_kargath(&mut h, boss);
        h.engage(boss);
        h.advance(0.2);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, KARGATH_BIT),
            Some(EncounterState::InProgress)
        );
        assert_eq!(kargath_yells(&h, boss, 0), 1, "source SAY_AGGRO once");
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 1938 && start.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).chat.is_empty() && h.inbox(1).starts.is_empty());
    }
}
