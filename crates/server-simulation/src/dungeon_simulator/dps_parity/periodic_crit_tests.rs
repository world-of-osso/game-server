//! Native aura-to-tick receipts: no invented rows, amounts, stats or crit flags.
//! Controlled aura applications isolate the periodic boundary, not cast admission/APL.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::followers::simulator::ParityActor;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const BARBED_SHOT: u32 = 217200;
const RENEW: u32 = 139;
const RUPTURE: u32 = 1943;
const IMPROVED_RUPTURE: u32 = 21880;
const PLAGUE_BOMB: u32 = 341408;
const LANGUISH: u32 = 71023;
const BURNING_BLOOD: u32 = 98957;
const BIO_SPORES: u32 = 154611;
const CRITICAL_FOCUS: u32 = 75060;
const DENOUNCE: u32 = 171884;
const LUCKYDO: u32 = 130165;
const PERIODIC_CAN_CRIT: u32 = 0x200;
const CANT_CRIT: u32 = 0x2000_0000;

fn native_actor(spec: u32, identity: ParityActor) -> Run {
    let (profile, traits) = read_reference(spec).unwrap();
    prepare_reference(profile, &traits, 1, identity).unwrap()
}

fn apply_native_aura(run: &mut Run, caster: Entity, target: Entity, spell_id: u32) {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap();
    let effect_mask = spell
        .effects
        .iter()
        .filter(|effect| effect.effect == 6)
        .fold(0, |mask, effect| mask | (1 << effect.index));
    assert_ne!(
        effect_mask, 0,
        "native aura effects required for {spell_id}"
    );
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
    run.app.update();
    assert!(crate::auras::has_aura(run.app.world(), target, spell_id));
}

fn periodic_events(run: &Run, caster: Entity, target: Entity, spell: u32) -> Vec<CombatLogEvent> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(spell)
                && event.periodic
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Heal)
        })
        .cloned()
        .collect()
}

fn collect_native_ticks(
    run: &mut Run,
    caster: Entity,
    target: Entity,
    spell: u32,
    count: usize,
) -> Vec<CombatLogEvent> {
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..TICKS {
        if !crate::auras::has_aura(run.app.world(), target, spell) {
            apply_native_aura(run, caster, target, spell);
        }
        run.app.update();
        let events = periodic_events(run, caster, target, spell);
        if events.len() >= count {
            assert!(
                events.iter().all(|event| event.amount > 0),
                "positive native ticks: {events:?}"
            );
            return events;
        }
    }
    panic!("native {spell} did not deliver {count} periodic events");
}

fn assert_native_flags(run: &Run, spell: u32, allow: bool, deny: bool) {
    let store = run.app.world().resource::<crate::spell_info::SpellStore>();
    let misc = store.get(spell).unwrap().misc.as_ref().unwrap();
    assert_eq!(misc.attributes[8] & PERIODIC_CAN_CRIT != 0, allow);
    assert_eq!(misc.attributes[2] & CANT_CRIT != 0, deny);
}

fn assert_natural_barbed_crits(identity: ParityActor) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_actor(253, identity);
    let (caster, target) = (run.actor, run.targets[0]);
    assert_native_flags(&run, BARBED_SHOT, true, false);
    let chance = run
        .app
        .world()
        .get::<CharacterStats>(caster)
        .unwrap()
        .crit_pct;
    assert!(chance > 0.0 && chance < 100.0);
    let events = collect_native_ticks(&mut run, caster, target, BARBED_SHOT, 32);
    let crits = events.iter().filter(|event| event.crit).count();
    println!(
        "NATIVE_PERIODIC_CRIT {}",
        json!({"spell": BARBED_SHOT, "player": run.app.world().get::<shared::components::Player>(caster).is_some(), "crit_pct": chance, "ticks": events.len(), "crits": crits})
    );
    assert!(
        crits > 0,
        "eligible native Barbed Shot never crit across {} seeded ticks",
        events.len()
    );
    assert!(
        crits < events.len(),
        "ordinary native chance must also produce non-crits"
    );
}

