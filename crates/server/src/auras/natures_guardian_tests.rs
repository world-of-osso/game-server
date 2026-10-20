//! Active70448 receipt-matching inputs; TC a352b1fa spell_shaman.cpp2348–2376.
//! Qualification only: no Nature's Guardian script consumer/chain certificate.
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

use super::{ActiveAuras, AuraInstance, AuraSource};
use crate::spell_cast::test_harness::{Harness, harness, spawn_npc};
use crate::spell_info::SpellStore;

const PASSIVE: u32 = 30_884;
const COOLDOWN: u32 = 445_698;
const HEAL: u32 = 31_616;
const LIGHTNING_BOLT: u32 = 188_196;
const WOUND: u32 = 209_140;
const NPC_BUFFS: u32 = 62_627;

#[path = "natures_guardian_behavior_tests.rs"]
mod behavior;

fn fixture(passive: bool) -> Harness {
    let mut ids = vec![COOLDOWN, HEAL, LIGHTNING_BOLT, WOUND, NPC_BUFFS];
    if passive {
        ids.push(PASSIVE);
    }
    let mut h = harness(Vec::new(), &ids, 7).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::ActiveSpec(262));
    h.advance(3);
    h
}

fn aura(h: &Harness, unit: Entity, spell: u32) -> Option<AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(unit)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == spell))
        .cloned()
}

fn cast_native_wound(h: &mut Harness, target: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, WOUND, Some(target), false).unwrap();
    // Native209140 SpellMisc183160/CastingTime5 is2000ms, not instant.
    let dt = h.app.world().resource::<Time>().delta_secs();
    let frames = h.app.world().get::<CastState>(h.npc).map_or(0, |cast| {
        ((cast.duration - cast.elapsed).max(0.0) / dt).ceil() as usize
    });
    h.advance(frames + 3);
}

fn buff_native_npc(h: &mut Harness, npc: Entity) {
    super::apply_aura(
        h.app.world_mut(),
        &super::AuraApplication {
            caster: npc,
            target: npc,
            spell_id: NPC_BUFFS,
            effect_mask: 0b111,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
    let auras = h.app.world().get::<ActiveAuras>(npc).unwrap();
    assert_eq!(auras.modifier(52), -100.0);
    assert_eq!(auras.modifier(54), 100.0);
    assert_eq!(auras.modifier(240), 10000.0);
}

fn swing_until_health(h: &mut Harness, actors: &[Entity], expected_health: f32) {
    let starts = actors.to_vec();
    let victim = h.caster;
    h.app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            for actor in &starts {
                crate::combat::start_attack(&mut commands, *actor, victim, None);
            }
        })
        .unwrap();
    for _ in 0..66 * 60 {
        h.advance(1);
        if h.health(victim) <= expected_health {
            assert_eq!(
                h.health(victim),
                expected_health,
                "native damage path changed; never rewrite HP"
            );
            for actor in actors {
                crate::combat::attack_stop(h.app.world_mut(), *actor);
            }
            h.advance(3);
            return;
        }
    }
    panic!(
        "native melee did not reach {expected_health}: health{} events{:?}",
        h.health(victim),
        h.inbox.log
    );
}

