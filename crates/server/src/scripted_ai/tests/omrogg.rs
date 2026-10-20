//! TC a352b1fa Shattered Halls Warbringer O'mrogg16809 (boss_warbringer_omrogg.cpp).
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;

const OMROGG: u32 = 16809;
const LEFT_HEAD: u32 = 19523;
const RIGHT_HEAD: u32 = 19524;
const SHATTERED_HALLS: u32 = 540;
/// TC DATA_OMROGG: installed DB2 encounter1937, completion bit2.
const OMROGG_BIT: u32 = 2;
const OMROGG_ENCOUNTER: u32 = 1937;
const FEAR: u32 = 30584;
const THUNDERCLAP: u32 = 30633;
const BEATDOWN: u32 = 30618;
const BURNING_MAUL: u32 = 30598;
const BURNING_MAUL_H: u32 = 36056;
const BURNING_MAUL_HIT: u32 = 36057;
const BURNING_MAUL_PERIODIC: u32 = 30620;
const BLAST_WAVE: u32 = 30600;
/// `creature_equip_template` 16809: ID1 the spawn's weapon, ID2 EQUIP_ID_BURNING_MAUL.
const ORIGINAL_WEAPON: u32 = 29484;
const BURNING_MAUL_WEAPON: u32 = 29479;

/// A Shattered Halls copy filled by production copy fill from the real map540 rows,
/// with authentic ground and eye tables.
fn omrogg_copy(difficulty: u32) -> (Harness, InMap) {
    let (mut h, _, map) = encounter_fixture(
        OMROGG,
        SHATTERED_HALLS,
        OMROGG_BIT,
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

/// The first player sees `actors` (chat, SpellGo and combat log delivery).
fn watch(h: &mut Harness, actors: &[Entity]) {
    let connection = h.players[0].1;
    for &actor in actors {
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(connection);
        h.app.world_mut().entity_mut(actor).insert(visibility);
    }
}

/// Every logged `spell` CastSuccess by `caster`, including events no player receives.
fn observed_casts(h: &Harness, caster: Entity, spell: u32) -> usize {
    h.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::CastSuccess
                && event.source == Some(caster.to_bits())
                && event.spell_id == Some(spell)
        })
        .count()
}

/// The first player stands `offset` yards east of O'mrogg.
fn stand_beside(h: &mut Harness, boss: Entity, offset: f32) {
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + offset,
        ..origin
    });
}

fn weapon(h: &Harness, unit: Entity) -> Option<u32> {
    h.app
        .world()
        .get::<shared::components::EquipmentAppearance>(unit)
        .and_then(|equipment| equipment.entries.first())
        .and_then(|entry| entry.item_id)
}

#[test]
fn encounter_coverage_omrogg_native_inputs_qualify() {
    for difficulty in [1, 2] {
        let (mut h, map) = omrogg_copy(difficulty);
        assert_eq!(
            h.app
                .world()
                .resource::<MapStore>()
                .instance_scripts
                .get(&SHATTERED_HALLS)
                .map(String::as_str),
            Some("instance_shattered_halls")
        );
        for entry in [OMROGG, LEFT_HEAD, RIGHT_HEAD] {
            assert_eq!(
                h.creatures_of(entry).len(),
                1,
                "TDB spawnDifficulties1,2: {entry} in difficulty{difficulty}"
            );
        }
        let boss = h.creatures_of(OMROGG)[0];
        assert_eq!(
            weapon(&h, boss),
            Some(ORIGINAL_WEAPON),
            "spawn equipment_id1"
        );
        let equipment = &h.app.world().resource::<WorldData>().equipment[&OMROGG];
        let maul = equipment[&2]
            .entries
            .first()
            .and_then(|entry| entry.item_id);
        assert_eq!(maul, Some(BURNING_MAUL_WEAPON), "EQUIP_ID_BURNING_MAUL");

        let texts = &h.app.world().resource::<CreatureTexts>().groups;
        assert_eq!(texts[&OMROGG][&0].len(), 1, "EMOTE_ROAR");
        assert_eq!(texts[&OMROGG][&0][0].chat_type, 16, "MonsterEmote");
        let left: Vec<_> = (0..=11).map(|g| texts[&LEFT_HEAD][&g].len()).collect();
        assert_eq!(left, [1; 12], "left head groups0-11");
        let right: Vec<_> = (0..=10).map(|g| texts[&RIGHT_HEAD][&g].len()).collect();
        assert_eq!(right, [1; 11], "right head groups0-10");

        let spells = h.app.world().resource::<SpellStore>();
        // OMROGG_BURNING_MAUL_30620_ABSENT_70448: the periodic trigger has no SpellInfo.
        assert!(spells.get(BURNING_MAUL_PERIODIC).is_none());
        for spell in [
            FEAR,
            THUNDERCLAP,
            BEATDOWN,
            BURNING_MAUL,
            BURNING_MAUL_H,
            BURNING_MAUL_HIT,
            BLAST_WAVE,
        ] {
            assert!(spells.get(spell).is_some(), "spell{spell}");
        }

        // HARNESS SETUP: direct threat, not a walked pull.
        h.engage(boss);
        h.advance(0.5);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, OMROGG_BIT),
            Some(EncounterState::InProgress)
        );
        assert!(
            h.inbox(0).starts.iter().any(|start| {
                start.encounter_id == OMROGG_ENCOUNTER && start.difficulty_id == difficulty
            }),
            "bit2 -> DungeonEncounter1937 in difficulty{difficulty}"
        );
        assert!(h.inbox(1).starts.is_empty(), "other copy isolated");
    }
}

#[test]
fn encounter_coverage_omrogg_native_spell_admissions_qualify() {
    for difficulty in [1, 2] {
        let (mut h, _) = omrogg_copy(difficulty);
        h.app.init_resource::<ObservedCombatEvents>();
        let boss = h.creatures_of(OMROGG)[0];
        stand_beside(&mut h, boss, 2.0);
        watch(&mut h, &[boss]);
        let maul = if difficulty == 2 {
            BURNING_MAUL_H
        } else {
            BURNING_MAUL
        };
        // Thunderclap and Beatdown share category1152 "Creature Special 1":
        // Beatdown locks it8s, Thunderclap3s, from completion of their1s casts.
        // Each cast waits out the prior lock.
        for spell in [FEAR, BEATDOWN, THUNDERCLAP, BLAST_WAVE, maul] {
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, spell, None, false)
                .unwrap_or_else(|error| panic!("ordinary self-area {spell}: {error:?}"));
            h.advance(9.5);
            eprintln!(
                "OMROGG difficulty{difficulty} spell{spell} casts={}",
                observed_casts(&h, boss, spell)
            );
            assert_eq!(observed_casts(&h, boss, spell), 1, "spell{spell} completes");
        }
        assert!(h.has_aura(boss, maul), "Burning Maul self aura");

        // The aura's PROC_TRIGGER_SPELL default: melee hits trigger Burning Maul36057.
        // HARNESS SETUP: direct threat so O'mrogg swings at the adjacent player.
        h.engage(boss);
        h.advance(6.0);
        eprintln!(
            "OMROGG difficulty{difficulty} maul hits={}",
            observed_casts(&h, boss, BURNING_MAUL_HIT)
        );
        assert!(
            observed_casts(&h, boss, BURNING_MAUL_HIT) > 0,
            "default proc of the Burning Maul aura"
        );
    }
}