#[test]
#[ignore = "native periodic crit RED/GREEN; requires prepared parity inputs"]
fn dps_parity_periodic_crit_native_player_barbed_shot_rolls_each_tick() {
    // Native ChrRaces3 = Dwarf, matching the pinned MID2 Hunter profile.
    assert_natural_barbed_crits(ParityActor::Player { race: 3 });
}

#[test]
#[ignore = "native periodic crit RED/GREEN; requires prepared parity inputs"]
fn dps_parity_periodic_crit_native_class_follower_barbed_shot_rolls_each_tick() {
    assert_natural_barbed_crits(ParityActor::Follower);
}

#[test]
#[ignore = "native periodic crit controls; requires prepared parity inputs"]
fn dps_parity_periodic_crit_native_cant_crit_overrides_allow_and_unflagged_stays_normal() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_actor(253, ParityActor::Follower);
    let (caster, target) = (run.actor, run.targets[0]);
    assert_native_flags(&run, PLAGUE_BOMB, true, true);
    assert_native_flags(&run, LANGUISH, false, false);
    apply_native_aura(&mut run, caster, caster, CRITICAL_FOCUS);
    assert!(
        run.app
            .world()
            .get::<CharacterStats>(caster)
            .unwrap()
            .crit_pct
            >= 100.0
    );
    for spell in [PLAGUE_BOMB, LANGUISH] {
        let events = collect_native_ticks(&mut run, caster, target, spell, 4);
        assert!(
            run.app
                .world()
                .get::<CharacterStats>(caster)
                .unwrap()
                .crit_pct
                >= 100.0
        );
        assert!(
            events.iter().all(|event| !event.crit),
            "native gate violated: {events:?}"
        );
        println!(
            "NATIVE_PERIODIC_CRIT_CONTROL {}",
            json!({"spell":spell,"ticks":events.len(),"crits":0})
        );
    }
}

#[test]
#[ignore = "native ordinary-creature spell-owner control; requires prepared parity inputs"]
fn dps_parity_periodic_crit_native_plain_creature_has_no_class_spell_owner() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_actor(253, ParityActor::Follower);
    let (caster, target) = (run.targets[0], run.actor);
    assert!(run.app.world().get::<CharacterStats>(caster).is_none());
    assert_native_flags(&run, BURNING_BLOOD, true, false);
    // Native Bio Spores has +100 spell crit (57), independently of class stats.
    apply_native_aura(&mut run, caster, caster, BIO_SPORES);
    assert!(
        run.app
            .world()
            .get::<crate::auras::AuraStatMods>(caster)
            .unwrap()
            .spell_crit_pct
            >= 100.0
    );
    let events = collect_native_ticks(&mut run, caster, target, BURNING_BLOOD, 4);
    assert!(
        events.iter().all(|event| !event.crit),
        "ordinary NPC has no Player-equivalent spell owner: {events:?}"
    );
}

#[test]
#[ignore = "native pet spell-owner RED/GREEN; requires prepared parity inputs"]
fn dps_parity_periodic_crit_native_pet_uses_class_owners_current_chance() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let (owner, target) = (run.actor, run.targets[0]);
    assert!(run.app.world().get::<CharacterStats>(pet).is_none());
    assert_native_flags(&run, BURNING_BLOOD, true, false);
    apply_native_aura(&mut run, owner, owner, CRITICAL_FOCUS);
    assert!(
        run.app
            .world()
            .get::<CharacterStats>(owner)
            .unwrap()
            .crit_pct
            >= 100.0
    );
    let events = collect_native_ticks(&mut run, pet, target, BURNING_BLOOD, 4);
    assert!(
        events.iter().all(|event| event.crit),
        "native pet must use its class spell owner's current chance: {events:?}"
    );
}

