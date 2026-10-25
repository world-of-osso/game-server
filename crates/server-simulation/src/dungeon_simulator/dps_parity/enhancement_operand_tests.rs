//! Native state used by the literal Enhancement profile; aliases are data bindings.
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use crate::spell_cast::test_harness::{harness, spawn_npc};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

#[test]
fn native_apl_racial_action_metadata_exposes_cooldown_and_duration() {
    let h = harness(Vec::new(), &[26297], 8).unwrap();
    let bindings = Bindings {
        actions: [("berserking".into(), 26297)].into(),
        ..Default::default()
    };
    let facts = NativeFacts::new(h.app.world(), h.caster, &bindings, encounter(), 1);
    assert_eq!(facts.value("action.berserking.cooldown").unwrap(), 180.0);
    assert_eq!(facts.value("action.berserking.duration").unwrap(), 12.0);
}

fn encounter() -> Encounter {
    Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

#[test]
fn native_apl_bare_ticking_uses_proposed_action_and_owned_target() {
    const REND: u32 = 388539;
    let mut h = harness(Vec::new(), &[REND], 1).unwrap();
    let other = spawn_npc(&mut h.app, 3.0);
    let targets = [h.npc, other];
    let bindings = Bindings {
        actions: [("rend".into(), REND)].into(),
        buffs: [("rend".into(), REND)].into(),
        ..Default::default()
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id: REND,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let facts = NativeFacts::new(h.app.world(), h.caster, &bindings, encounter(), 2)
        .with_targets(Some(h.npc), &targets);
    let action = Action {
        source_row: None,
        name: "rend".into(),
        target: None,
        options: Default::default(),
    };
    assert_eq!(facts.value_for(&action, "ticking").unwrap(), 1.0);
    let secondary = Action {
        source_row: None,
        target: Some(other.to_bits()),
        ..action
    };
    assert_eq!(facts.value_for(&secondary, "ticking").unwrap(), 0.0);
}

#[test]
fn native_apl_pet_active_filters_actual_owner_life_and_expiry() {
    use crate::spell_cast::summon::{SummonProperties, SummonedCreature};
    const SUMMON: u32 = 51533;
    let mut h = harness(Vec::new(), &[SUMMON], 1).unwrap();
    let bindings = Bindings {
        actions: [("wolves".into(), SUMMON)].into(),
        ..Default::default()
    };
    let pet = spawn_npc(&mut h.app, 1.0);
    let summon = SummonedCreature {
        owner: h.caster,
        spell_id: SUMMON,
        properties_id: 0,
        properties: SummonProperties {
            control: 0,
            faction: 0,
            title: 2,
            slot: 0,
            flags: 0,
        },
        display_scale: 1.0,
        expires_at: Some(10.0),
    };
    h.app.world_mut().entity_mut(pet).insert(summon.clone());
    let read = |world: &World| {
        NativeFacts::new(world, h.caster, &bindings, encounter(), 1)
            .value("pet.wolves.active")
            .unwrap()
    };
    assert_eq!(read(h.app.world()), 1.0);
    h.app
        .world_mut()
        .get_mut::<SummonedCreature>(pet)
        .unwrap()
        .owner = h.npc;
    assert_eq!(read(h.app.world()), 0.0);
    h.app.world_mut().entity_mut(pet).insert(SummonedCreature {
        expires_at: Some(0.0),
        ..summon.clone()
    });
    assert_eq!(read(h.app.world()), 0.0);
    h.app.world_mut().entity_mut(pet).insert(summon);
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(pet)
        .unwrap()
        .current = 0.0;
    assert_eq!(read(h.app.world()), 0.0);
}

#[test]
fn native_apl_damage_prediction_uses_native_power_and_critical_multiplier_without_casting() {
    use crate::stats::CharacterStats;
    const LIGHTNING_BOLT: u32 = 188196;
    let mut h = harness(Vec::new(), &[LIGHTNING_BOLT], 7).unwrap();
    let bindings = Bindings {
        actions: [("lightning_bolt".into(), LIGHTNING_BOLT)].into(),
        ..Default::default()
    };
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CharacterStats {
            spell_power: 1000.0,
            crit_pct: 0.0,
            ..Default::default()
        });
    let targets = [h.npc];
    let read = |world: &World| {
        NativeFacts::new(world, h.caster, &bindings, encounter(), 1)
            .with_targets(Some(h.npc), &targets)
            .value("action.lightning_bolt.damage")
            .unwrap()
    };
    // Native70448 LB coefficient is1.7, no flat/AP amount. SimC action.cpp3502
    // requests an average direct amount, not a real cast or an RNG sample.
    let hit = read(h.app.world());
    assert!((hit - 1700.0).abs() < 0.01, "native expected hit={hit}");
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    assert!((read(h.app.world()) - hit * 2.0).abs() < 0.01);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
    );
}
