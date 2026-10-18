//! Native Prayer of Mending admission; TC a352b1fa spell_priest.cpp3636–3806.
use super::test_harness::{Harness, harness, spawn_player, varied};
use crate::auras::has_aura;
use bevy::prelude::Entity;
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

const PRAYER_OF_MENDING: u32 = 33_076;
const MENDING_WARD: u32 = 41_635;
const MENDING_HEAL: u32 = 33_110;
const MENDING_JUMP: u32 = 155_793;
/// Native800 Fire, no coefficient or variance; leaves the existing1000HP player at200.
const ANNIHILATION: u32 = 209_140;

fn fixture() -> Harness {
    harness(
        Vec::new(),
        &[
            PRAYER_OF_MENDING,
            MENDING_WARD,
            MENDING_HEAL,
            MENDING_JUMP,
            ANNIHILATION,
        ],
        5,
    )
    .unwrap()
}

fn ordinary_damage(h: &mut Harness) {
    let target = h.caster;
    ordinary_damage_to(h, target);
}

fn ordinary_damage_to(h: &mut Harness, target: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, ANNIHILATION, Some(target), false)
        .unwrap();
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    let frames = h.app.world().get::<CastState>(h.npc).map_or(0, |cast| {
        ((cast.duration - cast.elapsed).max(0.0) / dt).ceil() as usize
    });
    h.advance(frames + 3);
}

#[test]
fn prayer_of_mending_native_unprotected_damage_does_not_heal_or_jump() {
    let mut h = fixture();
    ordinary_damage(&mut h);
    assert_eq!(h.health(h.caster), 200.0);
    assert!(!has_aura(h.app.world(), h.caster, MENDING_WARD));
    assert!(!h.inbox.log.iter().any(|event| {
        event.spell_id == Some(MENDING_HEAL) && event.kind == CombatLogKind::Heal
    }));
}

#[test]
fn prayer_of_mending_native_normal_player_cast_applies_real_ward() {
    let mut h = fixture();
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(h.caster));
    h.advance(3);
    assert!(
        has_aura(h.app.world(), h.caster, MENDING_WARD),
        "ordinary33076 must create real41635; cast failures: {:?}",
        h.inbox.failed
    );
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let ward = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == MENDING_WARD)
        .unwrap();
    // Source initial native jump count4 plus the initial recipient.
    assert_eq!(ward.stacks, 5);
    assert_eq!(ward.caster, h.caster);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::AuraApplied
                && event.spell_id == Some(MENDING_WARD)
                && event.amount == 5)
    );
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::AuraApplied
            && event.spell_id == Some(MENDING_WARD)
            && event.source == Some(h.caster.to_bits())
            && event.target == Some(h.caster.to_bits())
    }));
}

#[test]
fn prayer_of_mending_native_damage_heals_then_delayed_jump_keeps_original_priest() {
    let mut h = fixture();
    let holder = spawn_player(&mut h.app, 1, 12.0);
    let healthy_decoy = spawn_player(&mut h.app, 1, 11.0);
    // Real native injury makes the original Priest the unique injured candidate.
    ordinary_damage(&mut h);
    assert_eq!(h.health(h.caster), 200.0);
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(holder));
    h.advance(3);
    assert!(has_aura(h.app.world(), holder, MENDING_WARD));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    ordinary_damage_to(&mut h, holder);
    let heal = h
        .inbox
        .log
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(MENDING_HEAL)
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(holder.to_bits())
        })
        .expect("damage must execute real33110, attributed to the original Priest");
    // Native SP coefficient0.732 and unchanged fixture healing power100.
    let base = if heal.crit { 146.4 } else { 73.2 };
    assert!(varied(base).contains(&(heal.amount as f32)), "{heal:?}");
    assert_eq!(h.health(holder), 200.0 + heal.amount as f32);
    assert!(!has_aura(h.app.world(), holder, MENDING_WARD));
    assert!(
        !has_aura(h.app.world(), h.caster, MENDING_WARD),
        "the native40yd/s jump must not apply before flight"
    );
    assert!(!has_aura(h.app.world(), healthy_decoy, MENDING_WARD));
    let jump = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(MENDING_JUMP)
        .unwrap();
    let speed = jump.misc.as_ref().unwrap().speed;
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    h.advance((12.0 / speed / dt).ceil() as usize + 3);
    let transferred = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == MENDING_WARD)
        .expect("real155793 must transfer the ward after its native flight");
    assert_eq!(transferred.caster, h.caster);
    assert_eq!(transferred.stacks, 4);
    assert!(!has_aura(h.app.world(), healthy_decoy, MENDING_WARD));
}

#[test]
fn prayer_of_mending_native_no_stack_override_preserves_agony_floor_and_refresh() {
    for talent in [false, true] {
        let ids = if talent {
            vec![980, 1_260_209]
        } else {
            vec![980]
        };
        let mut h = harness(Vec::new(), &ids, 9).unwrap();
        if talent {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.caster,
                    target: h.caster,
                    spell_id: 1_260_209,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        h.cast_and_deliver(980, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 980)
            .unwrap();
        let initial = if talent { 3 } else { 1 };
        assert_eq!(aura.stacks, initial);
        let period = aura
            .effects
            .iter()
            .find(|effect| effect.aura_type == 3)
            .unwrap()
            .period_ms;
        let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
        h.advance((period as f32 / 1000.0 / dt).ceil() as usize);
        let stacks = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 980)
            .unwrap()
            .stacks;
        assert_eq!(stacks, initial + 1);
        h.cast_and_deliver(980, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(h.npc)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 980)
                .unwrap()
                .stacks,
            stacks
        );
    }
}
