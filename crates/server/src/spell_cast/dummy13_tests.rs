//! Local Mainline rows, actual casts/ticks/lifetimes; no synthetic guardian scaling.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

fn apply(h: &mut Harness, caster: Entity, target: Entity, spell_id: u32, effect_mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn execute(h: &mut Harness, spell_id: u32, target: Option<Entity>) {
    let spell = world_db().unwrap().store.get(spell_id).unwrap();
    super::execute_cast(h.app.world_mut(), h.caster, spell, target).unwrap();
}

fn damage(h: &Harness, spell_id: u32, target: Entity) -> Vec<i32> {
    h.inbox
        .log
        .iter()
        .filter(|e| {
            e.spell_id == Some(spell_id)
                && e.target == Some(target.to_bits())
                && e.kind == CombatLogKind::Damage
        })
        .map(|e| e.amount)
        .collect()
}

#[test]
fn dummy13_mortal_strike_wound_reduces_real_healing_and_expires() {
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[12294, 115804, 2061], 1).unwrap();
    let target = h.npc;
    h.set_power(PowerType::Rage, 1000.0);
    execute(&mut h, 12294, Some(target));
    assert!(crate::auras::has_aura(h.app.world(), target, 115804));
    h.app.world_mut().get_mut::<Health>(target).unwrap().current = 2000.0;
    let spell = world_db().unwrap().store.get(2061).unwrap();
    let ctx = super::effects::EffectCtx {
        caster: h.caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::apply_heal(h.app.world_mut(), &ctx, target, 1000.0, false, false);
    assert_eq!(h.health(target), 2500.0);
    h.advance(230);
    assert!(!crate::auras::has_aura(h.app.world(), target, 115804));
    let before = h.health(target);
    super::effects::apply_heal(h.app.world_mut(), &ctx, target, 1000.0, false, false);
    assert_eq!(h.health(target) - before, 1000.0);
}

#[test]
fn dummy13_rend_applies_owned_native_bleed_and_ticks() {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[772, 388539], 1).unwrap();
    let target = h.npc;
    h.set_power(PowerType::Rage, 1000.0);
    execute(&mut h, 772, Some(target));
    assert!(crate::auras::has_aura(h.app.world(), target, 388539));
    h.advance(65);
    let ticks = damage(&h, 388539, target);
    assert_eq!(ticks.len(), 1);
    assert!(varied(38.0931).contains(&(ticks[0] as f32)), "{ticks:?}");
    h.advance(500);
    assert!(!crate::auras::has_aura(h.app.world(), target, 388539));
}

#[test]
fn dummy13_execute_refunds_actual_optional_rage_only_with_rank_and_survivor() {
    for (rank, health, expected) in [
        (false, 900.0, 200.0),
        (true, 900.0, 240.0),
        (true, 1.0, 200.0),
    ] {
        let mut h = harness_with_native_enemy_dodge_reduction(
            Vec::new(),
            &[163201, 260798, 316405, 137049],
            1,
        )
        .unwrap();
        let caster = h.caster;
        let target = h.npc;
        // Active SpellPower9491 requires the real Arms specialization aura.
        apply(&mut h, caster, caster, 137049, u32::MAX);
        if rank {
            apply(&mut h, caster, caster, 316405, u32::MAX);
        }
        h.set_power(PowerType::Rage, 600.0);
        h.app.world_mut().get_mut::<Health>(target).unwrap().current = health;
        execute(&mut h, 163201, Some(target));
        assert_eq!(
            h.power(PowerType::Rage),
            expected,
            "rank={rank},health={health}"
        );
        assert!(h.health(target) < health);
    }
}

#[test]
fn dummy13_execute_partial_optional_spend_refusal_and_trigger_do_not_invent_refunds() {
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[163201, 260798, 316405, 137049], 1)
            .unwrap();
    let caster = h.caster;
    let target = h.npc;
    // Active SpellPower9491 requires the real Arms specialization aura.
    apply(&mut h, caster, caster, 137049, u32::MAX);
    apply(&mut h, caster, caster, 316405, u32::MAX);
    h.app.world_mut().get_mut::<Health>(target).unwrap().current = 900.0;
    h.set_power(PowerType::Rage, 300.0);
    execute(&mut h, 163201, Some(target));
    assert_eq!(
        h.power(PowerType::Rage),
        30.0,
        "10% of actual300, not maximum400"
    );
    h.set_power(PowerType::Rage, 100.0);
    let spell = world_db().unwrap().store.get(163201).unwrap();
    let before = h.health(target);
    assert!(super::execute_cast(h.app.world_mut(), caster, spell, Some(target)).is_err());
    assert_eq!(h.power(PowerType::Rage), 100.0);
    assert_eq!(h.health(target), before);
    super::cast_triggered(h.app.world_mut(), caster, 163201, Some(target), 1);
    assert_eq!(
        h.power(PowerType::Rage),
        100.0,
        "triggered cast paid no Rage"
    );
}

