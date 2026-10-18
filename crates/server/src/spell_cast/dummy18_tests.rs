//! Native Mainline leech, shield budget and weapon-branch contracts.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

fn fixture(ids: &[u32], class: u8) -> Harness {
    let spells = ids
        .iter()
        .map(|id| {
            let mut spell = world_db().unwrap().store.get(*id).unwrap().clone();
            for effect in &mut spell.effects {
                effect.variance = 0.0;
            }
            spell
        })
        .collect();
    let mut h = harness(spells, &[], class).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(ids.to_vec()));
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 100.0;
    h
}
fn execute(h: &mut Harness, id: u32, target: Entity) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, Some(target)).unwrap();
}
fn apply(h: &mut Harness, caster: Entity, target: Entity, id: u32, mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}
fn periodic_leech(id: u32, class: u8, index: u8, ticks: usize) {
    let mut h = fixture(&[id], class);
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, npc, id, 1 << index);
    h.advance(125);
    let damage: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(id) && e.kind == CombatLogKind::Damage && e.periodic)
        .collect();
    let healing: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(id) && e.kind == CombatLogKind::Heal && e.periodic)
        .collect();
    assert_eq!(damage.len(), ticks);
    assert_eq!(healing.len(), ticks);
    let effect = world_db()
        .unwrap()
        .store
        .get(id)
        .unwrap()
        .effects
        .iter()
        .find(|e| e.index == index)
        .unwrap();
    for (damage, heal) in damage.iter().zip(healing) {
        assert!(damage.amount > 0);
        assert_eq!(heal.target, Some(caster.to_bits()));
        assert_eq!(
            heal.amount,
            (damage.amount as f32 * effect.amplitude).round() as i32
        );
    }
}
#[test]
fn dummy18_vampiric_touch_native_tick_leech() {
    periodic_leech(34914, 5, 1, 2);
}
#[test]
fn dummy18_madness_native_tick_leech() {
    periodic_leech(335467, 5, 2, 3);
}
#[test]
fn dummy18_drain_life_native_tick_leech() {
    periodic_leech(234153, 9, 0, 5);
}
#[test]
fn dummy18_blood_plague_native_tick_leech() {
    periodic_leech(55078, 6, 0, 2);
}
#[test]
fn dummy18_singularity_native_primary_leech_area_dummy_retained() {
    periodic_leech(205179, 9, 0, 3);
}

#[test]
fn dummy18_madness_direct_leech_excludes_absorbs_immunity_and_overkill() {
    for mode in [0, 1, 2, 3] {
        let mut h = fixture(&[335467, 17, 45438], 5);
        let caster = h.caster;
        let npc = h.npc;
        match mode {
            1 => apply(&mut h, caster, npc, 17, 1),
            2 => apply(&mut h, npc, npc, 45438, u32::MAX),
            3 => h.app.world_mut().get_mut::<Health>(npc).unwrap().current = 20.0,
            _ => {}
        }
        h.set_power(PowerType::Insanity, 10_000.0);
        execute(&mut h, 335467, npc);
        h.advance(2);
        let damage =
            h.inbox.log.iter().find(|e| {
                e.spell_id == Some(335467) && e.kind == CombatLogKind::Damage && !e.periodic
            });
        let healing =
            h.inbox.log.iter().find(|e| {
                e.spell_id == Some(335467) && e.kind == CombatLogKind::Heal && !e.periodic
            });
        if mode == 1 || mode == 2 {
            assert!(healing.is_none());
        } else {
            let damage = damage.unwrap();
            let healing = healing.unwrap();
            assert_eq!(
                healing.amount,
                ((damage.amount - damage.overflow) as f32 * 0.3).round() as i32
            );
            if mode == 3 {
                assert_eq!(healing.amount, 6);
            }
        }
    }
}

#[test]
fn dummy18_madness_reapplication_rolls_remaining_damage_only_for_same_owner() {
    let mut h = fixture(&[335467], 5);
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, npc, 335467, 4);
    h.advance(65);
    let initial = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(npc)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 335467)
        .unwrap()
        .effects
        .iter()
        .find(|e| e.index == 2)
        .unwrap()
        .amount;
    assert!(initial > 0.0, "native leech must snapshot spell power");
    apply(&mut h, caster, npc, 335467, 4);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(npc)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 335467)
        .unwrap();
    let refreshed = aura.effects.iter().find(|e| e.index == 2).unwrap().amount;
    assert!((refreshed - initial * 5.0 / 3.0).abs() < 0.01);
    let other = spawn_player(&mut h.app, 5, 2.0);
    h.advance(1);
    apply(&mut h, other, npc, 335467, 4);
    let foreign = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(npc)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 335467 && a.caster == other)
        .unwrap();
    assert!(
        (foreign
            .effects
            .iter()
            .find(|e| e.index == 2)
            .unwrap()
            .amount
            - initial)
            .abs()
            < 0.01
    );
}

#[test]
fn dummy18_ignore_pain_soaks_native_fraction_and_accumulates_to_health_cap() {
    let mut h = fixture(&[190456], 1);
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(shared::components::ActiveSpec(73));
    apply(&mut h, caster, caster, 190456, u32::MAX);
    let (left, absorbed) = crate::auras::absorb_damage(h.app.world_mut(), caster, 4, 100.0);
    assert_eq!((left, absorbed), (50.0, 50.0));
    apply(&mut h, caster, caster, 190456, u32::MAX);
    let (left, absorbed) = crate::auras::absorb_damage(h.app.world_mut(), caster, 4, 1000.0);
    assert_eq!((left, absorbed), (700.0, 300.0));
    assert!(!crate::auras::has_aura(h.app.world(), caster, 190456));
}

