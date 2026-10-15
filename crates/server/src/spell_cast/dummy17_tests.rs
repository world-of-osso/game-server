//! Eight bounded Mainline contracts. Every child is loaded from the real store.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health, PowerType};
use shared::protocol::CombatLogKind;

fn fixture(ids: &[u32], roots: &[u32], class: u8) -> Harness {
    //Only random variance is pinned; IDs, amounts, masks, clocks and costs remain native.
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
        .insert(crate::class_progression::KnownSpells::fixture(
            roots.to_vec(),
        ));
    h
}
fn execute(h: &mut Harness, id: u32, target: Option<Entity>) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, target).unwrap();
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
fn owned(h: &Harness, target: Entity, caster: Entity, id: u32) -> bool {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == id && aura.caster == caster)
        })
}
fn stack_count(h: &Harness, id: u32) -> u32 {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == id)
        .map_or(0, |a| u32::from(a.stacks))
}
fn incoming(h: &mut Harness, target: Entity) {
    let attacker = h.npc;
    h.app.world_mut().entity_mut(attacker).insert((
        shared::components::Mana {
            current: 10000.0,
            max: 10000.0,
        },
        crate::stats::CharacterStats {
            spell_power: 100.0,
            base_mana: 10000.0,
            ..default()
        },
    ));
    let spell = world_db().unwrap().store.get(133).unwrap();
    super::execute_cast(h.app.world_mut(), attacker, spell, Some(target)).unwrap();
    h.advance(15);
}

#[test]
fn dummy17_shadow_bolt_grants_one_native_shard_only_in_demonology() {
    for spec in [265, 266, 267, 0] {
        let mut h = fixture(&[686, 194192], &[686], 9);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ActiveSpec(spec));
        h.set_power(PowerType::SoulShards, 0.0);
        let npc = h.npc;
        execute(&mut h, 686, Some(npc));
        assert_eq!(
            h.power(PowerType::SoulShards),
            if spec == 266 { 10.0 } else { 0.0 }
        );
        h.advance(15);
        assert_eq!(
            h.inbox
                .log
                .iter()
                .filter(|e| e.spell_id == Some(194192) && e.kind == CombatLogKind::Energize)
                .count(),
            usize::from(spec == 266)
        );
    }
}

#[test]
fn dummy17_conflagrate_grants_named_fragments_and_ranked_backdraft_uses() {
    for ranked in [false, true] {
        let mut h = fixture(
            &[17962, 245330, 196406, 117828, 29722, 244670],
            &[17962, 29722],
            9,
        );
        let caster = h.caster;
        if ranked {
            apply(&mut h, caster, caster, 196406, u32::MAX);
        }
        h.set_power(PowerType::SoulShards, 0.0);
        let npc = h.npc;
        execute(&mut h, 17962, Some(npc));
        h.advance(2);
        assert_eq!(h.power(PowerType::SoulShards), 5.0);
        assert_eq!(stack_count(&h, 117828), u32::from(ranked));
        if ranked {
            execute(&mut h, 17962, Some(npc));
            execute(&mut h, 17962, Some(npc));
            assert_eq!(stack_count(&h, 117828), 2);
            for remaining in [1, 0] {
                let spell = world_db().unwrap().store.get(29722).unwrap();
                assert_eq!(super::cast_time_ms(h.app.world(), caster, spell), 1400);
                h.cast_and_deliver(29722, Some(npc));
                let cast = h
                    .app
                    .world()
                    .get::<shared::casting::CastState>(caster)
                    .unwrap();
                assert!((cast.duration - 1.4).abs() < 0.001);
                h.advance(45);
                assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
                assert_eq!(stack_count(&h, 117828), remaining);
            }
            assert_eq!(stack_count(&h, 117828), 0);
            let spell = world_db().unwrap().store.get(29722).unwrap();
            assert_eq!(super::cast_time_ms(h.app.world(), caster, spell), 2000);
        }
    }
}

#[test]
fn dummy17_soul_fire_grants_named_shard_and_applies_only_selected_dot_variant() {
    for wither in [false, true] {
        let mut h = fixture(&[6353, 281490, 157736, 445465, 445474], &[6353], 9);
        let caster = h.caster;
        if wither {
            apply(&mut h, caster, caster, 445465, u32::MAX);
        }
        h.set_power(PowerType::SoulShards, 0.0);
        let npc = h.npc;
        execute(&mut h, 6353, Some(npc));
        h.advance(15);
        assert_eq!(h.power(PowerType::SoulShards), 10.0);
        assert_eq!(owned(&h, npc, caster, 157736), !wither);
        assert_eq!(owned(&h, npc, caster, 445474), wither);
        h.advance(65);
        let dot = if wither { 445474 } else { 157736 };
        assert!(
            h.inbox
                .log
                .iter()
                .any(|e| e.spell_id == Some(dot) && e.periodic && e.kind == CombatLogKind::Damage)
        );
    }
}

