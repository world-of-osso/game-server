//! Spell travel time on real world.db spells.

use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

use crate::spell_cast::test_harness::{
    Harness, harness, harness_with_native_enemy_dodge_reduction,
};

/// Frostbolt: 1.75 s cast, missile `Speed` 35 yd/s; its damage is the triggered 228597.
const FROSTBOLT: u32 = 116;
const FROSTBOLT_DAMAGE: u32 = 228_597;
/// Mortal Strike: instant, `Speed` 0.
const MORTAL_STRIKE: u32 = 12_294;
const FRAME_SECS: f64 = 0.05;

fn place_npc(h: &mut Harness, x: f32) {
    *h.app.world_mut().get_mut::<Position>(h.npc).unwrap() = Position { x, y: 0.0, z: 0.0 };
}

/// Frames from the spell's CastSuccess line to its damage line, both logged to the
/// caster's connection (the same delivery delay), or `None` without damage in `limit`.
fn frames_from_launch_to_damage(
    h: &mut Harness,
    spell_id: u32,
    damage_spell: u32,
    limit: usize,
    mut in_flight: impl FnMut(&mut Harness),
) -> Option<usize> {
    let mut launched = None;
    for frame in 0..limit {
        h.advance(1);
        let log = std::mem::take(&mut h.inbox.log);
        if launched.is_none()
            && log
                .iter()
                .any(|e| e.kind == CombatLogKind::CastSuccess && e.spell_id == Some(spell_id))
        {
            launched = Some(frame);
            in_flight(h);
        }
        let damaged = log
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(damage_spell));
        if damaged {
            return Some(frame - launched.expect("damage after the launch"));
        }
    }
    assert!(launched.is_some(), "{:?}", h.inbox.failed);
    None
}

fn mage() -> Option<Harness> {
    harness(Vec::new(), &[FROSTBOLT, FROSTBOLT_DAMAGE], 8)
}

#[test]
fn frostbolt_at_20_yards_lands_its_flight_time_after_the_launch() {
    let Some(mut h) = mage() else {
        return;
    };
    place_npc(&mut h, 20.0);
    h.cast(FROSTBOLT, Some(h.npc));
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |_| {})
        .expect("the Frostbolt lands");
    // 20 yd / 35 yd/s = 0.571 s.
    let secs = frames as f64 * FRAME_SECS;
    assert!((0.55..=0.65).contains(&secs), "{secs} s");
}

#[test]
fn a_target_that_moves_in_flight_is_hit_at_the_launch_time_delay() {
    let Some(mut h) = mage() else {
        return;
    };
    place_npc(&mut h, 20.0);
    h.cast(FROSTBOLT, Some(h.npc));
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |h| {
        place_npc(h, 29.0)
    })
    .expect("the Frostbolt still lands");
    let secs = frames as f64 * FRAME_SECS;
    assert!((0.55..=0.65).contains(&secs), "{secs} s");
}

#[test]
fn a_target_that_dies_or_leaves_in_flight_is_not_hit() {
    let Some(mut h) = mage() else {
        return;
    };
    place_npc(&mut h, 20.0);
    let npc = h.npc;
    h.cast(FROSTBOLT, Some(npc));
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |h| {
        h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 0.0;
    });
    assert_eq!(frames, None, "no damage on a unit that died in flight");

    let Some(mut h) = mage() else {
        return;
    };
    place_npc(&mut h, 20.0);
    h.cast(FROSTBOLT, Some(h.npc));
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |h| {
        h.app.world_mut().despawn(h.npc);
    });
    assert_eq!(frames, None, "no damage on a unit that left the world");
}

#[test]
fn a_spell_without_missile_speed_lands_in_its_launch_tick() {
    let Some(mut h) = harness_with_native_enemy_dodge_reduction(Vec::new(), &[MORTAL_STRIKE], 1)
    else {
        return;
    };
    h.set_power(shared::components::PowerType::Rage, 1000.0);
    h.cast(MORTAL_STRIKE, Some(h.npc));
    let frames = frames_from_launch_to_damage(&mut h, MORTAL_STRIKE, MORTAL_STRIKE, 20, |_| {});
    assert_eq!(frames, Some(0));
}

fn in_combat(h: &Harness, unit: bevy::prelude::Entity) -> bool {
    h.app
        .world()
        .get::<shared::components::CombatStatus>(unit)
        .is_some_and(|status| status.0)
}

