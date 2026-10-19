//! TC a352b1fa Unit.cpp2916–2921: victim339 matches actual TempSummon.summoner.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, UnitLevel};
use shared::protocol::{CombatEvent, CombatEventType};

use super::npc_expertise_tests::collect_broadcast_swings;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const SUMMONER_CRIT: u32 = 344_021;
const NPC_BUFFS: u32 = 62_627;
const APPLICATIONS: usize = 10;
const SWINGS_PER_APPLICATION: usize = 2;

fn fixture() -> (Harness, Entity, Entity) {
    let mut h = harness(Vec::new(), &[SUMMONER_CRIT, NPC_BUFFS], 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<CombatEvent>::default(),
        MessageReceiver::<CombatEvent>::default(),
    ));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let owner = h.caster;
    let attacker = spawn_darion(&mut h, owner);
    let victim =
        crate::zones::summon_creature(h.app.world_mut(), owner, 31_144, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(victim).unwrap().0, 80);
    (h, attacker, victim)
}

fn spawn_darion(h: &mut Harness, summoner: Entity) -> Entity {
    let actor =
        crate::zones::summon_creature(h.app.world_mut(), summoner, 30_838, [0.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::TempSummon>(actor)
            .unwrap()
            .summoner,
        summoner
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(actor)
            .unwrap()
            .0,
        0x20000
    );
    apply(h, actor, actor, NPC_BUFFS, (1 << 1) | (1 << 2));
    assert_hit(h, actor);
    actor
}

fn apply(h: &mut Harness, caster: Entity, target: Entity, spell_id: u32, effect_mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), target, spell_id));
}

fn modifier(h: &Harness, target: Entity, aura_type: u32) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(target)
        .map_or(0.0, |a| a.modifier(aura_type))
}

fn assert_hit(h: &Harness, attacker: Entity) {
    assert_eq!(modifier(h, attacker, 54), 100.0);
    assert_eq!(modifier(h, attacker, 240), 10_000.0);
    assert_eq!(modifier(h, attacker, 52), 0.0);
    assert_eq!(modifier(h, attacker, 290), 0.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(attacker)
            .unwrap()
            .melee_hit_chance_pct,
        100.0
    );
}

fn assert_native_window(h: &Harness, recipient: Entity, caster: Entity) -> f64 {
    let auras = h.app.world().get::<ActiveAuras>(recipient).unwrap();
    let aura = auras.iter().find(|a| a.spell_id == SUMMONER_CRIT).unwrap();
    assert_eq!(aura.caster, caster);
    assert_eq!(aura.duration_ms, 6000);
    let expiry = aura.expires_at.unwrap();
    assert!(expiry > h.app.world().resource::<Time>().elapsed_secs_f64());
    assert_eq!(auras.modifier(339), 50.0);
    assert_eq!(auras.modifier(308), 0.0); // Native sibling effect0 EXCLUDED.
    assert_eq!(auras.modifier(306), 0.0);
    expiry
}

fn wait_for_natural_expiry(h: &mut Harness, recipient: Entity, expiry: f64) {
    // Ordinary50ms updates only, no removal/refresh/duration/clock mutation.
    for _ in 0..160 {
        if !crate::auras::has_aura(h.app.world(), recipient, SUMMONER_CRIT) {
            assert!(h.app.world().resource::<Time>().elapsed_secs_f64() >= expiry);
            assert_eq!(modifier(h, recipient, 339), 0.0);
            return;
        }
        h.advance(1);
    }
    panic!("native344021 did not expire naturally");
}

fn sample_native_applications(
    h: &mut Harness,
    attacker: Entity,
    victim: Entity,
    aura_caster: Entity,
    recipient: Entity,
) -> Vec<CombatEvent> {
    let mut events = Vec::new();
    for application in 0..APPLICATIONS {
        assert!(!crate::auras::has_aura(
            h.app.world(),
            recipient,
            SUMMONER_CRIT
        ));
        apply(h, aura_caster, recipient, SUMMONER_CRIT, 1 << 1);
        let expiry = assert_native_window(h, recipient, aura_caster);
        let window = collect_broadcast_swings(h, attacker, victim, SWINGS_PER_APPLICATION);
        assert_eq!(assert_native_window(h, recipient, aura_caster), expiry);
        assert_eq!(window.len(), SWINGS_PER_APPLICATION);
        events.extend(window);
        wait_for_natural_expiry(h, recipient, expiry);
        eprintln!(
            "native339application{}:2ordinaryattemptsinside6s,naturalexpiryBEFOREnext",
            application + 1
        );
    }
    assert_eq!(events.len(), APPLICATIONS * SWINGS_PER_APPLICATION);
    events
}

#[test]
fn native_melee_victim_summoner_crit_ten_short_owner_windows_are_runtime_qualified() {
    let (mut h, attacker, victim) = fixture();
    let owner = h.caster;
    let events = sample_native_applications(&mut h, attacker, victim, owner, victim);
    assert_hit(&h, attacker);
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit
        ) && e.amount > 0.0),
        "{events:?}"
    );
}

#[test]
fn native_melee_victim_summoner_crit_nested_direct_owner_windows_are_runtime_qualified() {
    let (mut h, parent, victim) = fixture();
    let grandparent = h.caster;
    let attacker = spawn_darion(&mut h, parent);
    assert_ne!(parent, grandparent);
    let events = sample_native_applications(&mut h, attacker, victim, grandparent, victim);
    assert_hit(&h, attacker);
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit
        ) && e.amount > 0.0),
        "{events:?}"
    );
}

#[test]
fn native_melee_victim_summoner_crit_player_is_not_temp_summon_is_runtime_qualified() {
    let (mut h, _, victim) = fixture();
    let player = h.caster;
    assert!(
        h.app
            .world()
            .get::<crate::zones::TempSummon>(player)
            .is_none()
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(player)
            .unwrap()
            .crit_pct,
        0.0
    );
    let events = sample_native_applications(&mut h, player, victim, player, victim);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(player)
            .unwrap()
            .crit_pct,
        0.0
    );
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage
                | CombatEventType::CriticalHit
                | CombatEventType::Miss
                | CombatEventType::Dodge
        )),
        "{events:?}"
    );
}