#[test]
fn dummy18_improved_conflagrate_grants_two_uses_and_native_cap_four() {
    let mut h = fixture(&[17962, 245330, 196406, 117828, 267115], 9);
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, caster, 196406, u32::MAX);
    apply(&mut h, caster, caster, 267115, u32::MAX);
    h.set_power(PowerType::SoulShards, 0.0);
    for expected in [2, 4, 4] {
        execute(&mut h, 17962, npc);
        let stacks = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(caster)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 117828)
            .unwrap()
            .stacks;
        assert_eq!(stacks, expected);
    }
    assert_eq!(h.power(PowerType::SoulShards), 15.0);
}

#[test]
fn dummy18_obliterate_selects_native_physical_and_frost_weapon_children() {
    for dual in [false, true] {
        let mut h = fixture(
            &[
                49020,
                222024,
                66198,
                325461,
                1264082,
                1264083,
                1264084,
                SUNWELL_RADIANCE,
            ],
            6,
        );
        apply_native_enemy_dodge_reduction(&mut h);
        if dual {
            h.app
                .world_mut()
                .get_mut::<crate::stats::CharacterStats>(h.caster)
                .unwrap()
                .off_hand = Some(crate::item_stats::WeaponDamage::UNARMED);
        }
        let npc = h.npc;
        execute(&mut h, 49020, npc);
        h.advance(2);
        let ids: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|e| e.kind == CombatLogKind::Damage)
            .filter_map(|e| e.spell_id)
            .collect();
        assert_eq!(
            ids,
            if dual {
                // Native effect64 delays group10ms children before20ms children.
                vec![222024, 1264082, 66198, 1264083]
            } else {
                vec![325461, 1264084]
            }
        );
    }
}

#[test]
fn dummy18_drain_life_native_channel_ticks_stop_on_cancellation_or_expiry() {
    for cancel in [false, true] {
        let mut h = fixture(&[234153], 9);
        let caster = h.caster;
        let npc = h.npc;
        h.cast_and_deliver(234153, Some(npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        h.advance(25);
        let count = |h: &Harness| {
            h.inbox
                .log
                .iter()
                .filter(|e| {
                    e.spell_id == Some(234153) && e.kind == CombatLogKind::Heal && e.periodic
                })
                .count()
        };
        assert_eq!(count(&h), 1);
        if cancel {
            super::cancel_cast(h.app.world_mut(), caster);
        }
        h.advance(140);
        assert_eq!(count(&h), if cancel { 1 } else { 5 });
        assert!(!crate::auras::has_aura(h.app.world(), npc, 234153));
    }
}

#[test]
fn dummy18_periodic_leech_copies_post_absorb_damage_without_second_versatility() {
    let mut h = fixture(&[34914, 17], 5);
    let caster = h.caster;
    let npc = h.npc;
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .versatility_pct = 100.0;
    apply(&mut h, caster, npc, 34914, 2);
    let healer = spawn_player(&mut h.app, 5, 2.0);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .healing_power = 10.0;
    apply(&mut h, healer, npc, 17, 1);
    h.advance(65);
    let damage = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(34914) && e.kind == CombatLogKind::Damage)
        .unwrap();
    let heal = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(34914) && e.kind == CombatLogKind::Heal)
        .unwrap();
    assert!(damage.amount > 0 && damage.absorbed > 0);
    assert_eq!(heal.amount, (damage.amount as f32 * 0.3).round() as i32);
}

#[test]
fn dummy18_ignore_pain_versatility_scales_budget_below_cap() {
    let mut h = fixture(&[190456], 1);
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(shared::components::ActiveSpec(73));
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(caster)
        .unwrap();
    stats.attack_power = 1.0;
    stats.versatility_pct = 100.0;
    apply(&mut h, caster, caster, 190456, u32::MAX);
    let (_, absorbed) = crate::auras::absorb_damage(h.app.world_mut(), caster, 4, 100.0);
    let coefficient =
        world_db().unwrap().store.get(190456).unwrap().effects[0].bonus_coefficient_from_ap;
    assert!((absorbed - coefficient * 2.0).abs() < 0.01);
}

#[test]
fn dummy18_native_roots_reach_leech_through_completed_casts() {
    for (root, aura, class) in [
        (34914, 34914, 5),
        (335467, 335467, 5),
        (234153, 234153, 9),
        (50842, 55078, 6),
        (205179, 205179, 9),
    ] {
        let mut h = fixture(&[root, aura, 232698], class);
        if root == 335467 {
            // Native Devouring Plague requires form28. Unlike Vampiric Touch,
            // it lacks AllowCasterForm; use native Shadowform's form effect.
            let caster = h.caster;
            apply(&mut h, caster, caster, 232698, 1 << 1);
            h.set_power(PowerType::Insanity, 10_000.0);
        }
        let npc = h.npc;
        h.cast_and_deliver(root, Some(npc));
        h.advance(105);
        assert!(
            h.inbox.failed.is_empty(),
            "root {root}: {:?}",
            h.inbox.failed
        );
        assert!(
            h.inbox
                .log
                .iter()
                .any(|e| e.spell_id == Some(aura) && e.kind == CombatLogKind::Damage && e.periodic),
            "root {root} damage"
        );
        assert!(
            h.inbox.log.iter().any(|e| e.spell_id == Some(aura)
                && e.kind == CombatLogKind::Heal
                && e.periodic
                && e.target == Some(h.caster.to_bits())),
            "root {root} healing"
        );
    }
}
