//! Native victim flags, TC a352b1fa Unit.cpp2749–2861/CreatureData.h346–348.
use bevy::prelude::*;
use shared::components::{Level, UnitLevel};
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

use crate::spell_cast::test_harness::{Harness, harness, world_db};

const ICE_TOMB: u32 = 36_980;
const GRANDMASTER_DUMMY: u32 = 31_144;
const NO_PARRY: u32 = 0x4;
const NO_BLOCK: u32 = 0x10;
const SWINGS: usize = 256;

fn native_target(entry: u32) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[], 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    let target =
        crate::zones::summon_creature(h.app.world_mut(), caster, entry, [3.0, 0.0, 0.0, 0.0])
            .expect("native level80 template");
    assert_eq!(h.app.world().get::<UnitLevel>(target).unwrap().0, 80);
    (h, target)
}

fn ordinary_swings(h: &mut Harness, target: Entity) -> Vec<CombatLogEvent> {
    let mut cursor = h.inbox.log.len();
    let mut swings = Vec::new();
    h.attack(target);
    // Bound delivery at3s per native2s swing; never alter a swing timer or RNG.
    for _ in 0..SWINGS * 60 {
        h.advance(1);
        swings.extend(
            h.inbox.log[cursor..]
                .iter()
                .filter(|event| {
                    event.source == Some(h.caster.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id.is_none()
                        && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
                })
                .cloned(),
        );
        cursor = h.inbox.log.len();
        if swings.len() == SWINGS {
            h.stop_attack();
            h.advance(3);
            assert!(h.health(target) > 0.0, "native target survives the sample");
            return swings;
        }
    }
    panic!("only {} ordinary swings at native{target:?}", swings.len());
}

fn parry_or_block(event: &CombatLogEvent) -> bool {
    event.kind == CombatLogKind::Miss(MissKind::Parry) || event.blocked > 0
}

#[test]
fn native_autoattack_npc_no_parry_and_no_block_flags_reject_both_outcomes() {
    let (mut h, target) = native_target(ICE_TOMB);
    let flags = h
        .app
        .world()
        .resource::<crate::world_data::WorldData>()
        .templates[&ICE_TOMB]
        .flags_extra;
    assert_eq!(flags & (NO_PARRY | NO_BLOCK), NO_PARRY | NO_BLOCK);
    let swings = ordinary_swings(&mut h, target);
    let forbidden: Vec<_> = swings
        .iter()
        .filter(|event| parry_or_block(event))
        .collect();
    // At6% parry+3% block, a faulty ordinary table escapes this RED with
    // probability0.91^256=3.27e-11. Correct masks forbid both deterministically.
    assert!(
        forbidden.is_empty(),
        "forbidden native Ice Tomb outcomes: {forbidden:?}"
    );
}

#[test]
fn native_autoattack_unflagged_creature_retains_parry_or_block() {
    let (mut h, target) = native_target(GRANDMASTER_DUMMY);
    let flags = h
        .app
        .world()
        .resource::<crate::world_data::WorldData>()
        .templates[&GRANDMASTER_DUMMY]
        .flags_extra;
    assert_eq!(flags & (NO_PARRY | NO_BLOCK), 0);
    let swings = ordinary_swings(&mut h, target);
    // Requiring either outcome, not each separately, has false-fail
    // probability0.91^256=3.27e-11 with the unchanged real RNG and clocks.
    assert!(swings.iter().any(parry_or_block), "{swings:?}");
}