fn first_new_tick(run: &mut Run, caster: Entity, target: Entity, spell: u32) -> CombatLogEvent {
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..TICKS {
        run.app.update();
        if let Some(event) = periodic_events(run, caster, target, spell)
            .into_iter()
            .next()
        {
            assert!(event.amount > 0);
            return event;
        }
    }
    panic!("native {spell} did not deliver its next tick");
}

fn aura_instance(run: &Run, target: Entity, spell: u32) -> u32 {
    run.app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .unwrap()
        .instance_id
}

#[test]
#[ignore = "native HoT live-crit RED/GREEN; requires prepared parity inputs"]
fn dps_parity_periodic_crit_native_renew_reads_current_crit_after_application() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_actor(63, ParityActor::Follower);
    let caster = run.actor;
    assert_native_flags(&run, RENEW, true, false);
    // Denounce contributes its real -100 crit aura; no direct-damage effect is applied.
    apply_native_aura(&mut run, caster, caster, DENOUNCE);
    assert!(
        run.app
            .world()
            .get::<CharacterStats>(caster)
            .unwrap()
            .crit_pct
            <= 0.0
    );
    apply_native_aura(&mut run, caster, caster, RENEW);
    let instance = aura_instance(&run, caster, RENEW);
    let normal = first_new_tick(&mut run, caster, caster, RENEW);
    assert!(!normal.crit);
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Dispelled,
        |aura| aura.spell_id == DENOUNCE,
    );
    apply_native_aura(&mut run, caster, caster, CRITICAL_FOCUS);
    assert!(
        run.app
            .world()
            .get::<CharacterStats>(caster)
            .unwrap()
            .crit_pct
            >= 100.0
    );
    let critical = first_new_tick(&mut run, caster, caster, RENEW);
    assert_eq!(
        aura_instance(&run, caster, RENEW),
        instance,
        "crit changes without reapplying HoT"
    );
    println!(
        "NATIVE_PERIODIC_HEAL_CRIT {}",
        json!({"normal":normal.amount,"later":critical.amount,"crit":critical.crit})
    );
    assert!(
        critical.crit,
        "native HoT must use current crit, not application-time crit"
    );
    let multiplier =
        crate::auras::combat_stats::critical_healing_multiplier(run.app.world(), caster);
    let expected = normal.amount as f32 * 2.0 * multiplier;
    assert!(
        (critical.amount as f32 - expected).abs() <= 2.0,
        "native heal critical bonus: expected {expected}, got {critical:?}"
    );
}

