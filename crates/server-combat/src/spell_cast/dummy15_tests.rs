//! Local Mainline native rows, deterministic casts and lifecycle boundaries.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

fn execute(h: &mut Harness, id: u32, target: Option<Entity>) {
    let spell = world_db().unwrap().store.get(id).unwrap();
    super::execute_cast(h.app.world_mut(), h.caster, spell, target).unwrap();
}
fn apply(h: &mut Harness, caster: Entity, target: Entity, id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}
fn pin_known(h: &mut Harness, ids: &[u32]) {
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(ids.to_vec()));
}
fn amounts(h: &Harness, id: u32, target: Entity, kind: CombatLogKind) -> Vec<i32> {
    h.inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(id) && e.target == Some(target.to_bits()) && e.kind == kind)
        .map(|e| e.amount)
        .collect()
}

#[test]
fn dummy15_vampiric_blood_grants_native_max_health_without_refresh_stacking() {
    let mut h = harness_with(Vec::new(), &[55233], 6, true).unwrap();
    let caster = h.caster;
    let before = h.app.world().get::<Health>(caster).unwrap().max;
    execute(&mut h, 55233, None);
    h.advance(1);
    let expected = (before * 1.3).round();
    assert_eq!(h.app.world().get::<Health>(caster).unwrap().max, expected);
    apply(&mut h, caster, caster, 55233);
    h.advance(1);
    assert_eq!(h.app.world().get::<Health>(caster).unwrap().max, expected);
    h.advance(205);
    assert_eq!(h.app.world().get::<Health>(caster).unwrap().max, before);
}

#[test]
fn dummy15_frost_strike_uses_native_two_hand_or_dual_wield_children_not_both() {
    for dual in [false, true] {
        let mut h = harness_with_native_enemy_dodge_reduction(
            Vec::new(),
            &[49143, 222026, 66196, 325464],
            6,
        )
        .unwrap();
        let caster = h.caster;
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .off_hand = dual.then_some(crate::item_stats::WeaponDamage::UNARMED);
        h.set_power(PowerType::RunicPower, 1000.0);
        let target = h.npc;
        execute(&mut h, 49143, Some(target));
        h.advance(1);
        for id in [222026, 66196, 325464] {
            assert_eq!(
                amounts(&h, id, target, CombatLogKind::Damage).len(),
                usize::from(if id == 325464 { !dual } else { dual }),
                "child{id}, dual{dual}"
            );
        }
    }
}

#[test]
fn dummy15_rising_sun_kick_rank_wounds_only_actual_primary_and_expires() {
    for ranked in [false, true] {
        let mut h = harness_with_native_enemy_dodge_reduction(
            Vec::new(),
            &[107428, 185099, 128595, 115804],
            10,
        )
        .unwrap();
        pin_known(&mut h, &[107428]);
        let caster = h.caster;
        if ranked {
            apply(&mut h, caster, caster, 128595);
        }
        h.set_power(PowerType::Chi, 5.0);
        let target = h.npc;
        let other = spawn_npc(&mut h.app, 4.0);
        execute(&mut h, 107428, Some(target));
        h.advance(1);
        assert_eq!(
            crate::auras::has_aura(h.app.world(), target, 115804),
            ranked
        );
        assert!(!crate::auras::has_aura(h.app.world(), other, 115804));
        if ranked {
            assert_eq!(
                h.app
                    .world()
                    .get::<crate::auras::AuraStatMods>(target)
                    .unwrap()
                    .healing_taken_mult,
                0.5
            );
            h.advance(205);
            assert!(!crate::auras::has_aura(h.app.world(), target, 115804));
        }
    }
}

#[test]
fn dummy15_karma_shield_uses_native_half_health_cap_and_damage_depletion() {
    let mut h = harness(Vec::new(), &[122470, 133], 10).unwrap();
    let caster = h.caster;
    let target = h.npc;
    execute(&mut h, 122470, Some(target));
    let spell = world_db().unwrap().store.get(133).unwrap();
    let ctx = super::effects::EffectCtx {
        caster: target,
        spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_damage(h.app.world_mut(), &ctx, caster, 300.0, false, false);
    assert_eq!(h.health(caster), 1000.0);
    super::effects::deal_damage(h.app.world_mut(), &ctx, caster, 300.0, false, false);
    assert_eq!(h.health(caster), 900.0);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 122470));
}

#[test]
fn dummy15_combustion_pyromaniac_snapshots_native_crit_fraction_and_rank_base() {
    for ranked in [false, true] {
        let mut h = harness(Vec::new(), &[190319, 1254512, 133], 8).unwrap();
        pin_known(&mut h, &[190319, 133]);
        let caster = h.caster;
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct = 50.0;
        if ranked {
            apply(&mut h, caster, caster, 1254512);
        }
        execute(&mut h, 190319, None);
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct = 90.0;
        let target = h.npc;
        super::cast_triggered(h.app.world_mut(), caster, 133, Some(target), 1);
        h.advance(10);
        // Native Fireball3.88125SP; Combustion crit;10% base +20% of50% =20%.
        //20% vs10% has disjoint variance bounds, so the absent snapshot always fails.
        let expected = 388.125 * 2.0 * if ranked { 1.2 } else { 1.0 };
        assert!(
            amounts(&h, 133, target, CombatLogKind::Damage)
                .iter()
                .all(|&n| varied(expected).contains(&(n as f32))),
            "rank{ranked}: {:?}",
            amounts(&h, 133, target, CombatLogKind::Damage)
        );
        assert_eq!(amounts(&h, 133, target, CombatLogKind::Damage).len(), 1);
    }
}