#[test]
fn dummy13_shadow_word_death_native_execute_threshold_and_deathspeaker() {
    for (rank, health, multiplier) in [
        (false, 1000.0, 1.0),
        (false, 999.0, 2.5),
        (false, 2000.0, 1.0),
        (true, 2000.0, 3.5),
        (true, 2500.0, 1.0),
    ] {
        let mut h = harness(Vec::new(), &[32379, 364675], 5).unwrap();
        let caster = h.caster;
        let target = h.npc;
        if rank {
            apply(&mut h, caster, caster, 364675, u32::MAX);
        }
        h.app.world_mut().get_mut::<Health>(target).unwrap().current = health;
        execute(&mut h, 32379, Some(target));
        assert!(
            varied(85.0 * multiplier).contains(&(health - h.health(target))),
            "rank={rank},health={health},loss={}",
            health - h.health(target)
        );
        assert_eq!(h.health(caster), 950.0, "one native backlash, not twice");
    }
}

#[test]
fn dummy13_rapid_fire_native_initial_and_six_clocked_shots_generate_focus() {
    let mut h = harness(Vec::new(), &[257044, 257045, 263585], 3).unwrap();
    let target = h.npc;
    h.set_power(PowerType::Focus, 0.0);
    h.cast_and_deliver(257044, Some(target));
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let shots = damage(&h, 257045, target);
    assert_eq!(shots.len(), 7, "{shots:?}");
    assert!(
        shots
            .iter()
            .all(|&amount| varied(105.0).contains(&(amount as f32)))
    );
    // Baseline Focus regenerates independently: assert authored energize logs.
    let gained: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(263585) && e.kind == CombatLogKind::Energize)
        .collect();
    assert_eq!(gained.len(), 7);
    assert!(gained.iter().all(|e| e.amount == 3));
    assert!(!crate::auras::has_aura(h.app.world(), target, 257044));
}

#[test]
fn dummy13_rapid_fire_cancellation_stops_future_shots_and_focus() {
    let mut h = harness(Vec::new(), &[257044, 257045, 263585], 3).unwrap();
    let target = h.npc;
    h.set_power(PowerType::Focus, 0.0);
    h.cast_and_deliver(257044, Some(target));
    h.advance(14);
    super::cancel_cast(h.app.world_mut(), h.caster);
    h.advance(4);
    let count = damage(&h, 257045, target).len();
    assert!((1..7).contains(&count));
    h.advance(60);
    assert_eq!(damage(&h, 257045, target).len(), count);
}

// Non-Npc attacker pins the hit table; fixture CharacterStats pins crit at zero.
// This proves persistent-pet script routing, not live pet spawn/stat inheritance.
fn pet(h: &mut Harness) -> Entity {
    let owner = h.caster;
    let pet = spawn_player(&mut h.app, 3, 2.0);
    h.app
        .world_mut()
        .entity_mut(pet)
        .remove::<shared::components::Player>()
        .insert((
            crate::hunter_pet::HunterPet { owner, slot: 0 },
            crate::networking::CombatTarget(Some(h.npc)),
        ));
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::hunter_pet::PetStable {
            character_id: Some(1),
            stored: default(),
            summoned: Some(pet),
        });
    pet
}

#[test]
fn dummy13_bestial_wrath_strike_uses_own_pets_current_target() {
    let mut h = harness(Vec::new(), &[19574, 344572, SUNWELL_RADIANCE], 3).unwrap();
    let own = pet(&mut h);
    apply_native_enemy_dodge_reduction_to(&mut h, own);
    let target = h.npc;
    let other = spawn_npc(&mut h.app, 4.0);
    h.select(Some(other));
    h.advance(4);
    let before = h.health(target); // Existing pet melee may already have swung.
    execute(&mut h, 19574, None);
    assert!(
        !crate::auras::has_aura(h.app.world(), own, 19574),
        "owner modifier is not an authored pet buff binding"
    );
    assert_eq!(
        before - h.health(target),
        176.0,
        "native8AP * inherited22AP; isolate instant strike from prior pet melee"
    );
    assert_eq!(
        h.health(other),
        5000.0,
        "owner selection is not pet selection"
    );
}

