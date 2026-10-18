//! Native Death Touch kills the queued Flame Shock recipient without despawning it.
use shared::components::Mana;

use super::test_harness::*;
use crate::auras::has_aura;

const FLAME_SHOCK_PARENT: u32 = 470411;
const FLAME_SHOCK_CHILD: u32 = 188389;
const DEATH_TOUCH: u32 = 5;
const NATURAL_FIXTURE_MANA: f32 = 10_000.0;

fn queue_child_then_kill_target() -> Harness {
    let data = world_db().unwrap();
    let spells = [FLAME_SHOCK_PARENT, FLAME_SHOCK_CHILD, DEATH_TOUCH]
        .into_iter()
        .map(|id| data.store.get(id).expect("native cast chain").clone())
        .collect();
    let mut h = harness(spells, &[], 7).unwrap();
    for spell in [FLAME_SHOCK_PARENT, DEATH_TOUCH] {
        super::creature::creature_cast(h.app.world_mut(), h.caster, spell, Some(h.npc), false)
            .unwrap();
    }
    assert_eq!(h.health(h.npc), 0.0);
    assert!(h.app.world().get_entity(h.npc).is_ok());
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        NATURAL_FIXTURE_MANA,
    );
    h.advance(1);
    assert!(h.app.world().get_entity(h.npc).is_ok());
    h
}

#[test]
fn native_trigger64_dead_present_recipient_refuses_before_payment() {
    let h = queue_child_then_kill_target();
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        NATURAL_FIXTURE_MANA,
    );
}

#[test]
fn native_trigger64_dead_present_recipient_does_not_receive_flame_shock() {
    let h = queue_child_then_kill_target();
    assert!(!has_aura(h.app.world(), h.npc, FLAME_SHOCK_CHILD));
}
