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
fn atonement_transfers_critical_smite_before_absorb_but_not_fireball() {
    let mut h = harness(Vec::new(), &[17, 194384, 81749, 94472, 585, 133], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    let ally = spawn_player(&mut h.app, 5, 1.0);
    {
        let mut health = h.app.world_mut().get_mut::<Health>(ally).unwrap();
        health.max = 10_000.0;
        health.current = 5_000.0;
    }
    {
        let mut health = h.app.world_mut().get_mut::<Health>(h.npc).unwrap();
        health.max = 100_000.0;
        health.current = 100_000.0;
    }
    h.cast_and_deliver(17, Some(ally));
    h.advance(35);
    super::cast_triggered_with_effect0_amount(h.app.world_mut(), h.npc, 17, Some(h.npc), 1, 10.0);
    h.advance(2);
    h.inbox.log.clear();
    h.cast_and_deliver(585, Some(h.npc));
    h.advance(60);
    let smite = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(585))
        .expect("real Smite damage");
    let transfer = h
        .inbox
        .log
        .iter()
        .find(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(94472)
                && e.target == Some(ally.to_bits())
        })
        .expect("Atonement transfer");
    assert!(
        smite.crit && transfer.crit,
        "critical outcome must be carried: {smite:?} {transfer:?}"
    );
    assert!(
        smite.absorbed > 0 && smite.amount > 0,
        "fixture requires partial absorb: {smite:?}"
    );
    let expected = ((smite.amount + smite.absorbed) as f32 * 0.46).round() as i32;
    assert!(
        (transfer.amount - expected).abs() <= 1,
        "transfer uses pre-absorb damage: {smite:?} {transfer:?}"
    );
    h.advance(35);
    h.inbox.log.clear();
    h.cast_and_deliver(133, Some(h.npc));
    h.advance(80);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(133)),
        "real Fireball damage required"
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(94472)),
        "Fireball is not a SimC Atonement trigger"
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