#[test]
fn dummy17_blood_boil_applies_plague_to_hit_enemies_unless_rank_replaces_it() {
    for replaced in [false, true] {
        let roots = if replaced {
            vec![50842, 212744]
        } else {
            vec![50842]
        };
        let mut h = fixture(&[50842, 55078], &roots, 6);
        let caster = h.caster;
        let near = spawn_npc(&mut h.app, 5.0);
        let far = spawn_npc(&mut h.app, 50.0);
        let ally = spawn_player(&mut h.app, 6, 4.0);
        execute(&mut h, 50842, None);
        let npc = h.npc;
        assert_eq!(owned(&h, npc, caster, 55078), !replaced);
        assert_eq!(owned(&h, near, caster, 55078), !replaced);
        assert!(!owned(&h, far, caster, 55078));
        assert!(!owned(&h, ally, caster, 55078));
    }
}

#[test]
fn dummy17_marrowrend_generates_three_bone_shield_stacks_up_to_native_cap() {
    let mut h = fixture(&[195182, 195181, SUNWELL_RADIANCE], &[195182], 6);
    apply_native_enemy_dodge_reduction(&mut h);
    let npc = h.npc;
    for expected in [3, 6, 9, 10] {
        h.set_power(PowerType::Runes, 6.0);
        if let Some(mut runes) = h.app.world_mut().get_mut::<super::runes::Runes>(h.caster) {
            *runes = super::runes::Runes::new(6);
        }
        execute(&mut h, 195182, Some(npc));
        assert_eq!(stack_count(&h, 195181), expected);
    }
}

#[test]
fn dummy17_lifebloom_expires_or_dispels_into_named_heal_but_not_cancel_or_death() {
    use crate::auras::RemoveReason;
    for reason in [
        RemoveReason::Expired,
        RemoveReason::Dispelled,
        RemoveReason::Cancelled,
        RemoveReason::Death,
        RemoveReason::Replaced,
    ] {
        let mut h = fixture(&[33763, 33778], &[33763], 11);
        let caster = h.caster;
        execute(&mut h, 33763, Some(caster));
        h.app.world_mut().get_mut::<Health>(caster).unwrap().current = 100.0;
        crate::auras::remove_auras(h.app.world_mut(), caster, reason, |a| a.spell_id == 33763);
        h.advance(1);
        let heals: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(33778) && e.kind == CombatLogKind::Heal)
            .collect();
        let blooms = matches!(reason, RemoveReason::Expired | RemoveReason::Dispelled);
        assert_eq!(heals.len(), usize::from(blooms));
        if blooms {
            assert_eq!(heals[0].amount, 219);
        }
    }
}

#[test]
fn dummy17_lifebloom_native_expiry_blooms_once_after_periodic_heals() {
    let mut h = fixture(&[33763, 33778], &[33763], 11);
    let caster = h.caster;
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = 100.0;
    execute(&mut h, 33763, Some(caster));
    h.advance(305);
    assert!(!owned(&h, caster, caster, 33763));
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(33778) && e.kind == CombatLogKind::Heal)
            .count(),
        1
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(33763) && e.periodic && e.kind == CombatLogKind::Heal)
    );
}

#[test]
fn dummy17_lifebloom_moves_only_its_owners_single_target_without_blooming() {
    let mut h = fixture(&[33763, 33778], &[33763], 11);
    let caster = h.caster;
    let ally = spawn_player(&mut h.app, 11, 3.0);
    let other = spawn_player(&mut h.app, 11, 4.0);
    apply(&mut h, other, caster, 33763, 1);
    execute(&mut h, 33763, Some(caster));
    execute(&mut h, 33763, Some(ally));
    h.advance(1);
    assert!(!owned(&h, caster, caster, 33763));
    assert!(owned(&h, caster, other, 33763));
    assert!(owned(&h, ally, caster, 33763));
    assert!(!h.inbox.log.iter().any(|e| e.spell_id == Some(33778)));
}

#[test]
fn dummy17_improved_regrowth_native_bonus_makes_existing_hot_initial_heal_critical() {
    for (ranked, hot, foreign) in [
        (true, true, false),
        (true, true, true),
        (false, true, false),
        (true, false, false),
    ] {
        let mut h = fixture(&[8936, 231032], &[8936], 11);
        let caster = h.caster;
        if ranked {
            apply(&mut h, caster, caster, 231032, u32::MAX);
        }
        if hot {
            let owner = if foreign {
                spawn_player(&mut h.app, 11, 3.0)
            } else {
                caster
            };
            apply(&mut h, owner, caster, 8936, 1 << 1);
        }
        if ranked && hot {
            h.app
                .world_mut()
                .get_mut::<crate::stats::CharacterStats>(caster)
                .unwrap()
                .crit_pct = 60.0;
        }
        let base = h
            .app
            .world()
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct;
        let chance =
            super::scripts::crit_chance(h.app.world(), caster, caster, 8936).unwrap_or(base);
        assert_eq!(chance, if ranked && hot { 100.0 } else { 0.0 });
        h.app.world_mut().get_mut::<Health>(caster).unwrap().current = 1.0;
        execute(&mut h, 8936, Some(caster));
        h.advance(1);
        let heal = h
            .inbox
            .log
            .iter()
            .find(|e| e.spell_id == Some(8936) && e.kind == CombatLogKind::Heal)
            .unwrap();
        assert_eq!(heal.crit, ranked && hot);
    }
}

