//! Concrete native max-level selection and Voltaic Blaze boundaries, not full APL parity.
use super::kit_tests::{at_level, uses_spell};
use crate::auras::{ActiveAuras, aura_now};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{Health, ThreatTable};
use shared::protocol::CombatLogKind;

fn prepare(entry: u32, count: usize) -> (Harness, Entity, Vec<Entity>) {
    prepare_at_level(entry, count, 90)
}

fn prepare_at_level(entry: u32, count: usize, level: u8) -> (Harness, Entity, Vec<Entity>) {
    let (mut h, caster) = at_level(entry, level, |_| {});
    let mut targets = vec![h.npc];
    targets.extend((1..count).map(|index| spawn_npc(&mut h.app, index as f32)));
    for &target in &targets {
        let mut threat = ThreatTable::default();
        threat.add_threat(h.caster.to_bits(), 1.0);
        h.app.world_mut().entity_mut(target).insert((
            Health {
                current: 1_000_000_000.0,
                max: 1_000_000_000.0,
            },
            threat,
        ));
    }
    (h, caster, targets)
}

#[test]
fn rotation_selection_63_native_book_reaches_fireball() {
    let (mut h, caster, _) = prepare(209059, 1);
    uses_spell(&mut h, caster, 133);
}

#[test]
fn rotation_selection_262_native_book_applies_flame_shock() {
    let (mut h, caster, _) = prepare(209053, 1);
    uses_spell(&mut h, caster, 470057);
    h.advance(20);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(h.npc)
            .is_some_and(|auras| auras
                .iter()
                .any(|aura| aura.spell_id == 188389 && aura.caster == caster))
    );
}

fn kybel_actions(level: u8) -> (Harness, Entity, Vec<u32>) {
    let (mut h, caster, _) = prepare_at_level(215240, 3, level);
    for _ in 0..600 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
    }
    let actions: Vec<_> = h
        .inbox
        .go
        .iter()
        .filter(|go| go.caster == caster.to_bits())
        .map(|go| go.spell_id)
        .collect();
    (h, caster, actions)
}

#[test]
fn rotation_selection_1467_three_targets_use_empower_and_spender() {
    let (h, caster, actions) = kybel_actions(90);
    assert!(crate::auras::has_aura(h.app.world(), caster, 411212));
    // Native411212 effects1077606/1077607 replace both empower roots.
    assert!(
        actions.contains(&382266) && actions.contains(&382411),
        "learned Font of Magic must cast native empowered variants: {actions:?}"
    );
    assert!(!actions.contains(&357208) && !actions.contains(&359073));
    assert!(
        actions.iter().any(|spell| matches!(spell, 356995 | 357211)),
        "Azure filler must not starve native Essence spenders: {actions:?}"
    );
}

#[test]
fn rotation_selection_1467_three_targets_before_font_use_native_roots() {
    let (h, caster, actions) = kybel_actions(56);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 411212));
    assert!(
        actions.contains(&357208) && actions.contains(&359073),
        "before native Font of Magic57, both base empower roots must cast: {actions:?}"
    );
    assert!(!actions.contains(&382266) && !actions.contains(&382411));
}

#[test]
fn rotation_selection_262_native_ascendance_after_stormkeeper() {
    let (mut h, caster, _) = prepare(209053, 3);
    let known = h
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(caster)
        .unwrap();
    assert!(
        known.contains(114050) && known.contains(191634),
        "fixture must retain the actual selected native Ascendance talent and Stormkeeper"
    );
    for _ in 0..200 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
        if crate::auras::has_aura(h.app.world(), caster, 1219480) {
            break;
        }
    }
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.caster == caster.to_bits() && go.spell_id == 191634),
        "ordinary native Stormkeeper must precede the Ascendance decision"
    );
    let world = h.app.world();
    let stormkeeper = world
        .resource::<crate::spell_info::SpellStore>()
        .get(191634)
        .unwrap();
    assert!(
        world
            .get::<crate::spell_cast::SpellHistory>(caster)
            .unwrap()
            .on_cooldown(stormkeeper, crate::spell_cast::history_now(world) + 15.0),
        "actual Stormkeeper cooldown must satisfy the sourced greater-than15s branch"
    );
    assert!(
        crate::auras::has_aura(world, caster, 1219480),
        "native known114050 must expose actual1219480 Ascendance, not remain an unused utility parent"
    );
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.caster == caster.to_bits() && go.spell_id == 1219480)
    );
}

