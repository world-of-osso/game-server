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