#[test]
#[ignore = "native flat periodic bonus ordering RED/GREEN; requires prepared parity inputs"]
fn dps_parity_periodic_crit_native_rupture_flat_bonus_precedes_critical_multiplier() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_actor(259, ParityActor::Player { race: 3 });
    let (caster, target) = (run.actor, run.targets[0]);
    assert_native_flags(&run, RUPTURE, true, false);
    apply_native_aura(&mut run, caster, caster, DENOUNCE);
    apply_native_aura(&mut run, caster, target, RUPTURE);
    let without_flat = first_new_tick(&mut run, caster, target, RUPTURE);
    assert!(!without_flat.crit);

    apply_native_aura(&mut run, caster, caster, IMPROVED_RUPTURE);
    let flat = run
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == IMPROVED_RUPTURE)
        .unwrap()
        .effects[0]
        .amount;
    assert_eq!(flat, 10.0, "unmodified native21880 flat periodic bonus");
    apply_native_aura(&mut run, caster, caster, DENOUNCE);
    apply_native_aura(&mut run, caster, target, RUPTURE);
    let normal = first_new_tick(&mut run, caster, target, RUPTURE);
    assert!(!normal.crit);
    assert!(
        normal.amount > without_flat.amount,
        "native flat bonus must alter actual normal tick: {without_flat:?} -> {normal:?}"
    );

    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Dispelled,
        |aura| aura.spell_id == DENOUNCE,
    );
    apply_native_aura(&mut run, caster, caster, LUCKYDO);
    assert!(
        run.app
            .world()
            .get::<CharacterStats>(caster)
            .unwrap()
            .crit_pct
            >= 100.0
    );
    let critical = first_new_tick(&mut run, caster, target, RUPTURE);
    assert!(critical.crit);
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(RUPTURE)
        .unwrap();
    let multiplier = world
        .get::<crate::auras::AuraStatMods>(caster)
        .unwrap()
        .critical_damage(spell.school_mask());
    // TC Unit.cpp6913 applies flat PeriodicHealingAndDamage before
    // SpellAuraEffects.cpp5680 rolls/bonuses the entire resulting tick.
    let bonus = crate::auras::spell_mods::modify(
        world,
        caster,
        spell,
        crate::auras::spell_mods::SpellModOp::CritDamageAndHealing,
        normal.amount as f32 * (2.0 * multiplier - 1.0),
    );
    let expected = normal.amount as f32 + bonus;
    let critical_factor = expected / normal.amount as f32;
    // Each receipt rounds once: propagate the normal receipt's half-unit error.
    let rounding_bound = ((critical_factor + 1.0) / 2.0).ceil();
    println!(
        "NATIVE_PERIODIC_FLAT_CRIT {}",
        json!({"spell":RUPTURE,"modifier":IMPROVED_RUPTURE,"flat":flat,"without_flat":without_flat.amount,"normal_with_flat":normal.amount,"critical":critical.amount,"expected":expected,"rounding_bound":rounding_bound})
    );
    assert!(
        (critical.amount as f32 - expected).abs() <= rounding_bound,
        "TC crit includes (base + native flat) before multiplication: expected {expected}, got {critical:?}"
    );
}

#[test]
#[ignore = "native DoT live-crit/bonus RED/GREEN; requires prepared parity inputs"]
fn dps_parity_periodic_crit_native_barbed_reads_current_crit_and_spell_critical_bonus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_actor(253, ParityActor::Follower);
    let (caster, target) = (run.actor, run.targets[0]);
    apply_native_aura(&mut run, caster, caster, DENOUNCE);
    assert!(
        run.app
            .world()
            .get::<CharacterStats>(caster)
            .unwrap()
            .crit_pct
            <= 0.0
    );
    apply_native_aura(&mut run, caster, target, BARBED_SHOT);
    let instance = aura_instance(&run, target, BARBED_SHOT);
    let normal = first_new_tick(&mut run, caster, target, BARBED_SHOT);
    assert!(!normal.crit);
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Dispelled,
        |aura| aura.spell_id == DENOUNCE,
    );
    // Native Luckydo: +100 crit (290), +200% school critical damage (163).
    apply_native_aura(&mut run, caster, caster, LUCKYDO);
    assert!(
        run.app
            .world()
            .get::<CharacterStats>(caster)
            .unwrap()
            .crit_pct
            >= 100.0
    );
    let critical = first_new_tick(&mut run, caster, target, BARBED_SHOT);
    assert_eq!(
        aura_instance(&run, target, BARBED_SHOT),
        instance,
        "crit changes without refreshing DoT"
    );
    assert!(
        critical.crit,
        "native DoT must read the caster's current crit"
    );
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(BARBED_SHOT)
        .unwrap();
    let multiplier = world
        .get::<crate::auras::AuraStatMods>(caster)
        .unwrap()
        .critical_damage(spell.school_mask());
    let bonus = crate::auras::spell_mods::modify(
        world,
        caster,
        spell,
        crate::auras::spell_mods::SpellModOp::CritDamageAndHealing,
        normal.amount as f32 * (2.0 * multiplier - 1.0),
    );
    let expected = normal.amount as f32 + bonus;
    println!(
        "NATIVE_PERIODIC_DAMAGE_CRIT {}",
        json!({"normal":normal.amount,"later":critical.amount,"multiplier":multiplier,"expected":expected})
    );
    assert!(
        (critical.amount as f32 - expected).abs() <= 4.0,
        "native spell critical damage bonus: expected {expected}, got {critical:?}"
    );
}