#[test]
fn rotation_selection_262_native_ascendance_entry_hits_six_enemies() {
    let (mut h, caster, targets) = prepare(209053, 7);
    for &target in &targets {
        crate::combat_refs::set_in_combat_with(h.app.world_mut(), caster, target);
    }
    assert!(targets.iter().all(|&target| {
        h.app
            .world()
            .get::<shared::components::CombatStatus>(target)
            .is_some_and(|status| status.0)
    }));
    let idle = spawn_npc(&mut h.app, 0.25);
    let distant = spawn_npc(&mut h.app, 80.0);
    crate::combat_refs::set_in_combat_with(h.app.world_mut(), caster, distant);
    assert!(
        targets
            .iter()
            .all(|&target| !crate::auras::has_aura(h.app.world(), target, 188389,))
    );
    let first = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 1219480, Some(caster), false)
        .unwrap();
    h.advance(20);
    assert!(crate::auras::has_aura(h.app.world(), caster, 1219480));
    let shocked: Vec<_> = targets
        .iter()
        .copied()
        .filter(|&target| {
            h.app
                .world()
                .get::<ActiveAuras>(target)
                .is_some_and(|auras| {
                    auras
                        .iter()
                        .any(|aura| aura.spell_id == 188389 && aura.caster == caster)
                })
        })
        .collect();
    assert_eq!(
        shocked.len(),
        6,
        "native1219480s7 must apply six actual owned Flame Shocks"
    );
    let burst_hits: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(285452)
                && event.source == Some(caster.to_bits())
        })
        .collect();
    assert_eq!(
        burst_hits.len(),
        6,
        "entry must launch real native Lava Burst damage once per selected enemy"
    );
    assert!(shocked.iter().all(|target| {
        burst_hits
            .iter()
            .any(|hit| hit.target == Some(target.to_bits()))
    }));
    for excluded in [idle, distant] {
        assert!(!crate::auras::has_aura(h.app.world(), excluded, 188389));
        assert!(
            burst_hits
                .iter()
                .all(|hit| hit.target != Some(excluded.to_bits()))
        );
    }
}

#[test]
fn rotation_selection_262_native_ascendance_entry_uses_half_lava_burst_damage() {
    let (mut h, caster, _) = prepare(209053, 1);
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 188389, Some(h.npc), 1);
    h.advance(1);
    let first = h.inbox.log.len();
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 285452, Some(h.npc), 1);
    h.advance(1);
    let full = h.inbox.log[first..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(285452)
                && event.source == Some(caster.to_bits())
                && event.target == Some(h.npc.to_bits())
        })
        .expect("native ordinary Lava Burst control must actually damage its Flame-Shocked target")
        .clone();
    assert!(full.crit && full.amount > 0);
    let first = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 1219480, Some(caster), false)
        .unwrap();
    h.advance(20);
    assert!(crate::auras::has_aura(h.app.world(), caster, 1219480));
    let half = h.inbox.log[first..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(285452)
                && event.source == Some(caster.to_bits())
                && event.target == Some(h.npc.to_bits())
        })
        .expect("native Ascendance must launch actual Lava Burst damage");
    assert!(
        half.crit,
        "entry Flame Shock precedes the Lava Burst crit decision"
    );
    let damage = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(285452)
        .unwrap();
    let variance = f64::from(damage.effects[0].variance) / 2.0;
    // Independent real variance draws, not paired RNG: both native rounding bounds count.
    let minimum = ((f64::from(full.amount) - 0.5) * 0.5 * (1.0 - variance) / (1.0 + variance) - 0.5)
        .floor() as i32;
    let maximum = ((f64::from(full.amount) + 0.5) * 0.5 * (1.0 + variance) / (1.0 - variance) + 0.5)
        .ceil() as i32;
    assert!(
        (minimum..=maximum).contains(&half.amount),
        "native1219480s10=50%: full={}, half={}, expected={minimum}..={maximum}",
        full.amount,
        half.amount
    );
    let first = h.inbox.log.len();
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 285452, Some(h.npc), 1);
    h.advance(1);
    let ordinary = h.inbox.log[first..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(285452)
                && event.source == Some(caster.to_bits())
                && event.target == Some(h.npc.to_bits())
        })
        .expect("ordinary native Lava Burst still damages during Ascendance");
    assert!(ordinary.crit);
    let minimum =
        ((f64::from(full.amount) - 0.5) * (1.0 - variance) / (1.0 + variance) - 0.5).floor() as i32;
    let maximum =
        ((f64::from(full.amount) + 0.5) * (1.0 + variance) / (1.0 - variance) + 0.5).ceil() as i32;
    assert!(
        (minimum..=maximum).contains(&ordinary.amount),
        "only the entry cast is half-effectiveness; ordinary damage={}",
        ordinary.amount
    );
}

