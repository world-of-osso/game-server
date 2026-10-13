//! Native Mainline rows; bounded low-count rank and charge consumers.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Mana, PowerType};
use shared::protocol::CombatLogKind;

fn fixture(ids: &[u32], roots: &[u32], class: u8) -> Harness {
    let mut h = harness(Vec::new(), ids, class).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(
            roots.to_vec(),
        ));
    h
}
fn apply(h: &mut Harness, id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}
fn execute(h: &mut Harness, id: u32, target: Option<Entity>) {
    let spell = world_db().unwrap().store.get(id).unwrap();
    super::execute_cast(h.app.world_mut(), h.caster, spell, target).unwrap();
}
fn reduce_judgment(root: u32, target_enemy: bool) {
    for ranked in [false, true] {
        let mut h = fixture(&[root, 196926, 275773, 25912, 25914], &[root], 2);
        if ranked {
            apply(&mut h, 196926);
        }
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(super::SpellHistory::default());
        let until = super::now(h.app.world()) + 20.0;
        h.app
            .world_mut()
            .get_mut::<super::SpellHistory>(h.caster)
            .unwrap()
            .spell_ready_at
            .insert(275773, until);
        let target = if target_enemy { h.npc } else { h.caster };
        execute(&mut h, root, Some(target));
        h.advance(1);
        let actual = h
            .app
            .world()
            .get::<super::SpellHistory>(h.caster)
            .unwrap()
            .spell_ready_at[&275773];
        assert_eq!(actual, until - if ranked { 1.5 } else { 0.0 });
        assert_eq!(
            h.inbox
                .cooldowns
                .iter()
                .filter(|u| u.spell_id == 275773)
                .count(),
            usize::from(ranked)
        );
        h.app
            .world_mut()
            .get_mut::<super::SpellHistory>(h.caster)
            .unwrap()
            .spell_ready_at
            .remove(&275773);
        execute(&mut h, root, Some(target));
        assert!(
            !h.app
                .world()
                .get::<super::SpellHistory>(h.caster)
                .unwrap()
                .spell_ready_at
                .contains_key(&275773)
        );
    }
}
#[test]
fn dummy16_holy_shock_crusaders_might_reduces_only_running_judgment() {
    reduce_judgment(20473, false);
}
#[test]
fn dummy16_crusader_strike_crusaders_might_reduces_only_running_judgment() {
    reduce_judgment(35395, true);
}
#[test]
fn dummy16_holy_light_generates_named_holy_power_only_with_rank() {
    for ranked in [false, true] {
        let mut h = fixture(&[82326, 231642, 88852], &[82326], 2);
        if ranked {
            apply(&mut h, 231642);
        }
        h.set_power(PowerType::HolyPower, 0.0);
        let caster = h.caster;
        execute(&mut h, 82326, Some(caster));
        assert_eq!(
            h.power(PowerType::HolyPower),
            if ranked { 1.0 } else { 0.0 }
        );
    }
}
fn judgment_power(root: u32) {
    for ranked in [false, true] {
        let mut h = fixture(&[root, 315867, 220637], &[root], 2);
        if ranked {
            apply(&mut h, 315867);
        }
        h.set_power(PowerType::HolyPower, 0.0);
        let target = h.npc;
        execute(&mut h, root, Some(target));
        h.advance(10);
        assert_eq!(
            h.power(PowerType::HolyPower),
            if ranked { 1.0 } else { 0.0 }
        );
        assert_eq!(
            h.inbox
                .log
                .iter()
                .filter(|e| e.spell_id == Some(220637) && e.kind == CombatLogKind::Energize)
                .count(),
            usize::from(ranked)
        );
    }
}
#[test]
fn dummy16_retribution_judgment_generates_named_rank_power_once() {
    judgment_power(20271);
}
#[test]
fn dummy16_holy_judgment_generates_named_rank_power_once() {
    judgment_power(275773);
}
#[test]
fn dummy16_arcane_power_native_school_discount_applies_once_with_or_without_rank() {
    for (active, ranked) in [(false, false), (true, false), (true, true)] {
        let mut h = fixture(&[12042, 343208, 133], &[12042, 133], 8);
        if ranked {
            apply(&mut h, 343208);
        }
        if active {
            apply(&mut h, 12042);
        }
        let before = h.app.world().get::<Mana>(h.caster).unwrap().current;
        let target = h.npc;
        execute(&mut h, 133, Some(target));
        let spent = before - h.app.world().get::<Mana>(h.caster).unwrap().current;
        // TC CalcPowerCost applies native12042s2 aura72 once;343208 adds no gate.
        assert_eq!(
            spent,
            if active { 70.0 } else { 100.0 },
            "active={active} ranked={ranked}"
        );
    }
}
#[test]
fn dummy16_arcane_power_discount_ends_at_native_expiry() {
    let mut h = fixture(&[12042, 343208, 133], &[12042, 133], 8);
    apply(&mut h, 343208);
    apply(&mut h, 12042);
    let duration = world_db()
        .unwrap()
        .store
        .get(12042)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .unwrap()
        .duration_ms;
    h.advance((duration / 50 + 2) as usize);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 12042));
    let before = h.app.world().get::<Mana>(h.caster).unwrap().current;
    let target = h.npc;
    execute(&mut h, 133, Some(target));
    assert_eq!(
        before - h.app.world().get::<Mana>(h.caster).unwrap().current,
        100.0
    );
}

