//! Mainline native paths: actor bindings, successful spend and owned snapshots.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Npc, PowerType};
use shared::protocol::CombatLogKind;

fn apply(h: &mut Harness, caster: Entity, target: Entity, spell: u32, mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn actor_data(h: &mut Harness) {
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
    h.app
        .insert_resource(super::guardian::load_guardian_spells(&conn).unwrap());
}

fn actors(h: &mut Harness, entry: u32) -> Vec<Entity> {
    let world = h.app.world_mut();
    world
        .query::<(Entity, &Npc, &super::summon::SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == entry && summon.owner == h.caster)
        .map(|(unit, _, _)| unit)
        .collect()
}

fn damage(h: &Harness, spell: u32, target: Entity) -> Vec<i32> {
    h.inbox
        .log
        .iter()
        .filter(|e| {
            e.spell_id == Some(spell)
                && e.target == Some(target.to_bits())
                && e.kind == CombatLogKind::Damage
        })
        .map(|e| e.amount)
        .collect()
}

#[test]
fn dummy12_feral_spirit_summons_two_authentic_wolves_and_expires() {
    let mut h = harness(Vec::new(), &[51533, 228562, 58867, 58875, 58876], 7).unwrap();
    actor_data(&mut h);
    h.cast_and_deliver(51533, Some(h.npc));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let wolves = actors(&mut h, 29264);
    assert_eq!(wolves.len(), 2);
    for wolf in &wolves {
        assert!(
            crate::auras::has_aura(h.app.world(), *wolf, 58875),
            "archive Spirit Walk cast"
        );
    }
    h.advance(400);
    assert!(
        wolves
            .iter()
            .all(|wolf| h.app.world().get_entity(*wolf).is_err())
    );
}

#[test]
fn dummy12_gargoyle_native_attack_crit_and_successful_owner_spend() {
    let mut h = harness(
        Vec::new(),
        &[49206, 51963, 61777, 1242147, 211947, 47541, 47632],
        6,
    )
    .unwrap();
    actor_data(&mut h);
    h.select(Some(h.npc));
    h.advance(4);
    h.cast_and_deliver(49206, Some(h.npc));
    let gargoyle = actors(&mut h, 27829)[0];
    // Explicit actor stats isolate this script; non-demon stat inheritance is not claimed.
    h.app
        .world_mut()
        .entity_mut(gargoyle)
        .insert(crate::stats::CharacterStats {
            attack_power: 100.0,
            ..default()
        });
    h.set_power(PowerType::RunicPower, 1000.0);
    let spell = world_db().unwrap().store.get(47541).unwrap();
    super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).unwrap();
    assert_eq!(h.power(PowerType::RunicPower), 700.0);
    h.advance(100);
    let bolts: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(51963) && e.kind == CombatLogKind::Damage)
        .collect();
    assert!(!bolts.is_empty());
    assert!(
        bolts.iter().all(|e| e.crit),
        "authored always-critical attack"
    );
    assert!(
        bolts
            .iter()
            .any(|e| varied(34.776 * 2.0 * 1.3).contains(&(e.amount as f32))),
        "30 spent displayed RP =>30% bonus: {bolts:?}"
    );
    // A refused cast cannot increase the guardian's bonus.
    h.set_power(PowerType::RunicPower, 0.0);
    assert!(super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).is_err());
    h.inbox.log.clear();
    h.advance(80);
    assert!(
        damage(&h, 51963, h.npc)
            .iter()
            .all(|&n| varied(34.776 * 2.0 * 1.3).contains(&(n as f32)))
    );
}

#[test]
fn dummy12_dispatch_and_eviscerate_damage_use_combo_spent_not_left() {
    for spell_id in [2098, 196819] {
        for points in [1.0, 3.0, 5.0] {
            let mut h = harness(Vec::new(), &[spell_id], 4).unwrap();
            h.set_power(PowerType::ComboPoints, points);
            h.set_power(PowerType::Energy, 100.0);
            let spell = world_db().unwrap().store.get(spell_id).unwrap();
            let base =
                super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
            let before = h.health(h.npc);
            super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).unwrap();
            assert_eq!(h.power(PowerType::ComboPoints), 0.0);
            assert!(
                varied(base * points).contains(&(before - h.health(h.npc))),
                "spell={spell_id}, points={points}, loss={}",
                before - h.health(h.npc)
            );
        }
    }
}

