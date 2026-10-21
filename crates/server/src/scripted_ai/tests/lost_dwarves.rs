//! Current LittleWigs2777a9d; project policy LOST_DWARVES_SURRENDER_AT_10PCT_LUA.
use super::*;

const MAP: u32 = 2451;
const DWARVES: [u32; 3] = [184581, 184582, 184580];

fn dwarf_copy(entry: u32, difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(entry, MAP, 0, difficulty, None);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    let wdt = h.app.world().resource::<MapStore>().maps[&MAP].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(MAP, wdt)]),
    ));
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|g| g.caster == boss.to_bits() && g.spell_id == spell)
        .count()
}

#[test]
fn encounter_coverage_lost_dwarves_native_cleave_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = dwarf_copy(DWARVES[0], difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, 369563, Some(player), false)
            .expect("native369563");
        h.advance(4.5);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(369563)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "actual cone damage difficulty{difficulty}"
        );
    }
}

#[test]
#[ignore = "RED: Lost Dwarves source clocks absent"]
fn encounter_coverage_lost_dwarves_first_cleave_uses_current_lua_clock() {
    let (mut h, boss, _) = dwarf_copy(DWARVES[0], 205);
    h.engage(boss);
    h.advance(12.0);
    assert_eq!(
        cast_count(&h, boss, 369563),
        1,
        "current Lua8.5s first Cleave"
    );
}

#[test]
#[ignore = "RED: LOST_DWARVES_SURRENDER_AT_10PCT_LUA completion absent"]
fn encounter_coverage_lost_dwarves_overkill_surrenders_alive_and_all_three_grant_credit() {
    for difficulty in [1, 205] {
        let (mut h, baelog, map) = dwarf_copy(DWARVES[0], difficulty);
        let conn =
            Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let (eric, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[1], map, difficulty);
        let (olaf, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[2], map, difficulty);
        for (index, actor) in [baelog, eric, olaf].into_iter().enumerate() {
            h.engage(actor);
            h.hit_pct(actor, 200.0);
            h.advance(0.1);
            assert!(
                h.app
                    .world()
                    .get::<crate::combat::DeathState>(actor)
                    .is_none(),
                "surrender never requires death actor{actor:?}"
            );
            assert!(h.health(actor).current > 0.0);
            let expected = if index == 2 { 1 } else { 0 };
            assert_eq!(
                h.app
                    .world()
                    .resource::<crate::maps::InstanceManager>()
                    .completed_encounters(map),
                expected,
                "no credit until all three surrender"
            );
        }
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, 0),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|e| e.encounter_id == 2555 && e.success)
                .count(),
            1
        );
        assert!(h.inbox(1).ends.is_empty(), "copy-isolated credit");
    }
}
