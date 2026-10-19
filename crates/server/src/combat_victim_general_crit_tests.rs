//! TC a352b1fa Unit.cpp2924/2926: victim197 joins187 BEFORE clamp and334.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, UnitLevel};

use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const POSITIVE: u32 = 291_576;
const NEGATIVE: u32 = 1_248_794;
const NPC_BUFFS: u32 = 62_627;
const AUTO_CRIT: u32 = 277_903;
const MELEE_REDUCTION: u32 = 460_575;
const SWINGS: usize = 256;

fn fixture() -> (Harness, Entity) {
    let mut h = harness(
        Vec::new(),
        &[POSITIVE, NEGATIVE, NPC_BUFFS, AUTO_CRIT, MELEE_REDUCTION],
        1,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
    h.app.world_mut().entity_mut(h.npc).insert((
        UnitLevel(80),
        shared::components::Position {
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
    // Targetx3faces+x: both actual NPC0 and player0 attack from behind, in range.
    let victim =
        crate::zones::summon_creature(h.app.world_mut(), caster, 31_144, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(victim).unwrap().0, 80);
    assert_eq!(modifier(&h, victim, 187), 0.0);
    assert_eq!(modifier(&h, victim, 197), 0.0);
    let npc = h.npc;
    apply(&mut h, npc, NPC_BUFFS, (1 << 1) | (1 << 2));
    assert_eq!(modifier(&h, npc, 52), 0.0);
    assert_hit(&h, npc);
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

fn assert_hit(h: &Harness, npc: Entity) {
    assert_eq!(modifier(h, npc, 54), 100.0);
    assert_eq!(modifier(h, npc, 240), 10_000.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(npc)
            .unwrap()
            .melee_hit_chance_pct,
        100.0
    );
}

#[test]
fn native_melee_victim_general_crit_signed_coexistence_is_runtime_qualified() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, NEGATIVE, 1 << 1);
    apply(&mut h, victim, POSITIVE, 1); // Positive LAST, not last-effect-wins.
    let npc = h.npc;
    apply(&mut h, npc, AUTO_CRIT, 1);
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed.as_secs_f64() > SWINGS as f64 * 2.0);
    for spell in [POSITIVE, NEGATIVE] {
        assert!(crate::auras::has_aura(h.app.world(), victim, spell));
    }
    assert_eq!(modifier(&h, victim, 197), -1.0);
    assert_eq!(modifier(&h, victim, 187), 0.0);
    assert_eq!(modifier(&h, npc, 197), 0.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    assert_hit(&h, npc);
    eprintln!(
        "native291576effect0+1248794effect1 victim197=-1 remains{}s;187/attacker197=0;33450",
        elapsed.as_secs_f64()
    );
}

#[test]
fn native_melee_victim_general_crit_other_modifiers_and_player_are_runtime_qualified() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, POSITIVE, 1);
    apply(&mut h, victim, MELEE_REDUCTION, 1 << 2);
    let npc = h.npc;
    // Full native62627 includes52=-100 as well as the qualified hit/expertise.
    crate::auras::remove_auras(
        h.app.world_mut(),
        npc,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == NPC_BUFFS,
    );
    apply(&mut h, npc, NPC_BUFFS, 7);
    let caster = h.caster;
    apply(&mut h, caster, AUTO_CRIT, 1);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct,
        0.0
    );
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    assert!(
        (h.app.world().resource::<Time>().elapsed() - start).as_secs_f64() > SWINGS as f64 * 2.0
    );
    assert_eq!(modifier(&h, victim, 197), 100.0);
    assert_eq!(modifier(&h, victim, 187), -60.0);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    assert_eq!(modifier(&h, caster, 197), 0.0);
    assert_eq!(modifier(&h, caster, 334), 50.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct,
        0.0
    );
    assert_hit(&h, npc);
}

#[test]
fn native_melee_victim_general_crit_no_crit_source_pair_is_runtime_qualified() {
    let (mut h, victim) = fixture();
    let caster = h.caster;
    let darion =
        crate::zones::summon_creature(h.app.world_mut(), caster, 30_838, [0.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(darion)
            .unwrap()
            .0,
        0x20000
    );
    apply(&mut h, darion, NPC_BUFFS, (1 << 1) | (1 << 2));
    apply(&mut h, victim, POSITIVE, 1);
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    assert!(
        (h.app.world().resource::<Time>().elapsed() - start).as_secs_f64() > SWINGS as f64 * 2.0
    );
    assert_eq!(modifier(&h, victim, 197), 100.0);
    assert_eq!(modifier(&h, darion, 197), 0.0);
    assert_eq!(modifier(&h, darion, 52), 0.0);
    assert_hit(&h, darion);
}