#[test]
fn dummy16_judgment_immune_primary_never_generates_rank_power() {
    for root in [20271, 275773] {
        let mut h = fixture(&[root, 315867, 220637, 642], &[root], 2);
        apply(&mut h, 315867);
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.npc,
                target: h.npc,
                spell_id: 642,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        h.set_power(PowerType::HolyPower, 0.0);
        let target = h.npc;
        execute(&mut h, root, Some(target));
        h.advance(10);
        assert_eq!(h.power(PowerType::HolyPower), 0.0);
        assert_eq!(h.health(target), 5000.0);
    }
}

#[test]
fn dummy16_arcane_blast_snapshots_pre_gain_charge_damage_and_cost() {
    for charges in [0.0, 2.0, 4.0] {
        let mut h = fixture(&[30451, 36032], &[30451], 8);
        h.set_power(PowerType::ArcaneCharges, charges);
        let spell = world_db().unwrap().store.get(30451).unwrap();
        let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
        let before = h.app.world().get::<Mana>(h.caster).unwrap().current;
        let target = h.npc;
        execute(&mut h, 30451, Some(target));
        let spent = before - h.app.world().get::<Mana>(h.caster).unwrap().current;
        assert_eq!(spent, 275.0 * (1.0 + charges));
        assert_eq!(h.power(PowerType::ArcaneCharges), (charges + 1.0).min(4.0));
        h.set_power(PowerType::ArcaneCharges, 0.0);
        h.advance(10);
        let damage: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(30451) && e.kind == CombatLogKind::Damage)
            .map(|e| e.amount as f32)
            .collect();
        assert_eq!(damage.len(), 1);
        assert!(
            varied(base * (1.0 + 0.6 * charges)).contains(&damage[0]),
            "{charges}: {damage:?}"
        );
    }
}
#[test]
fn dummy16_arcane_blast_charge_cast_time_and_unaffordable_cast_boundary() {
    for charges in [0.0, 2.0, 4.0] {
        let mut h = fixture(&[30451, 36032], &[30451], 8);
        h.set_power(PowerType::ArcaneCharges, charges);
        h.cast_and_deliver(30451, Some(h.npc));
        assert!(h.inbox.failed.is_empty());
        let cast = h
            .app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .unwrap();
        assert!((cast.duration - 2.25 * (1.0 - 0.08 * charges)).abs() < 0.001);
    }
    let mut h = fixture(&[30451, 36032], &[30451], 8);
    h.set_power(PowerType::ArcaneCharges, 4.0);
    h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = 1000.0;
    h.cast_and_deliver(30451, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
    );
    assert_eq!(h.power(PowerType::ArcaneCharges), 4.0);
    assert_eq!(h.health(h.npc), 5000.0);
}
#[test]
fn dummy16_healing_wave_heals_only_native_primary_and_never_enemy() {
    let mut h = fixture(&[77472], &[77472], 7);
    let ally = spawn_player(&mut h.app, 7, 3.0);
    let other = spawn_player(&mut h.app, 7, 4.0);
    for unit in [ally, other] {
        h.app
            .world_mut()
            .entity_mut(unit)
            .insert(shared::components::Health {
                current: 100.0,
                max: 5000.0,
            });
    }
    h.cast_and_deliver(77472, Some(ally));
    h.advance(45);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(1501.0).contains(&(h.health(ally) - 100.0)));
    assert_eq!(h.health(other), 100.0);
    assert_eq!(h.health(h.npc), 5000.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::Health {
            current: 100.0,
            max: 5000.0,
        });
    //Existing friendly-cast selection chooses self when selection is hostile.
    h.cast_and_deliver(77472, Some(h.npc));
    h.advance(45);
    assert!(h.inbox.failed.is_empty());
    assert_eq!(h.health(h.npc), 5000.0);
    assert!(varied(1501.0).contains(&(h.health(h.caster) - 100.0)));
}

fn tidal_stacks(h: &Harness) -> u8 {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 53390)
        .map_or(0, |a| a.stacks)
}
#[test]
fn dummy16_riptide_grants_two_capped_tidal_uses_not_double_cast_speed() {
    let mut h = fixture(
        &[61295, 51564, 53390, 77472, 1064],
        &[61295, 77472, 1064],
        7,
    );
    apply(&mut h, 51564);
    let caster = h.caster;
    for _ in 0..3 {
        execute(&mut h, 61295, Some(caster));
    }
    assert_eq!(tidal_stacks(&h), 2);
    for (root, remaining) in [(77472, 1), (1064, 0)] {
        h.app
            .world_mut()
            .entity_mut(caster)
            .insert(super::SpellHistory::default());
        h.cast_and_deliver(root, Some(caster));
        assert!(
            h.inbox.failed.is_empty(),
            "root{root}: {:?}",
            h.inbox.failed
        );
        let duration = h
            .app
            .world()
            .get::<shared::casting::CastState>(caster)
            .unwrap()
            .duration;
        assert!((duration - 2.0 * 0.8).abs() < 0.001);
        h.advance(50);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(tidal_stacks(&h), remaining);
    }
    //Rank absent never creates a named buff.
    let mut plain = fixture(&[61295, 51564, 53390], &[61295], 7);
    let caster = plain.caster;
    execute(&mut plain, 61295, Some(caster));
    assert_eq!(tidal_stacks(&plain), 0);
}