#[test]
fn dummy13_bestial_wrath_does_not_infer_target_or_buff_foreign_pet() {
    let mut h = harness(Vec::new(), &[19574, 344572], 3).unwrap();
    let own = pet(&mut h);
    let foreign_owner = spawn_player(&mut h.app, 3, 0.0);
    let foreign = spawn_player(&mut h.app, 3, 2.0);
    h.app
        .world_mut()
        .entity_mut(foreign)
        .insert(crate::hunter_pet::HunterPet {
            owner: foreign_owner,
            slot: 0,
        });
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(own)
        .unwrap()
        .0 = None;
    let target = h.npc;
    execute(&mut h, 19574, None);
    assert!(!crate::auras::has_aura(h.app.world(), foreign, 19574));
    assert_eq!(h.health(foreign), 1000.0);
    assert_eq!(h.health(target), 5000.0, "no inferred pet target");
}

#[test]
fn dummy13_aspect_wild_fires_free_primary_cobra_without_later_focus_spend() {
    let mut h = harness(Vec::new(), &[193530, 193455, 34026], 3).unwrap();
    let target = h.npc;
    h.select(Some(target));
    h.advance(4);
    h.set_power(PowerType::Focus, 100.0);
    execute(&mut h, 193530, Some(target));
    let focus = h.power(PowerType::Focus);
    h.advance(8);
    let shots = damage(&h, 193455, target);
    assert_eq!(shots.len(), 1, "{shots:?}");
    assert!(varied(390.4 * ((112.0 + 9.0 * 12.0) / 340.0)).contains(&(shots[0] as f32)));
    assert_eq!(focus, 100.0, "triggered Cobra must not spend Focus");
}

#[test]
fn dummy13_haunt_foreign_owner_and_real_expiry_preserve_other_running_timer() {
    let mut h = harness(Vec::new(), &[48181, 108853], 9).unwrap();
    let target = h.npc;
    let other = spawn_npc(&mut h.app, 4.0);
    let foreign = spawn_player(&mut h.app, 9, 0.0);
    let spell = world_db().unwrap().store.get(48181).unwrap();
    execute(&mut h, 48181, Some(target));
    super::execute_cast(h.app.world_mut(), foreign, spell, Some(other)).unwrap();
    h.advance(10);
    assert!(crate::auras::has_aura(h.app.world(), other, 48181));
    let foreign_ready = crate::auras::aura_now(h.app.world()) + 1000.0;
    h.app
        .world_mut()
        .get_mut::<super::SpellHistory>(foreign)
        .unwrap()
        .spell_ready_at
        .insert(48181, foreign_ready);
    h.app.world_mut().get_mut::<Health>(target).unwrap().current = 1.0;
    super::cast_triggered(h.app.world_mut(), h.caster, 108853, Some(target), 0);
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<super::SpellHistory>(foreign)
            .unwrap()
            .spell_ready_at[&48181],
        foreign_ready
    );
    h.advance(500);
    assert!(!crate::auras::has_aura(h.app.world(), other, 48181));
    assert_eq!(
        h.app
            .world()
            .get::<super::SpellHistory>(foreign)
            .unwrap()
            .spell_ready_at[&48181],
        foreign_ready,
        "real expiry is not death"
    );
}

#[test]
fn dummy13_haunt_only_owned_target_death_resets_running_cooldown() {
    for death in [false, true] {
        let mut h = harness(Vec::new(), &[48181, 108853], 9).unwrap();
        let target = h.npc;
        execute(&mut h, 48181, Some(target));
        h.advance(10);
        assert!(crate::auras::has_aura(h.app.world(), target, 48181));
        if death {
            h.app.world_mut().get_mut::<Health>(target).unwrap().current = 1.0;
            super::cast_triggered(h.app.world_mut(), h.caster, 108853, Some(target), 0);
        } else {
            crate::auras::remove_auras(
                h.app.world_mut(),
                target,
                crate::auras::RemoveReason::Cancelled,
                |a| a.spell_id == 48181,
            );
        }
        h.advance(3);
        let now = crate::auras::aura_now(h.app.world());
        let ready = h
            .app
            .world()
            .get::<super::SpellHistory>(h.caster)
            .unwrap()
            .spell_ready_at
            .get(&48181)
            .copied()
            .unwrap_or(now);
        assert_eq!(ready <= now, death, "cancel cannot refund Haunt");
    }
}