/// `PreprocessSpellLaunch` (Spell.cpp:8538-8541) puts the caster in combat and holds
/// the target's side suppressed; `PreprocessTarget` (Spell.cpp:2763-2764) engages the
/// target when the Frostbolt lands.
#[test]
fn a_missile_target_enters_combat_when_the_missile_lands_not_at_launch() {
    let Some(mut h) = mage() else {
        return;
    };
    for unit in [h.caster, h.npc] {
        h.app
            .world_mut()
            .entity_mut(unit)
            .insert(shared::components::CombatStatus(false));
    }
    place_npc(&mut h, 20.0);
    h.cast(FROSTBOLT, Some(h.npc));
    let mut in_flight = None;
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |h| {
        in_flight = Some((in_combat(h, h.caster), in_combat(h, h.npc)));
    })
    .expect("the Frostbolt lands");
    assert!(frames > 0);
    assert_eq!(in_flight, Some((true, false)), "(caster, target) in flight");
    assert!(in_combat(&h, h.caster) && in_combat(&h, h.npc));
}

/// Avenger's Shield: 35 yd/s, 3 chain targets, `SPELL_ATTR4_BOUNCY_CHAIN_MISSILES`.
const AVENGERS_SHIELD: u32 = 31_935;
/// Spell Reflection: REFLECT_SPELLS 100%, 1 proc charge.
const SPELL_REFLECTION: u32 = 23_920;

/// Seconds after the CastSuccess line of each Damage line of `spell_id`, by target.
fn damage_times(h: &mut Harness, spell_id: u32, frames: usize) -> Vec<(u64, f64)> {
    let mut launched = None;
    let mut hits = Vec::new();
    for frame in 0..frames {
        h.advance(1);
        for event in std::mem::take(&mut h.inbox.log) {
            if event.spell_id != Some(spell_id) {
                continue;
            }
            match event.kind {
                CombatLogKind::CastSuccess => launched = Some(frame),
                CombatLogKind::Damage => {
                    let after = frame - launched.expect("damage after the launch");
                    hits.push((event.target.unwrap(), after as f64 * FRAME_SECS));
                }
                _ => {}
            }
        }
    }
    hits
}

/// `Spell::AddUnitTarget` (Spell.cpp:2489-2505): each chained unit is hit the flight
/// from the unit before it after that unit's hit. Chain 20 yd → 12 yd → 3 yd: 571 ms,
/// + 8/35 s = 799 ms, + 9/35 s = 1056 ms (from the caster they would land at 571, 342
/// and 142 ms).
#[test]
fn a_bouncy_chain_missile_hits_each_unit_after_the_one_before_it() {
    let Some(mut h) = harness(Vec::new(), &[AVENGERS_SHIELD], 2) else {
        return;
    };
    let near = h.npc;
    let middle = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 12.0);
    let far = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 20.0);
    h.cast(AVENGERS_SHIELD, Some(far));
    let hits = damage_times(&mut h, AVENGERS_SHIELD, 40);
    let targets: Vec<u64> = hits.iter().map(|(target, _)| *target).collect();
    assert_eq!(
        targets,
        [far, middle, near].map(bevy::prelude::Entity::to_bits),
        "{hits:?} {:?}",
        h.inbox.failed
    );
    let secs: Vec<f64> = hits.iter().map(|(_, secs)| *secs).collect();
    assert!((0.55..=0.65).contains(&secs[0]), "{secs:?}");
    assert!((0.78..=0.88).contains(&secs[1]), "{secs:?}");
    assert!((1.04..=1.14).contains(&secs[2]), "{secs:?}");
}

