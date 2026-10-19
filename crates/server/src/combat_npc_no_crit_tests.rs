//! TC a352b1fa Unit.cpp2885–2893: NO_CRIT excludes NPC source, not later334.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Health, Npc, UnitFlags, UnitLevel};
use shared::protocol::{CombatEvent, CombatEventType};

use super::npc_expertise_tests::collect_broadcast_swings;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const DARION: u32 = 30_838;
const KOBOLD: u32 = 6;
const DUMMY: u32 = 31_144;
const NO_CRIT: u32 = 0x20000;
const NPC_BUFFS: u32 = 62_627;
const WEAPON_CRIT: u32 = 24_353;
const GENERAL_CRIT: u32 = 235_027;
const AUTO_CRIT: u32 = 340_094;
const SWINGS: usize = 256;

fn fixture(entry: u32) -> (Harness, Entity, Entity) {
    let mut h = harness(
        Vec::new(),
        &[NPC_BUFFS, WEAPON_CRIT, GENERAL_CRIT, AUTO_CRIT],
        1,
    )
    .unwrap();
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<CombatEvent>::default(),
        MessageReceiver::<CombatEvent>::default(),
    ));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    let attacker =
        crate::zones::summon_creature(h.app.world_mut(), caster, entry, [0.0, 0.0, 0.0, 0.0])
            .unwrap();
    let victim =
        crate::zones::summon_creature(h.app.world_mut(), caster, DUMMY, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(
        h.app.world().get::<Npc>(attacker).unwrap().template_id,
        entry
    );
    let template_flags = h
        .app
        .world()
        .resource::<crate::world_data::WorldData>()
        .templates[&entry]
        .flags_extra;
    let cached_flags = h
        .app
        .world()
        .get::<crate::zones::CreatureFlagsExtra>(attacker)
        .unwrap()
        .0;
    assert_eq!(cached_flags, template_flags);
    assert_eq!(cached_flags & NO_CRIT != 0, entry == DARION);
    assert!(
        h.app
            .world()
            .get::<UnitFlags>(attacker)
            .unwrap()
            .can_be_attacked_by(false)
    );
    assert!(
        h.app
            .world()
            .get::<UnitFlags>(victim)
            .unwrap()
            .can_be_attacked_by(false)
    );
    // Only authored hit/expertise, excluding62627 effect0's negative weapon52.
    apply(&mut h, attacker, NPC_BUFFS, (1 << 1) | (1 << 2));
    assert_eq!(modifier(&h, attacker, 52), 0.0);
    assert_eq!(modifier(&h, attacker, 290), 0.0);
    assert_eq!(modifier(&h, victim, 187), 0.0);
    assert_hit(&h, attacker);
    (h, attacker, victim)
}

fn modifier(h: &Harness, actor: Entity, kind: u32) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(actor)
        .map_or(0.0, |a| a.modifier(kind))
}

fn apply(h: &mut Harness, actor: Entity, spell_id: u32, effect_mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), actor, spell_id));
}

fn assert_hit(h: &Harness, attacker: Entity) {
    assert_eq!(modifier(h, attacker, 54), 100.0);
    assert_eq!(modifier(h, attacker, 240), 10_000.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(attacker)
            .unwrap()
            .melee_hit_chance_pct,
        100.0
    );
}

fn swings(h: &mut Harness, attacker: Entity, victim: Entity, count: usize) -> Vec<CombatEvent> {
    let events = collect_broadcast_swings(h, attacker, victim, count);
    assert_hit(h, attacker);
    assert!(
        h.app.world().get::<Health>(victim).unwrap().current > 0.0,
        "native target exhausted: {events:?}"
    );
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit
        ) && e.amount > 0.0),
        "{events:?}"
    );
    events
}

#[test]
fn native_melee_npc_no_crit_actor_and_unchanged_target_are_runtime_qualified() {
    let (mut h, attacker, victim) = fixture(DARION);
    let attacker_level = h.app.world().get::<UnitLevel>(attacker).unwrap().0;
    let victim_level = h.app.world().get::<UnitLevel>(victim).unwrap().0;
    let before = *h.app.world().get::<Health>(victim).unwrap();
    let events = swings(&mut h, attacker, victim, SWINGS);
    let after = *h.app.world().get::<Health>(victim).unwrap();
    assert_eq!(after.max, before.max);
    assert_eq!(
        h.app.world().get::<UnitLevel>(attacker).unwrap().0,
        attacker_level
    );
    assert_eq!(
        h.app.world().get::<UnitLevel>(victim).unwrap().0,
        victim_level
    );
    eprintln!(
        "actual30838flags{} level{} vs31144level{};256ordinaryswings;nativeHP{}→{}/{};maxevent{}",
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(attacker)
            .unwrap()
            .0,
        attacker_level,
        victim_level,
        before.current,
        after.current,
        after.max,
        events.iter().map(|e| e.amount).fold(0.0_f32, f32::max)
    );
}

#[test]
fn native_melee_npc_no_crit_buff_coexistence_and_lifetime_are_runtime_qualified() {
    let (mut h, attacker, _) = fixture(DARION);
    apply(&mut h, attacker, WEAPON_CRIT, 1);
    apply(&mut h, attacker, GENERAL_CRIT, 1);
    // Native340094 effect1 is334; effect0 aura107 must remain absent.
    apply(&mut h, attacker, AUTO_CRIT, 1 << 1);
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed.as_secs_f64() > SWINGS as f64 * 2.0);
    for spell in [NPC_BUFFS, WEAPON_CRIT, GENERAL_CRIT, AUTO_CRIT] {
        assert!(crate::auras::has_aura(h.app.world(), attacker, spell));
    }
    assert_eq!(modifier(&h, attacker, 52), 1000.0);
    assert_eq!(modifier(&h, attacker, 290), 100.0);
    assert_eq!(modifier(&h, attacker, 334), 100.0);
    assert_eq!(modifier(&h, attacker, 107), 0.0);
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(attacker)
        .unwrap();
    assert_eq!(mods.npc_crit_pct, 1100.0);
    assert_eq!(mods.autoattack_crit_pct, 100.0);
    assert_hit(&h, attacker);
    eprintln!(
        "native30838 actual52+290=1100/334100/hit100/expertise10000 remain{}s;107absent",
        elapsed.as_secs_f64()
    );
}

#[test]
fn native_melee_npc_no_crit_unflagged_actor_is_runtime_qualified() {
    let (mut h, attacker, victim) = fixture(KOBOLD);
    let events = swings(&mut h, attacker, victim, SWINGS);
    assert_eq!(events.len(), SWINGS);
}
