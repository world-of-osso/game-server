//! Active70448 receipt-matching Spirit Wolf data; TC a352b1fa2480–2510.
//! Qualification fixtures are native data-boundary proof, not legal loadouts.
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

use super::{ActiveAuras, AuraInstance, AuraSource, stat_mods::AuraStatMods};
use crate::spell_cast::test_harness::{Harness, harness};
use crate::spell_info::SpellStore;

const GHOST_WOLF: u32 = 2_645;
const TALENT: u32 = 260_878;
const PERIODIC: u32 = 260_882;
const BUFF: u32 = 260_881;
const NATIVE_DAMAGE: u32 = 314_616;

fn fixture(talent: bool) -> Harness {
    let mut ids = vec![GHOST_WOLF, PERIODIC, BUFF, NATIVE_DAMAGE];
    if talent {
        ids.push(TALENT);
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

fn speed(h: &Harness, unit: Entity) -> f32 {
    h.app
        .world()
        .get::<AuraStatMods>(unit)
        .unwrap()
        .movement_speed
}

fn set_talent(h: &mut Harness, enabled: bool) {
    let mut known = vec![GHOST_WOLF, PERIODIC, BUFF, NATIVE_DAMAGE];
    if enabled {
        known.push(TALENT);
    }
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(known));
    if !enabled {
        super::remove_auras(
            h.app.world_mut(),
            h.caster,
            super::RemoveReason::Cancelled,
            |aura| aura.spell_id == TALENT,
        );
    }
    h.advance(3);
}

fn cancel_ghost(h: &mut Harness) {
    super::remove_auras(
        h.app.world_mut(),
        h.caster,
        super::RemoveReason::Cancelled,
        |aura| aura.spell_id == GHOST_WOLF,
    );
    h.advance(3);
}

fn assert_chain(h: &Harness, unit: Entity, stacks: u8) {
    let periodic =
        aura(h, unit, PERIODIC).expect("Ghost Wolf + actual talent must start native260882");
    assert_eq!(periodic.caster, unit);
    assert!(periodic.expires_at.is_none());
    let buff = aura(h, unit, BUFF).expect("native1000ms period must cast260881");
    assert_eq!(buff.caster, unit);
    assert_eq!(buff.stacks, stacks);
    assert!((speed(h, unit) - 1.3 * (1.0 + 0.05 * f32::from(stacks))).abs() < 1e-6);
}

fn cast_ghost(h: &mut Harness) {
    h.cast_and_deliver(GHOST_WOLF, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(super::has_aura(h.app.world(), h.caster, GHOST_WOLF));
}

#[test]
fn native_spirit_wolf_ghost_and_passive_inputs_are_qualified() {
    let mut h = fixture(true);
    let passive = aura(&h, h.caster, TALENT).unwrap();
    assert_eq!(passive.source, AuraSource::KnownSpell);
    assert_eq!(passive.caster, h.caster);
    assert!(passive.expires_at.is_none());
    assert!(
        passive
            .effects
            .iter()
            .any(|effect| effect.index == 0 && effect.aura_type == 4 && effect.amount == 50.0)
    );
    cast_ghost(&mut h);
    let ghost = aura(&h, h.caster, GHOST_WOLF).unwrap();
    assert!(ghost.expires_at.is_none());
    assert!(
        ghost
            .effects
            .iter()
            .any(|effect| effect.aura_type == 36 && effect.misc_values[0] == 16)
    );
    assert!((speed(&h, h.caster) - 1.3).abs() < 1e-6);
    let store = h.app.world().resource::<SpellStore>();
    let periodic = store.get(PERIODIC).unwrap();
    let effect = periodic
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert_eq!(
        (effect.aura, effect.aura_period_ms, effect.trigger_spell),
        (23, 1000, BUFF)
    );
    assert_eq!(periodic.misc.as_ref().unwrap().attributes[5], 1024);
    assert_eq!(periodic.misc.as_ref().unwrap().attributes[5] & 0x200, 0);
    eprintln!(
        "ordinary2645 permanentGhost/form16/speed1.3; KnownSpell260878A4amount50; native260882A23/1000ms/noinitialperiod qualified"
    );
}

#[test]
fn native_spirit_wolf_direct_period_stack_cap_speed_and_incoming_damage_are_qualified() {
    let mut h = fixture(false);
    let source = h.caster;
    assert!(!super::has_aura(h.app.world(), source, TALENT));
    let started = h.app.world().resource::<Time>().elapsed_secs_f64();
    crate::spell_cast::cast_triggered(h.app.world_mut(), source, PERIODIC, Some(source), 1);
    let periodic = aura(&h, source, PERIODIC).unwrap();
    assert!(periodic.expires_at.is_none());
    assert!(aura(&h, source, BUFF).is_none());
    let mut growth = Vec::new();
    for _ in 0..120 {
        h.advance(1);
        let stacks = aura(&h, source, BUFF).map_or(0, |aura| aura.stacks);
        if usize::from(stacks) > growth.len() {
            assert_eq!(usize::from(stacks), growth.len() + 1);
            growth.push(h.app.world().resource::<Time>().elapsed_secs_f64() - started);
            assert!((speed(&h, source) - (1.0 + 0.05 * f32::from(stacks))).abs() < 1e-6);
        }
    }
    assert_eq!(growth.len(), 4, "{growth:?}");
    for (actual, expected) in growth.iter().zip([1.0, 2.0, 3.0, 4.0]) {
        assert!((actual - expected).abs() <= 0.1, "{growth:?}");
    }
    let buff = aura(&h, source, BUFF).unwrap();
    assert_eq!((buff.stacks, buff.max_stacks), (4, 4));
    assert!(buff.expires_at.is_none());
    assert!(
        buff.effects
            .iter()
            .any(|effect| effect.index == 0 && effect.aura_type == 129 && effect.amount == 5.0)
    );
    assert!(buff.effects.iter().any(|effect| effect.index == 1
        && effect.aura_type == 87
        && effect.amount == -5.0
        && effect.misc_values[0] == 127));
    assert!((speed(&h, source) - 1.2).abs() < 1e-6);
    assert_eq!(h.health(source), 1000.0);
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, NATIVE_DAMAGE, Some(source), false)
        .unwrap();
    h.advance(5);
    let hit = h
        .inbox
        .log
        .iter()
        .find(|event| event.spell_id == Some(NATIVE_DAMAGE) && event.kind == CombatLogKind::Damage)
        .unwrap();
    assert_eq!(hit.source, Some(h.npc.to_bits()));
    assert_eq!(hit.target, Some(source.to_bits()));
    assert_eq!((hit.amount, hit.absorbed, hit.overflow), (800, 0, 0));
    assert_eq!(h.health(source), 200.0);
    eprintln!(
        "native260882 natural1/2/3/4s growth{growth:?}/cap4/permanent;native129speed1.2/native87actual314616damage800/HP200/noHPtimerRNGwrites"
    );
}

#[test]
#[ignore = "behavioral RED before Spirit Wolf parent-chain hook"]
fn native_spirit_wolf_talent_then_form_natural_stacks_cap_speed_and_damage() {
    let mut h = fixture(true);
    cast_ghost(&mut h);
    assert!(
        aura(&h, h.caster, PERIODIC).is_some(),
        "missing native260882"
    );
    assert!(aura(&h, h.caster, BUFF).is_none());
    h.advance(22);
    assert_chain(&h, h.caster, 1);
    for stacks in 2..=4 {
        h.advance(20);
        assert_chain(&h, h.caster, stacks);
    }
    h.advance(80);
    assert_chain(&h, h.caster, 4);
    let source = h.caster;
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, NATIVE_DAMAGE, Some(source), false)
        .unwrap();
    h.advance(5);
    assert_eq!(h.health(source), 200.0);
    let hit = h
        .inbox
        .log
        .iter()
        .find(|event| event.spell_id == Some(NATIVE_DAMAGE) && event.kind == CombatLogKind::Damage)
        .unwrap();
    assert_eq!((hit.amount, hit.absorbed, hit.overflow), (800, 0, 0));
}

