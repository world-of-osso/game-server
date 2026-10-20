//! TC a352b1fa boss_nethekurse.cpp; authentic map540 / encounter1936 bit1.
use super::*;

const NETHEKURSE: u32 = 16807;
const SHATTERED_HALLS: u32 = 540;
const NETHEKURSE_BIT: u32 = 1;

fn nethekurse_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(
        NETHEKURSE,
        SHATTERED_HALLS,
        NETHEKURSE_BIT,
        difficulty,
        None,
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
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn nethekurse_yells(h: &Harness, boss: Entity, group: u8) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&NETHEKURSE][&group];
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
#[ignore = "RED: Nethekurse TC script not ported"]
fn encounter_coverage_nethekurse_aggro_all_alive() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = nethekurse_copy(difficulty);
        h.engage(boss);
        h.advance(0.5);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, NETHEKURSE_BIT),
            Some(EncounterState::InProgress),
        );
        assert_eq!(
            nethekurse_yells(&h, boss, 3),
            1,
            "source SAY_AGGRO_ALL_ALIVE"
        );
        assert!(h.inbox(1).chat.is_empty(), "other-copy silence");
    }
}