#[test]
fn dummy12_rake_bleed_snapshots_stealth_bonus_across_later_rank_removal() {
    for empowered in [false, true] {
        let mut h = harness(Vec::new(), &[1822, 155722, 231052, 5215, 768, 163505], 11).unwrap();
        let caster = h.caster;
        apply(&mut h, caster, caster, 768, u32::MAX);
        if empowered {
            apply(&mut h, caster, caster, 5215, 1);
        }
        let before = h.health(h.npc);
        super::execute_cast(
            h.app.world_mut(),
            caster,
            world_db().unwrap().store.get(1822).unwrap(),
            Some(h.npc),
        )
        .unwrap();
        assert!(h.health(h.npc) < before);
        crate::auras::remove_auras(
            h.app.world_mut(),
            caster,
            crate::auras::RemoveReason::Cancelled,
            |a| a.spell_id == 5215 || a.spell_id == 231052,
        );
        h.advance(135);
        let ticks = damage(&h, 155722, h.npc);
        assert_eq!(ticks.len(), 2);
        assert!(
            ticks
                .iter()
                .all(|&n| varied(29.4 * if empowered { 1.6 } else { 1.0 }).contains(&(n as f32))),
            "{ticks:?}"
        );
    }
}

#[test]
fn dummy12_unstable_affliction_vulnerability_is_owned_not_stacked_and_expires() {
    let mut h = harness(Vec::new(), &[30108, 233490, 108853], 9).unwrap();
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, caster, target, 233490, 1);
    apply(&mut h, caster, target, 233490, 1);
    let spell = world_db().unwrap().store.get(108853).unwrap();
    let before = h.health(h.npc);
    super::cast_triggered(h.app.world_mut(), caster, 108853, Some(h.npc), 0);
    assert!(varied(197.25 * 1.1).contains(&(before - h.health(h.npc))));
    let foreign = spawn_player(&mut h.app, 9, 0.0);
    let before = h.health(h.npc);
    super::cast_triggered(h.app.world_mut(), foreign, spell.id, Some(h.npc), 0);
    assert!(varied(197.25).contains(&(before - h.health(h.npc))));
    h.advance(350);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 233490));
    let before = h.health(h.npc);
    super::cast_triggered(h.app.world_mut(), caster, spell.id, Some(h.npc), 0);
    assert!(varied(197.25).contains(&(before - h.health(h.npc))));
}

#[test]
fn dummy12_shield_of_righteous_applies_native_armor_buff_and_expires() {
    let mut h = harness(Vec::new(), &[53600, 132403, 403460, 407467], 2).unwrap();
    h.set_power(PowerType::HolyPower, 3.0);
    let caster = h.caster;
    h.cast_and_deliver(53600, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), caster, 132403));
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::StatModifiers>(caster)
            .unwrap()
            .armor_from_primary_stat_pct,
        160.0
    );
    assert_eq!(h.power(PowerType::HolyPower), 0.0);
    h.advance(110);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 132403));
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::StatModifiers>(caster)
            .unwrap()
            .armor_from_primary_stat_pct,
        0.0
    );
}

#[test]
fn dummy12_butchery_talent_bleeds_hit_enemies_only() {
    for talented in [false, true] {
        let mut h = harness(Vec::new(), &[212436, 459868, 459870], 3).unwrap();
        if !talented {
            h.app.world_mut().entity_mut(h.caster).insert(
                crate::class_progression::KnownSpells::fixture(vec![212436, 459870]),
            );
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                crate::auras::RemoveReason::Cancelled,
                |a| a.spell_id == 459868,
            );
        }
        let near = spawn_npc(&mut h.app, 4.0);
        let far = spawn_npc(&mut h.app, 30.0);
        let ally = spawn_player(&mut h.app, 3, 2.0);
        h.set_power(PowerType::Focus, 100.0);
        h.cast_and_deliver(212436, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        for target in [h.npc, near] {
            assert_eq!(
                crate::auras::has_aura(h.app.world(), target, 459870),
                talented
            );
        }
        for target in [far, ally] {
            assert!(!crate::auras::has_aura(h.app.world(), target, 459870));
        }
        h.advance(85);
        assert_eq!(damage(&h, 459870, near).is_empty(), !talented);
        h.advance(300);
        assert!(!crate::auras::has_aura(h.app.world(), near, 459870));
    }
}