fn stormkeeper_within_sync_window(h: &Harness, caster: Entity) -> bool {
    let world = h.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(191634)
        .unwrap();
    let history = world
        .get::<crate::spell_cast::SpellHistory>(caster)
        .unwrap();
    let now = crate::spell_cast::history_now(world);
    history.on_cooldown(spell, now) && !history.on_cooldown(spell, now + 15.0)
}

#[test]
fn rotation_selection_262_native_ascendance_unavailable_before_talent() {
    let (mut h, caster, _) = prepare_at_level(209053, 3, 50);
    assert!(
        !h.app
            .world()
            .get::<crate::class_progression::KnownSpells>(caster)
            .unwrap()
            .contains(114050)
    );
    for _ in 0..200 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.caster == caster.to_bits() && go.spell_id == 191634)
    );
    assert!(!crate::auras::has_aura(h.app.world(), caster, 1219480));
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == caster.to_bits() && go.spell_id == 1219480)
    );
}

#[test]
fn rotation_selection_262_native_ascendance_held_in_stormkeeper_sync_window() {
    let (mut h, caster, _) = prepare(209053, 3);
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 191634, Some(caster), false)
        .unwrap();
    h.advance(60);
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.caster == caster.to_bits() && go.spell_id == 191634)
    );
    // Advance the actual native clock, never reset cooldowns or invent a timer.
    for _ in 0..1600 {
        if stormkeeper_within_sync_window(&h, caster) {
            break;
        }
        h.advance(1);
    }
    assert!(stormkeeper_within_sync_window(&h, caster));
    for _ in 0..120 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        stormkeeper_within_sync_window(&h, caster),
        "control must remain inside the real synchronization window, without earned resets"
    );
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.caster == caster.to_bits() && go.spell_id == 470057),
        "ordinary Voltaic Blaze must execute while the major cooldown is held"
    );
    assert!(!crate::auras::has_aura(h.app.world(), caster, 1219480));
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == caster.to_bits() && go.spell_id == 1219480),
        "Ascendance must wait when native Stormkeeper cooldown is at most15s"
    );
}

#[test]
fn rotation_selection_262_voltaic_blaze_damage_precedes_delayed_flame_shock() {
    let (mut h, caster, targets) = prepare(209053, 3);
    let begin = aura_now(h.app.world());
    let first = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 470057, Some(h.npc), false)
        .unwrap();
    h.advance(1);
    let hits: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1259101)
                && event.source == Some(caster.to_bits())
        })
        .cloned()
        .collect();
    assert!(
        !hits.is_empty(),
        "native Dummy root must cast its actual1259101 damage child"
    );
    assert!(hits.iter().all(|event| event.crit));
    h.advance(8);
    assert!(aura_now(h.app.world()) - begin < 0.5);
    assert!(
        targets
            .iter()
            .all(|target| !crate::auras::has_aura(h.app.world(), *target, 188389)),
        "TC's500ms Flame Shock callback must not execute early"
    );
    h.advance(2);
    for hit in hits {
        let target = Entity::try_from_bits(hit.target.unwrap()).unwrap();
        assert!(
            h.app
                .world()
                .get::<ActiveAuras>(target)
                .is_some_and(|auras| auras
                    .iter()
                    .any(|aura| aura.spell_id == 188389 && aura.caster == caster)),
            "actual Voltaic hit must receive its delayed native Flame Shock"
        );
    }
}