#[test]
#[ignore = "behavioral RED before Spirit Wolf talent-after-form hook"]
fn native_spirit_wolf_form_then_learned_talent_activates_native_periodic() {
    let mut h = fixture(true);
    set_talent(&mut h, false);
    cast_ghost(&mut h);
    h.advance(60);
    assert!(aura(&h, h.caster, PERIODIC).is_none());
    set_talent(&mut h, true);
    assert!(aura(&h, h.caster, TALENT).is_some());
    assert!(
        aura(&h, h.caster, PERIODIC).is_some(),
        "learning actual talent while in Ghost Wolf must start260882"
    );
    h.advance(22);
    assert_chain(&h, h.caster, 1);
}

#[test]
#[ignore = "native Spirit Wolf no-talent control"]
fn native_spirit_wolf_form_without_talent_has_no_children() {
    let mut h = fixture(false);
    cast_ghost(&mut h);
    h.advance(120);
    assert!(aura(&h, h.caster, PERIODIC).is_none());
    assert!(aura(&h, h.caster, BUFF).is_none());
    assert!((speed(&h, h.caster) - 1.3).abs() < 1e-6);
}

#[test]
#[ignore = "native Spirit Wolf no-form control"]
fn native_spirit_wolf_talent_without_form_has_no_children() {
    let mut h = fixture(true);
    h.advance(120);
    assert!(aura(&h, h.caster, PERIODIC).is_none());
    assert!(aura(&h, h.caster, BUFF).is_none());
    assert!((speed(&h, h.caster) - 1.0).abs() < 1e-6);
}