#[test]
fn native_natures_guardian_limited_cooldown_preserves_actual_player_cast_and_expires() {
    let mut h = fixture(true);
    let (source, target) = (h.caster, h.npc);
    let passive = aura(&h, source, PASSIVE).unwrap();
    assert_eq!(passive.source, AuraSource::KnownSpell);
    assert_eq!(passive.caster, source);
    assert!(passive.expires_at.is_none());
    let effect = passive
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert_eq!(
        (effect.aura_type, effect.amount, effect.trigger_spell),
        (4, 35.0, HEAL)
    );
    let store = h.app.world().resource::<SpellStore>();
    let options = store.get(PASSIVE).unwrap().aura_options.as_ref().unwrap();
    assert_eq!(options.proc_chance, 100);
    assert_eq!(options.proc_type_mask, [0x800a_aaa8, 0]);
    assert_eq!(
        (options.proc_charges, options.proc_category_recovery_ms),
        (0, 0)
    );
    assert!(store.get(HEAL).unwrap().power_costs.is_empty());
    assert!(store.get(COOLDOWN).unwrap().power_costs.is_empty());
    h.cast_and_deliver(LIGHTNING_BOLT, Some(target));
    let before = h
        .app
        .world()
        .get::<CastState>(source)
        .expect("ordinary player Lightning Bolt must still be casting")
        .clone();
    assert_eq!(before.spell_id, LIGHTNING_BOLT);
    crate::spell_cast::creature_cast_ignoring_in_progress(
        h.app.world_mut(),
        source,
        COOLDOWN,
        Some(source),
    )
    .expect("existing limited path must support actual player self-cast");
    let after = h
        .app
        .world()
        .get::<CastState>(source)
        .expect("limited instant child must not replace/remove original player cast");
    assert_eq!(
        (
            after.spell_id,
            after.target,
            after.cast_type,
            after.duration,
            after.elapsed
        ),
        (
            before.spell_id,
            before.target,
            before.cast_type,
            before.duration,
            before.elapsed
        )
    );
    let cooldown = aura(&h, source, COOLDOWN).unwrap();
    assert_eq!(cooldown.caster, source);
    assert_eq!(cooldown.duration_ms, 45000);
    let expiry = cooldown.expires_at.unwrap();
    h.advance(120);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.app.world().get::<CastState>(source).is_none());
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(LIGHTNING_BOLT)
                && event.kind == CombatLogKind::Damage
                && event.source == Some(source.to_bits())
                && event.target == Some(target.to_bits())
                && event.amount > 0)
    );
    while h.app.world().resource::<Time>().elapsed_secs_f64() + 0.1 < expiry {
        assert!(super::has_aura(h.app.world(), source, COOLDOWN));
        h.advance(1);
    }
    h.advance(4);
    assert!(!super::has_aura(h.app.world(), source, COOLDOWN));
    eprintln!(
        "KnownSpell30884/A4threshold35/trigger31616; existing limited445698 preserves actual player188196 state+owned damage and native45000ms expiry"
    );
}

#[test]
fn native_natures_guardian_direct_heal_and_nonlethal_repeat_window_are_qualified() {
    let mut h = fixture(false);
    let (source, first) = (h.caster, h.npc);
    cast_native_wound(&mut h, source);
    assert_eq!(h.health(source), 200.0);
    crate::spell_cast::cast_triggered(h.app.world_mut(), source, HEAL, Some(source), 1);
    h.advance(3);
    let heal = h
        .inbox
        .log
        .iter()
        .find(|event| event.spell_id == Some(HEAL) && event.kind == CombatLogKind::Heal)
        .unwrap();
    assert_eq!(
        (heal.source, heal.target, heal.amount, heal.overflow),
        (Some(source.to_bits()), Some(source.to_bits()), 400, 0)
    );
    assert_eq!(h.health(source), 600.0);
    let second = spawn_npc(&mut h.app, 3.0);
    buff_native_npc(&mut h, first);
    buff_native_npc(&mut h, second);
    let started = h.app.world().resource::<Time>().elapsed_secs_f64();
    let first_event = h.inbox.log.len();
    swing_until_health(&mut h, &[first, second], 340.0);
    let elapsed = h.app.world().resource::<Time>().elapsed_secs_f64() - started;
    assert!(
        elapsed < 45.0,
        "native repeat path misses cooldown window: {elapsed}"
    );
    let hits: Vec<_> = h.inbox.log[first_event..]
        .iter()
        .filter(|event| {
            event.spell_id.is_none()
                && event.kind == CombatLogKind::Damage
                && event.target == Some(source.to_bits())
        })
        .collect();
    assert_eq!(hits.len(), 26);
    assert!(hits.iter().all(|event| event.amount == 10 && !event.crit));
    eprintln!(
        "actual209140800 wound1000→200;native31616realheal400→600;26 ordinary twoNPC10damage hits→340 in{elapsed}s<45;noHP/nativeamount/timer/RNGwrites"
    );
}

#[test]
fn native_natures_guardian_exact_thirty_five_percent_damage_path_is_qualified() {
    let mut h = fixture(false);
    let (source, attacker) = (h.caster, h.npc);
    buff_native_npc(&mut h, attacker);
    assert_eq!(h.health(source), 1000.0);
    swing_until_health(&mut h, &[attacker], 350.0);
    let hits: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.spell_id.is_none() && event.kind == CombatLogKind::Damage)
        .collect();
    assert_eq!(hits.len(), 65);
    assert!(hits.iter().all(|event| event.amount == 10 && !event.crit));
    swing_until_health(&mut h, &[attacker], 340.0);
    assert_eq!(h.health(source), 340.0);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Health>(source)
            .unwrap()
            .max,
        1000.0
    );
    eprintln!(
        "65 actual native NPC10 hits1000→350;next ordinary10→340;strict35% boundary qualified without HP writes"
    );
}