#[test]
fn dummy15_high_tide_two_chain_heals_have_no_falloff_then_native_falloff_returns() {
    let mut h = harness(Vec::new(), &[1064, 288675], 7).unwrap();
    pin_known(&mut h, &[1064]);
    let caster = h.caster;
    let ally = spawn_player(&mut h.app, 7, 3.0);
    let other = spawn_player(&mut h.app, 7, 5.0);
    apply(&mut h, caster, caster, 288675);
    for cast in 0..3 {
        for unit in [ally, other] {
            h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 100.0;
        }
        h.app
            .world_mut()
            .get_mut::<shared::components::Mana>(caster)
            .unwrap()
            .current = 10000.0;
        execute(&mut h, 1064, Some(ally));
        h.advance(1);
        let primary = *amounts(&h, 1064, ally, CombatLogKind::Heal).last().unwrap();
        let secondary = *amounts(&h, 1064, other, CombatLogKind::Heal)
            .last()
            .unwrap();
        let ratio = secondary as f32 / primary as f32;
        assert!(
            if cast < 2 {
                (0.94..=1.06).contains(&ratio)
            } else {
                (0.65..=0.75).contains(&ratio)
            },
            "cast{cast}: {primary}/{secondary}"
        );
        assert_eq!(
            crate::auras::has_aura(h.app.world(), caster, 288675),
            cast == 0
        );
    }
}

fn group(h: &mut Harness, members: &[Entity]) {
    let mut online = crate::group::OnlineView::new();
    for (index, &member) in members.iter().enumerate() {
        let id = index as u64 + 1;
        h.app
            .world_mut()
            .entity_mut(member)
            .insert(crate::persistence::ClientId(id));
        online.insert(
            id,
            crate::group::OnlineMember {
                entity: member.to_bits(),
                name: format!("Member{id}"),
                class: 2,
                level: 10,
                portrait: default(),
                dungeon_difficulty: 1,
                raid_difficulty: crate::maps::RaidDifficulty::default(),
            },
        );
    }
    let mut registry = crate::group::GroupRegistry::default();
    for index in 1..members.len() {
        let id = index as u64 + 1;
        registry.invite(1, &format!("Member{id}"), &online);
        registry.respond_invite(id, true);
    }
    h.app.insert_resource(registry);
}

#[test]
fn dummy15_beacon_transfers_only_effective_direct_group_heal_to_owned_mark() {
    let mut h = harness(Vec::new(), &[53563, 53651, 53652, 19750, 139], 2).unwrap();
    let caster = h.caster;
    let beacon = spawn_player(&mut h.app, 2, 4.0);
    let ally = spawn_player(&mut h.app, 2, 3.0);
    group(&mut h, &[caster, beacon, ally]);
    apply(&mut h, caster, beacon, 53563);
    h.app.world_mut().get_mut::<Health>(beacon).unwrap().current = 100.0;
    h.app.world_mut().get_mut::<Health>(ally).unwrap().current = 900.0;
    super::cast_triggered(h.app.world_mut(), caster, 19750, Some(ally), 1);
    assert_eq!(
        h.health(beacon),
        110.0,
        "53651s1:10% of100 effective healing, not overheal"
    );
    h.advance(1);
    assert_eq!(amounts(&h, 53652, beacon, CombatLogKind::Heal), vec![10]);
    apply(&mut h, caster, ally, 139);
    h.advance(61);
    assert_eq!(
        amounts(&h, 53652, beacon, CombatLogKind::Heal),
        vec![10],
        "periodic healing must not transfer"
    );
}

#[test]
fn dummy15_sigil_flame_waits_native_delay_hits_original_destination_and_bleeds_once() {
    let mut h = harness(Vec::new(), &[204596, 204598], 12).unwrap();
    let mut spell = world_db().unwrap().store.get(204596).unwrap().clone();
    spell.cast_destination = Some([3.0, 0.0, 0.0]);
    let caster = h.caster;
    let target = h.npc;
    super::execute_cast(h.app.world_mut(), caster, &spell, None).unwrap();
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(caster)
        .unwrap()
        .x = 40.0;
    h.advance(35);
    assert_eq!(h.health(target), 5000.0);
    h.advance(8);
    assert!(h.health(target) < 5000.0);
    assert!(crate::auras::has_aura(h.app.world(), target, 204598));
    h.advance(165);
    let ticks: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| {
            e.spell_id == Some(204598)
                && e.target == Some(target.to_bits())
                && e.kind == CombatLogKind::Damage
                && e.periodic
        })
        .collect();
    assert_eq!(
        ticks.len(),
        4,
        "only native targeted bleed, not duplicate latent effect2"
    );
    assert!(!crate::auras::has_aura(h.app.world(), target, 204598));
}
