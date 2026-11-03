use crate::combat::auras;
use crate::combat::spell_cast::{self, area_trigger::SpellAreaTrigger};
use crate::test_support::{Harness, ObservedCombatEvents, harness, spawn_player};
use bevy::prelude::*;
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

const ROTTEN_SUPPLIES: u32 = 1297699;
const ROTTEN_GROUND_FIELD: u32 = 1297700;
const ROTTEN_GROUND: u32 = 1297701;
const ROTTEN_GROUND_PROPERTIES: i32 = 43133;

fn ticks(h: &Harness, caster: Entity, target: Entity) -> usize {
    h.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(ROTTEN_GROUND)
                && event.kind == CombatLogKind::Damage
        })
        .count()
}

fn move_to(h: &mut Harness, unit: Entity, y: f32) {
    *h.app.world_mut().get_mut::<Position>(unit).unwrap() = Position { x: 0.0, y, z: 0.0 };
}

fn fields(h: &mut Harness) -> Vec<(u32, Position)> {
    h.app
        .world_mut()
        .query::<(&SpellAreaTrigger, &Position)>()
        .iter(h.app.world())
        .map(|(trigger, position)| (trigger.spell_id, *position))
        .collect()
}

#[test]
fn rotten_supplies_impact_leaves_ground_that_only_afflicts_units_inside() {
    let mut h = harness(
        Vec::new(),
        &[ROTTEN_SUPPLIES, ROTTEN_GROUND_FIELD, ROTTEN_GROUND],
        1,
    )
    .unwrap();
    let (radius, _, entry_spell) = spell_cast::script_registry::area_trigger_binding(
        h.app.world(),
        ROTTEN_GROUND_FIELD,
        ROTTEN_GROUND_PROPERTIES,
    )
    .expect("inferred 43133 binding");
    assert_eq!(entry_spell, ROTTEN_GROUND);
    // DB2 1297700 effect 179 targets 138 (ground under the destination).
    h.app
        .insert_resource(crate::combat::test_support::GroundMap::flat_arena(0.0));
    let gatherer = h.npc;
    let hit = h.caster;
    let inside = spawn_player(&mut h.app, 1, 0.0);
    let outside = spawn_player(&mut h.app, 1, 0.0);
    move_to(&mut h, inside, radius * 0.5);
    move_to(&mut h, outside, radius + 3.0);
    for unit in [hit, inside, outside] {
        let mut health = h.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.max = 10_000_000.0;
        health.current = health.max;
    }
    h.app.init_resource::<ObservedCombatEvents>();
    spell_cast::cast_triggered(h.app.world_mut(), gatherer, ROTTEN_SUPPLIES, Some(hit), 0);
    // DB2 1297699: Speed 1.0 with the delay-in-seconds attribute.
    h.advance(25);
    assert_eq!(
        fields(&mut h),
        vec![(
            ROTTEN_GROUND_FIELD,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }
        )],
        "one field at the hit player"
    );
    for unit in [hit, inside] {
        assert!(auras::has_aura(h.app.world(), unit, ROTTEN_GROUND));
    }
    assert!(!auras::has_aura(h.app.world(), outside, ROTTEN_GROUND));

    h.advance(60);
    let inside_ticks = ticks(&h, gatherer, inside);
    assert!(
        (2..=4).contains(&inside_ticks),
        "1-second ticks inside: {inside_ticks}"
    );
    assert_eq!(ticks(&h, gatherer, outside), 0);

    move_to(&mut h, inside, radius + 3.0);
    h.advance(2);
    assert!(
        !auras::has_aura(h.app.world(), inside, ROTTEN_GROUND),
        "removed on exit"
    );
    let after_exit = ticks(&h, gatherer, inside);
    h.advance(60);
    assert_eq!(
        ticks(&h, gatherer, inside),
        after_exit,
        "no ticks after leaving"
    );
    assert_eq!(ticks(&h, gatherer, outside), 0);

    // DB2 1297700 lasts 20 s; the holder loses the never-expiring aura with the field.
    assert!(auras::has_aura(h.app.world(), hit, ROTTEN_GROUND));
    h.advance(20 * 20);
    assert!(fields(&mut h).is_empty());
    assert!(
        !auras::has_aura(h.app.world(), hit, ROTTEN_GROUND),
        "removed at field end"
    );
    let final_ticks = ticks(&h, gatherer, hit);
    h.advance(60);
    assert_eq!(ticks(&h, gatherer, hit), final_ticks);
}
