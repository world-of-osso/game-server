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
fn master_the_darkness_penance_draws_one_proc_per_four_casts() {
    let mut h = harness(
        Vec::new(),
        &[47540, 47758, 47666, 1253590, 1253591, 1253593],
        5,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1253590,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let mut procs = 0;
    for _ in 0..4 {
        h.cast_and_deliver(47540, Some(h.npc));
        h.advance(250);
        if crate::auras::has_aura(h.app.world(), h.caster, 1253591) {
            procs += 1;
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 1253591,
            );
        }
    }
    assert!(
        h.inbox.failed.is_empty(),
        "all four real Penance casts must succeed: {:?}",
        h.inbox.failed
    );
    assert_eq!(procs, 1, "one success is drawn from each four-card deck");
}

#[test]
fn master_the_darkness_mind_blast_earns_void_shield_and_absorb_discharges() {
    let mut h = harness(
        Vec::new(),
        &[
            17, 8092, 133, 194384, 81749, 94472, 1253591, 1253593, 1253827, 1253828,
        ],
        5,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    let ally_one = spawn_player(&mut h.app, 5, 1.0);
    let ally_two = spawn_player(&mut h.app, 5, 2.0);
    let marked = spawn_player(&mut h.app, 5, 3.0);
    let ally_three = spawn_player(&mut h.app, 5, 4.0);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: marked,
            spell_id: 194384,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1253827,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.cast_and_deliver(8092, Some(h.npc));
    h.advance(80);
    assert!(
        crate::auras::has_aura(h.app.world(), h.caster, 1253591),
        "Mind Blast must earn the native override"
    );
    h.cast_and_deliver(17, Some(h.caster));
    h.advance(5);
    assert!(
        crate::auras::has_aura(h.app.world(), h.caster, 1253593),
        "Shield must resolve to Void Shield"
    );
    let protected: Vec<_> = [ally_one, ally_two, ally_three]
        .into_iter()
        .filter(|&ally| crate::auras::has_aura(h.app.world(), ally, 1253593))
        .collect();
    assert_eq!(
        protected.len(),
        2,
        "Void Shield protects the primary and two unmarked allies"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), marked, 1253593),
        "unmarked allies take priority"
    );
    for ally in protected {
        assert!(
            crate::auras::has_aura(h.app.world(), ally, 194384),
            "Void Shield must apply Atonement"
        );
    }
    assert!(
        !crate::auras::has_aura(h.app.world(), h.caster, 1253591),
        "Void Shield consumes its override"
    );
    let fire = world_db().unwrap().spell_store().get(133).unwrap().clone();
    h.inbox.log.clear();
    super::effects::deal_damage(
        h.app.world_mut(),
        &super::effects::EffectCtx {
            caster: h.npc,
            spell: &fire,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        },
        h.caster,
        200.0,
        false,
        false,
    );
    h.advance(5);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1253828)
                && event.target == Some(h.npc.to_bits())
                && event.amount > 0),
        "actual absorbed damage must discharge at its attacker: {:?}",
        h.inbox.log
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
fn atonement_shadow_word_pain_uses_native_abyssal_reverie() {
    let mut h = harness(Vec::new(), &[17, 194384, 81749, 94472, 589, 373054], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 373054,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let ally = spawn_player(&mut h.app, 5, 1.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1_000.0,
        max: 10_000.0,
    };
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    h.cast_and_deliver(17, Some(ally));
    h.advance(35);
    h.inbox.log.clear();
    h.cast_and_deliver(589, Some(h.npc));
    h.advance(120);
    let index = h
        .inbox
        .log
        .iter()
        .position(|event| {
            event.kind == CombatLogKind::Damage && event.spell_id == Some(589) && event.periodic
        })
        .expect("real Pain tick");
    let damage = &h.inbox.log[index];
    let transfer = h.inbox.log[index + 1..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(94472)
                && event.target == Some(ally.to_bits())
        })
        .expect("Atonement transfer");
    // 70448 Abyssal Reverie effect 0 adds 10% to Shadow transfers.
    let expected = (damage.amount as f32 * 0.46 * 1.10).round() as i32;
    assert!(
        (transfer.amount - expected).abs() <= 1,
        "native Shadow transfer bonus: {damage:?} {transfer:?}"
    );
}

#[test]
fn atonement_smite_uses_native_oracle_and_voidheart_bonuses() {
    for (aura, multiplier) in [(1272359, 1.15_f32), (449887, 1.20_f32)] {
        let mut h = harness(Vec::new(), &[17, 194384, 81749, 94472, 585, aura], 5).unwrap();
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ActiveSpec(256));
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: aura,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        let ally = spawn_player(&mut h.app, 5, 1.0);
        *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
            current: 1_000.0,
            max: 10_000.0,
        };
        *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
            current: 100_000.0,
            max: 100_000.0,
        };
        h.cast_and_deliver(17, Some(ally));
        h.advance(35);
        h.inbox.log.clear();
        h.cast_and_deliver(585, Some(h.npc));
        h.advance(60);
        let damage = h
            .inbox
            .log
            .iter()
            .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(585))
            .expect("real Smite damage");
        let transfer = h
            .inbox
            .log
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(94472)
                    && event.target == Some(ally.to_bits())
            })
            .expect("Atonement transfer");
        let expected = (damage.amount as f32 * 0.46 * multiplier).round() as i32;
        assert!(
            (transfer.amount - expected).abs() <= 1,
            "native talent {aura}: {damage:?} {transfer:?}"
        );
    }
}

#[test]
fn atonement_smite_uses_native_grace_mastery() {
    let mut h = harness(Vec::new(), &[17, 194384, 81749, 94472, 585, 271534], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .mastery_points = 20.0;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 271534,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let ally = spawn_player(&mut h.app, 5, 1.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1_000.0,
        max: 10_000.0,
    };
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    h.cast_and_deliver(17, Some(ally));
    h.advance(35);
    h.inbox.log.clear();
    h.cast_and_deliver(585, Some(h.npc));
    h.advance(60);
    let smite = h
        .inbox
        .log
        .iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(585))
        .expect("real Smite damage");
    let transfer = h
        .inbox
        .log
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(94472)
                && event.target == Some(ally.to_bits())
        })
        .expect("Atonement transfer");
    // 70448 Grace effect 0 is 1.35% per mastery point; 20 points yield 27%.
    let expected = (smite.amount as f32 * 0.46 * 1.27).round() as i32;
    assert!(
        (transfer.amount - expected).abs() <= 1,
        "native Grace must amplify transfer: {smite:?} {transfer:?}"
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
