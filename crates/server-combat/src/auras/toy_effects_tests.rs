//! Concrete Retail toy records; isolated effects do not certify the toy's scripts.
use super::*;
use crate::spell_cast::test_harness::{harness, world_db};
use shared::casting::CastState;

fn apply_effect(world: &mut World, unit: Entity, spell_id: u32, index: u8) {
    apply_aura(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
}

#[test]
fn toyfx3_no_actions_blocks_prevention_four_and_interrupts_only_matching_casts() {
    // MiniZep46709/65451 effect3 (MiscValue0=0), Racer37710/49352
    // effect2 (MiscValue0=85): both mean NO_ACTIONS, not a MiscValue mask.
    let mut h = harness(Vec::new(), &[116, 133, 65451, 49352], 8).unwrap();
    let frostbolt = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(116)
        .unwrap()
        .clone();
    let fireball = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    assert_eq!(frostbolt.categories.as_ref().unwrap().prevention_type, 5);
    assert_eq!(fireball.categories.as_ref().unwrap().prevention_type, 1);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CastState::normal(116, h.npc.to_bits(), 3.0, true));
    apply_effect(h.app.world_mut(), h.caster, 65451, 3);
    assert!(cast_blocker(h.app.world(), h.caster, &frostbolt).is_some());
    assert!(cast_blocker(h.app.world(), h.caster, &fireball).is_none());
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CastState::normal(133, h.npc.to_bits(), 3.0, true));
    apply_effect(h.app.world_mut(), h.caster, 49352, 2);
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    remove_auras(h.app.world_mut(), h.caster, RemoveReason::Cancelled, |a| {
        a.spell_id == 65451
    });
    assert!(cast_blocker(h.app.world(), h.caster, &frostbolt).is_some());
    remove_auras(h.app.world_mut(), h.caster, RemoveReason::Cancelled, |a| {
        a.spell_id == 49352
    });
    assert!(cast_blocker(h.app.world(), h.caster, &frostbolt).is_none());
}

#[test]
fn toyfx3_periodic_dummy_uses_data_period_and_channel_target_and_stops_on_remove() {
    // Gin-Ji Knife Set90175:131568 period330ms ->131569. Its final corpse
    // chopping dummy131571 still needs a script. Triggered casts emit no SpellGo
    // in this runtime, and that dummy has no observable result. Only in this
    // mechanism fixture, give131569 Fireball133's real damage effect to observe
    // delivery/target/cadence; no production data or toy script is replaced.
    let data = world_db().unwrap();
    let mut trigger = data.store.get(131569).unwrap().clone();
    trigger.effects = data.store.get(133).unwrap().effects.clone();
    let mut h = harness(vec![trigger], &[131568], 8).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CastState::channel(
            131568,
            h.npc.to_bits(),
            10.0,
            0.33,
            true,
        ));
    let now = aura_now(h.app.world());
    apply_effect(h.app.world_mut(), h.caster, 131568, 0);
    periodic::run_due_ticks(h.app.world_mut(), h.caster, now + 0.329);
    h.advance(2);
    assert_eq!(h.health(h.npc), 5000.0);
    periodic::run_due_ticks(h.app.world_mut(), h.caster, now + 0.33);
    h.advance(2);
    let ticks: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(131569) && event.kind == shared::protocol::CombatLogKind::Damage
        })
        .collect();
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].target, Some(h.npc.to_bits()));
    assert!(h.health(h.npc) < 5000.0);
    periodic::run_due_ticks(h.app.world_mut(), h.caster, now + 0.99);
    h.advance(2);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|event| event.spell_id == Some(131569)
                && event.kind == shared::protocol::CombatLogKind::Damage)
            .count(),
        3
    );
    remove_auras(h.app.world_mut(), h.caster, RemoveReason::Cancelled, |a| {
        a.spell_id == 131568
    });
    periodic::run_due_ticks(h.app.world_mut(), h.caster, now + 2.0);
    h.advance(2);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|event| event.spell_id == Some(131569)
                && event.kind == shared::protocol::CombatLogKind::Damage)
            .count(),
        3
    );
}

#[test]
fn toyfx3_conditional_parasol_is_not_an_unconditional_periodic_cast() {
    // Stylish Black Parasol182694 says "next time you fall". Trigger341677
    // consumes341678; DB2 has no falling predicate, so generic ticks must not fire it.
    let mut h = harness(Vec::new(), &[341678, 341677], 8).unwrap();
    let now = aura_now(h.app.world());
    apply_effect(h.app.world_mut(), h.caster, 341678, 0);
    periodic::run_due_ticks(h.app.world_mut(), h.caster, now + 2.0);
    h.advance(2);
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 341677));
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(h.caster)
            .unwrap()
            .has_spell(341678)
    );
    assert!(
        crate::toy_catalog::unsupported_types(h.app.world(), 341678)
            .iter()
            .any(|r| r.contains("conditional periodic dummy"))
    );
}
