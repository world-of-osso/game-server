use super::test_harness::*;
use crate::auras::ActiveAuras;
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};
use shared::protocol::CombatLogKind;

#[test]
fn discipline_flash_heal_applies_atonement_even_at_full_health() {
    let mut h = harness(Vec::new(), &[2061, 194384, 81749, 94472], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    let ally = spawn_player(&mut h.app, 5, 1.0);
    h.cast_and_deliver(2061, Some(ally));
    h.advance(80);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(ally)
            .unwrap()
            .iter()
            .any(|a| { a.spell_id == 194384 && a.caster == h.caster })
    );
}

#[test]
fn shield_absorbs_hit_and_smite_heals_its_atoned_ally() {
    let mut h = harness(Vec::new(), &[17, 139, 194384, 81749, 94472, 585, 133], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    let ally = spawn_player(&mut h.app, 5, 1.0);
    {
        let mut health = h.app.world_mut().get_mut::<Health>(ally).unwrap();
        health.max = 1000.0;
        health.current = 1000.0;
    }
    {
        let mut health = h.app.world_mut().get_mut::<Health>(h.npc).unwrap();
        health.max = 10000.0;
        health.current = 10000.0;
    }
    h.cast_and_deliver(17, Some(ally));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(ally)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 194384 && a.caster == h.caster)
    );
    let fire = world_db().unwrap().spell_store().get(133).unwrap().clone();
    let ctx = super::effects::EffectCtx {
        caster: h.npc,
        spell: &fire,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_damage(h.app.world_mut(), &ctx, ally, 600.0, false, false);
    let after_hit = h.health(ally);
    assert!(
        after_hit > 400.0 && after_hit < 1000.0,
        "shield did not absorb a portion: {after_hit}"
    );
    h.advance(35);
    h.cast_and_deliver(585, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.health(ally) > after_hit,
        "Smite did not heal the marked ally"
    );
    let damage = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(585) && e.kind == CombatLogKind::Damage)
        .expect("Smite damage")
        .amount;
    let healing = h
        .inbox
        .log
        .iter()
        .find(|e| {
            e.spell_id == Some(94472)
                && e.target == Some(ally.to_bits())
                && e.kind == CombatLogKind::Heal
        })
        .expect("Atonement heal")
        .amount;
    assert_eq!(healing, (damage as f32 * 0.46).round() as i32);
}