fn apply_spell_reflection(h: &mut Harness, unit: bevy::prelude::Entity) {
    let effect_mask = crate::auras::aura_effect_mask(
        h.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(SPELL_REFLECTION)
            .unwrap(),
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: unit,
            target: unit,
            spell_id: SPELL_REFLECTION,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(1);
    assert!(crate::auras::has_aura(
        h.app.world(),
        unit,
        SPELL_REFLECTION
    ));
}

/// `SpellHitResult` (Object.cpp:1971-1976) reflects the Frostbolt: the Reflect miss is
/// logged at launch, Spell Reflection spends its charge when the missile reaches the
/// kobold (`ProcReflectDelayed`, 571 ms), and the Frostbolt lands on the mage 1.5 × that
/// after the launch (856 ms, Spell.cpp:2543-2544), its triggered damage included.
#[test]
fn a_reflected_frostbolt_turns_on_its_caster_after_one_and_a_half_flights() {
    let Some(mut h) = harness(
        Vec::new(),
        &[FROSTBOLT, FROSTBOLT_DAMAGE, SPELL_REFLECTION],
        8,
    ) else {
        return;
    };
    place_npc(&mut h, 20.0);
    let (mage, kobold) = (h.caster, h.npc);
    apply_spell_reflection(&mut h, kobold);
    h.cast(FROSTBOLT, Some(kobold));
    let mut launched = None;
    let mut reflected = false;
    let mut aura_gone_at = None;
    let mut mage_hit_at = None;
    for frame in 0..60 {
        h.advance(1);
        for event in std::mem::take(&mut h.inbox.log) {
            match event.kind {
                CombatLogKind::CastSuccess if event.spell_id == Some(FROSTBOLT) => {
                    launched = Some(frame)
                }
                CombatLogKind::Miss(shared::protocol::MissKind::Reflect) => {
                    assert_eq!(event.target, Some(kobold.to_bits()));
                    assert_eq!(launched, Some(frame), "the reflect is logged at launch");
                    reflected = true;
                }
                CombatLogKind::Damage if event.spell_id == Some(FROSTBOLT_DAMAGE) => {
                    assert_eq!(event.target, Some(mage.to_bits()));
                    mage_hit_at.get_or_insert(frame);
                }
                _ => {}
            }
        }
        if launched.is_some()
            && aura_gone_at.is_none()
            && !crate::auras::has_aura(h.app.world(), kobold, SPELL_REFLECTION)
        {
            aura_gone_at = Some(frame);
        }
    }
    let launched = launched.expect("the Frostbolt launches");
    assert!(reflected, "{:?}", h.inbox.failed);
    let secs = |frame: usize| (frame - launched) as f64 * FRAME_SECS;
    let aura_gone = secs(aura_gone_at.expect("the charge is spent"));
    let mage_hit = secs(mage_hit_at.expect("the Frostbolt hits the mage"));
    assert!((0.55..=0.70).contains(&aura_gone), "{aura_gone} s");
    assert!((0.85..=0.95).contains(&mage_hit), "{mage_hit} s");
    assert_eq!(h.health(kobold), 5000.0);
    assert!(h.health(mage) < 1000.0);
}

/// Reflect auras proc only off a reflect (`PROC_HIT_REFLECT`, SpellMgr.cpp:1847-1851):
/// a Mortal Strike, a melee ability that cannot be reflected, leaves Spell Reflection's
/// charge.
#[test]
fn an_ability_that_cannot_be_reflected_leaves_spell_reflection_up() {
    let Some(mut h) = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[MORTAL_STRIKE, SPELL_REFLECTION],
        1,
    ) else {
        return;
    };
    let kobold = h.npc;
    apply_spell_reflection(&mut h, kobold);
    h.set_power(shared::components::PowerType::Rage, 1000.0);
    h.cast(MORTAL_STRIKE, Some(kobold));
    let frames = frames_from_launch_to_damage(&mut h, MORTAL_STRIKE, MORTAL_STRIKE, 20, |_| {});
    assert_eq!(frames, Some(0));
    h.advance(2);
    assert!(crate::auras::has_aura(
        h.app.world(),
        kobold,
        SPELL_REFLECTION
    ));
}

/// Chain Lightning: 3 chain targets, instant travel.
const CHAIN_LIGHTNING: u32 = 188_443;
const HALVING_CHAIN_LIGHTNING: u32 = 900_001;

/// `SearchChainTargets` (Spell.cpp:2216-2320) jumps from each unit to the nearest one
/// within 10 yd (3 → 10 → 17 yd; 40 yd is out of reach), and each jump multiplies the
/// damage by `EffectChainAmplitude` (Spell.cpp:8631-8636): a copy of Chain Lightning at
/// 0.5 deals 1, 1/2 and 1/4 of its first hit.
#[test]
fn a_chain_spell_jumps_to_the_nearest_units_and_loses_its_amplitude_per_jump() {
    let Some(data) = crate::spell_cast::test_harness::world_db() else {
        return;
    };
    let mut halving = data.store.get(CHAIN_LIGHTNING).unwrap().clone();
    halving.id = HALVING_CHAIN_LIGHTNING;
    halving.effects[0].chain_amplitude = 0.5;
    halving.effects[0].variance = 0.0;
    let Some(mut h) = harness(vec![halving], &[], 7) else {
        return;
    };
    let first = h.npc;
    let second = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 10.0);
    let third = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 17.0);
    let beyond = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 40.0);
    h.cast(HALVING_CHAIN_LIGHTNING, Some(first));
    // A 2 s cast.
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let damage: Vec<(u64, i32)> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage && event.spell_id == Some(HALVING_CHAIN_LIGHTNING)
        })
        .map(|event| (event.target.unwrap(), event.amount))
        .collect();
    let targets: Vec<u64> = damage.iter().map(|(target, _)| *target).collect();
    assert_eq!(
        targets,
        [first, second, third].map(bevy::prelude::Entity::to_bits)
    );
    let full = damage[0].1 as f32;
    assert!(full > 0.0);
    assert!((damage[1].1 as f32 - full / 2.0).abs() <= 1.0, "{damage:?}");
    assert!((damage[2].1 as f32 - full / 4.0).abs() <= 1.0, "{damage:?}");
    assert_eq!(h.health(beyond), 5000.0);
}
