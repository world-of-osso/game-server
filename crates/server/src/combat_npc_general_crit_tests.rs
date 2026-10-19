//! TC a352b1fa Unit.cpp2889–2890: NPC source crit includes both52 and290.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Position, UnitLevel};

use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const NPC_BUFFS: u32 = 62_627;
const POSITIVE: u32 = 235_027;
const NEGATIVE: u32 = 426_659;
const AUTO_CRIT: u32 = 277_903;
const HIT_EXPERTISE_MASK: u32 = (1 << 1) | (1 << 2);
const SWINGS: usize = 256;

fn fixture(npc_buff_mask: u32) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[NPC_BUFFS, POSITIVE, NEGATIVE, AUTO_CRIT], 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
    h.app.world_mut().entity_mut(h.npc).insert((
        UnitLevel(80),
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<shared::protocol::CombatEvent>::default(),
        MessageReceiver::<shared::protocol::CombatEvent>::default(),
    ));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    // Behind native equal-level31144: no parry/block, but native dodge still needs240.
    let victim =
        crate::zones::summon_creature(h.app.world_mut(), caster, 31_144, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(victim).unwrap().0, 80);
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(h.npc)
            .unwrap()
            .0,
        2
    );
    assert_eq!(modifier(&h, victim, 187), 0.0);
    let npc = h.npc;
    apply(&mut h, npc, NPC_BUFFS, npc_buff_mask);
    assert_eq!(
        modifier(&h, npc, 52),
        if npc_buff_mask & 1 == 0 { 0.0 } else { -100.0 }
    );
    assert_eq!(modifier(&h, npc, 290), 0.0);
    assert_hit(&h);
    (h, victim)
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

fn assert_hit(h: &Harness) {
    assert_eq!(modifier(h, h.npc, 54), 100.0);
    assert_eq!(modifier(h, h.npc, 240), 10_000.0);
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.npc)
        .unwrap();
    assert_eq!(mods.melee_hit_chance_pct, 100.0);
    assert_eq!(super::creature_miss_chance(Some(mods)), 0.0);
}

fn qualify_lifetime(h: &mut Harness, spells: &[u32]) {
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed.as_secs_f64() > SWINGS as f64 * 2.0);
    for spell in spells {
        assert!(crate::auras::has_aura(h.app.world(), h.npc, *spell));
    }
    assert_hit(h);
    eprintln!(
        "native290 qualification{}s without refresh, HP/timer/RNG edits",
        elapsed.as_secs_f64()
    );
}

#[test]
fn native_melee_npc_general_crit_signed_inputs_are_runtime_qualified() {
    let (mut h, _) = fixture(HIT_EXPERTISE_MASK);
    let npc = h.npc;
    apply(&mut h, npc, NEGATIVE, 1 << 1);
    apply(&mut h, npc, POSITIVE, 1); // Positive LAST: raw290 must sum to-100.
    apply(&mut h, npc, AUTO_CRIT, 1);
    assert_eq!(modifier(&h, npc, 52), 0.0);
    assert_eq!(modifier(&h, npc, 290), -100.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    qualify_lifetime(&mut h, &[NPC_BUFFS, NEGATIVE, POSITIVE, AUTO_CRIT]);
    assert_eq!(modifier(&h, npc, 52), 0.0);
    assert_eq!(modifier(&h, npc, 290), -100.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(npc)
            .unwrap()
            .crit_pct,
        -100.0
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(npc)
            .unwrap()
            .autoattack_crit_pct,
        50.0
    );
}

#[test]
fn native_melee_npc_general_crit_weapon_combination_inputs_are_runtime_qualified() {
    let (mut h, _) = fixture(7);
    let npc = h.npc;
    apply(&mut h, npc, POSITIVE, 1);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    assert_eq!(modifier(&h, npc, 290), 100.0);
    qualify_lifetime(&mut h, &[NPC_BUFFS, POSITIVE]);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    assert_eq!(modifier(&h, npc, 290), 100.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(npc)
            .unwrap()
            .crit_pct,
        100.0
    );
}