#[test]
fn dummy17_cenarion_ward_consumes_on_actual_damage_and_heals_its_ally_not_attacker() {
    let mut h = fixture(&[102351, 102352, 133, 2061], &[102351], 11);
    let caster = h.caster;
    let ally = spawn_player(&mut h.app, 11, 3.0);
    execute(&mut h, 102351, Some(ally));
    incoming(&mut h, ally);
    assert!(!owned(&h, ally, caster, 102351));
    assert!(owned(&h, ally, caster, 102352));
    assert!(!owned(&h, caster, caster, 102352));
    h.advance(45);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(102352) && e.periodic && e.kind == CombatLogKind::Heal)
    );
}

#[test]
fn dummy17_cenarion_ward_ignores_healing_and_fully_absorbed_damage() {
    let mut h = fixture(&[102351, 102352, 133, 2061, 17], &[102351], 11);
    let caster = h.caster;
    execute(&mut h, 102351, Some(caster));
    let healer = spawn_player(&mut h.app, 5, 3.0);
    let heal = world_db().unwrap().store.get(2061).unwrap();
    super::execute_cast(h.app.world_mut(), healer, heal, Some(caster)).unwrap();
    h.advance(1);
    assert!(owned(&h, caster, caster, 102351));
    apply(&mut h, healer, caster, 17, 1);
    incoming(&mut h, caster);
    assert!(owned(&h, caster, caster, 102351));
    assert!(!owned(&h, caster, caster, 102352));
}

#[test]
fn dummy17_immune_enemies_get_no_hit_dependent_fragments_marks_or_bones() {
    for root in [17962, 50842, 195182, 6353] {
        let mut h = fixture(
            &[
                root, 642, 245330, 196406, 117828, 55078, 195181, 281490, 157736, 445474,
            ],
            &[root],
            if matches!(root, 50842 | 195182) { 6 } else { 9 },
        );
        let npc = h.npc;
        apply(&mut h, npc, npc, 642, u32::MAX);
        let caster = h.caster;
        apply(&mut h, caster, caster, 196406, u32::MAX);
        if matches!(root, 17962 | 6353) {
            h.set_power(PowerType::SoulShards, 0.0);
        }
        execute(&mut h, root, if root == 50842 { None } else { Some(npc) });
        h.advance(15);
        assert_eq!(h.health(npc), 5000.0);
        for mark in [55078, 157736, 445474] {
            assert!(!owned(&h, npc, caster, mark), "root{root}, mark{mark}");
        }
        for buff in [195181, 117828] {
            assert!(!owned(&h, caster, caster, buff));
        }
        if root == 17962 {
            assert_eq!(h.power(PowerType::SoulShards), 0.0);
        }
        if root == 6353 {
            assert_eq!(h.power(PowerType::SoulShards), 10.0);
        }
    }
}

#[test]
fn dummy17_cenarion_ward_has_native_initial_plus_four_heals_then_expires_without_retriggering() {
    let mut h = fixture(&[102351, 102352, 133], &[102351], 11);
    let caster = h.caster;
    execute(&mut h, 102351, Some(caster));
    incoming(&mut h, caster);
    h.advance(170);
    assert!(!owned(&h, caster, caster, 102351));
    assert!(!owned(&h, caster, caster, 102352));
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(102352) && e.periodic && e.kind == CombatLogKind::Heal)
            .count(),
        5
    );
    incoming(&mut h, caster);
    assert!(!owned(&h, caster, caster, 102352));
}

#[test]
fn dummy17_shard_casts_grant_only_after_native_cast_completion_not_cancellation() {
    for root in [686, 6353] {
        for cancel in [false, true] {
            let mut h = fixture(&[root, 194192, 281490, 157736, 445474], &[root], 9);
            let caster = h.caster;
            h.app.world_mut().entity_mut(caster).insert(ActiveSpec(266));
            h.set_power(PowerType::SoulShards, 0.0);
            let npc = h.npc;
            h.cast_and_deliver(root, Some(npc));
            assert!(
                h.app
                    .world()
                    .get::<shared::casting::CastState>(caster)
                    .is_some()
            );
            assert_eq!(h.power(PowerType::SoulShards), 0.0);
            if cancel {
                super::cancel_cast(h.app.world_mut(), caster);
            }
            h.advance(140);
            assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
            assert_eq!(
                h.power(PowerType::SoulShards),
                if cancel { 0.0 } else { 10.0 }
            );
            assert_eq!(
                h.inbox.go.iter().filter(|go| go.spell_id == root).count(),
                usize::from(!cancel)
            );
        }
    }
}