#[test]
#[ignore = "behavioral RED before Ghost Wolf removal cleans native children"]
fn native_spirit_wolf_form_removal_cleans_period_and_stacks() {
    let mut h = fixture(true);
    cast_ghost(&mut h);
    h.advance(90);
    assert_chain(&h, h.caster, 4);
    cancel_ghost(&mut h);
    h.advance(120);
    assert!(aura(&h, h.caster, TALENT).is_some());
    assert!(aura(&h, h.caster, PERIODIC).is_none());
    assert!(aura(&h, h.caster, BUFF).is_none());
    assert!((speed(&h, h.caster) - 1.0).abs() < 1e-6);
}

#[test]
#[ignore = "behavioral RED before Spirit Wolf talent removal cleans native children"]
fn native_spirit_wolf_talent_removal_cleans_children_but_keeps_form() {
    let mut h = fixture(true);
    cast_ghost(&mut h);
    h.advance(90);
    assert_chain(&h, h.caster, 4);
    set_talent(&mut h, false);
    h.advance(120);
    assert!(aura(&h, h.caster, GHOST_WOLF).is_some());
    assert!(aura(&h, h.caster, PERIODIC).is_none());
    assert!(aura(&h, h.caster, BUFF).is_none());
    assert!((speed(&h, h.caster) - 1.3).abs() < 1e-6);
}

#[test]
#[ignore = "behavioral RED before Spirit Wolf re-entry starts fresh stacks"]
fn native_spirit_wolf_form_reentry_starts_again_at_one_stack() {
    let mut h = fixture(true);
    cast_ghost(&mut h);
    h.advance(90);
    assert_chain(&h, h.caster, 4);
    cancel_ghost(&mut h);
    cast_ghost(&mut h);
    assert!(aura(&h, h.caster, BUFF).is_none());
    h.advance(22);
    assert_chain(&h, h.caster, 1);
}

#[test]
#[ignore = "behavioral RED before Spirit Wolf two-owner lifecycle isolation"]
fn native_spirit_wolf_two_owners_form_removal_keeps_other_stack_chain() {
    let mut h = fixture(true);
    cast_ghost(&mut h);
    h.advance(90);
    let first = (h.caster, h.connection);
    assert_chain(&h, first.0, 4);
    let second = crate::spell_cast::test_harness::spawn_player(&mut h.app, 7, 1.0);
    h.app.world_mut().entity_mut(second).insert((
        crate::class_progression::ActiveSpec(262),
        crate::class_progression::KnownSpells::fixture(vec![
            GHOST_WOLF,
            TALENT,
            PERIODIC,
            BUFF,
            NATIVE_DAMAGE,
        ]),
    ));
    let second_connection =
        crate::spell_cast::test_harness::spawn_peer_connection(&mut h.app, second, 2);
    h.advance(3);
    h.caster = second;
    h.connection = second_connection;
    cast_ghost(&mut h);
    h.advance(90);
    assert_chain(&h, second, 4);
    h.caster = first.0;
    h.connection = first.1;
    cancel_ghost(&mut h);
    h.advance(120);
    assert!(aura(&h, first.0, PERIODIC).is_none());
    assert!(aura(&h, first.0, BUFF).is_none());
    assert_chain(&h, second, 4);
}
