//! Native70448 Mistweaver shield and Sheilun targeting at the actual cast boundary.
use super::kit_abundance_tests::{add_native_friend, native_party};
use crate::auras::ActiveAuras;
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use crate::spell_info::SpellStore;
use shared::components::Health;
use shared::protocol::CombatLogKind;

fn absorb_remaining(world: &bevy::prelude::World, unit: bevy::prelude::Entity) -> f32 {
    crate::auras::forecast::absorb_remaining(
        world.get::<ActiveAuras>(unit).unwrap(),
        crate::auras::aura_now(world),
    )
}

#[test]
fn life_cocoon_absorbs_hit_on_other_party_member_using_caster_health() {
    let (mut h, healer, party) = native_party(90);
    let target = party[1];
    let world = h.app.world_mut();
    world.get_mut::<Health>(healer).unwrap().max = 1000.0;
    world
        .get_mut::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .versatility_pct = 25.0;
    crate::spell_cast::creature_cast(world, healer, 116849, Some(target), true).unwrap();
    assert_eq!(absorb_remaining(world, target), 600.0); // 1000 * 48% * 1.25
    let spell = world.resource::<SpellStore>().get(133).unwrap().clone();
    let before = world.get::<Health>(target).unwrap().current;
    let ctx = EffectCtx {
        caster: h.npc,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(world, &ctx, target, 100.0, false, false);
    assert_eq!(world.get::<Health>(target).unwrap().current, before);
    assert_eq!(absorb_remaining(world, target), 500.0);
}

#[test]
fn sheilun_heals_primary_and_two_party_members_not_nearby_outsiders() {
    let (mut h, healer, party) = native_party(90);
    let outsider = add_native_friend(&mut h, 209057);
    let primary = party[1];
    for target in party.iter().chain([&outsider, &h.npc]) {
        if let Some(mut hp) = h.app.world_mut().get_mut::<Health>(*target) {
            hp.current = hp.max * 0.5;
        }
    }
    h.app
        .insert_resource(crate::spell_cast::test_harness::ObservedCombatEvents::default());
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 399491, Some(primary), true)
        .unwrap();
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let targets: std::collections::HashSet<_> = events
        .iter()
        .filter(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(399491))
        .filter_map(|e| e.target)
        .collect();
    assert_eq!(targets.len(), 3, "{targets:?}");
    assert!(targets.contains(&primary.to_bits()));
    assert!(
        targets
            .iter()
            .all(|target| party.iter().any(|unit| unit.to_bits() == *target))
    );
    assert!(!targets.contains(&outsider.to_bits()));
    assert!(!targets.contains(&h.npc.to_bits()));
}
