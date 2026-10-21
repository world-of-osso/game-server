//! Current LittleWigs2777a9d Bromach.lua, native70448; no legacy alternate.
use super::*;

const BROMACH: u32 = 184018;
const MAP: u32 = 2451;
const BIT: u32 = 1;
const CALL: u32 = 369605;
const SLAM: u32 = 369703;

fn bromach_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(BROMACH, MAP, BIT, difficulty, None);
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
fn encounter_coverage_bromach_native_slam_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = bromach_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SLAM, Some(boss), false)
            .expect("native369703 admission");
        h.advance(4.0);
        assert_eq!(cast_count(&h, boss, SLAM), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(SLAM)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "native Slam area damage difficulty{difficulty}"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|e| e.source != Some(boss.to_bits()))
        );
    }
}

#[test]
#[ignore = "RED: current Bromach Lua-clock port absent"]
fn encounter_coverage_bromach_first_call_of_the_deep_uses_current_lua_clock() {
    let (mut h, boss, _) = bromach_copy(205);
    h.engage(boss);
    h.advance(6.0);
    assert_eq!(
        cast_count(&h, boss, CALL),
        1,
        "current Lua5.1s request, not melee-only"
    );
}
