//! TC a352b1fa Spell.cpp4019/4303/4409; native70448 stages, no invented ranks.
use super::kit_tests::at_level;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::casting::{CastState, CastType};
use shared::components::Health;
use shared::protocol::CombatLogKind;

const KYBEL: u32 = 215240;
const FIRE_BREATH: u32 = 357208;
const FIRE_DAMAGE: u32 = 357209;
const ETERNITY_SURGE: u32 = 359073;
const SURGE_DAMAGE: u32 = 359077;

fn prepare(level: u8) -> (Harness, Entity) {
    let (mut h, caster) = at_level(KYBEL, level, |_| {});
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    (h, caster)
}

#[test]
fn native_empower_clock_holds_without_damage_and_defers_gcd() {
    let (mut h, caster) = prepare(21);
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIRE_BREATH, Some(caster), false)
        .unwrap();
    let cast = h.app.world().get::<CastState>(caster).unwrap();
    assert_eq!(cast.cast_type, CastType::Channel);
    // Three native stages1000/750/750ms plus TC Spell.h88's1000ms max hold.
    assert_eq!(cast.duration, 3.5);
    let info = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(FIRE_BREATH)
        .unwrap();
    assert!(
        !h.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(caster)
            .unwrap()
            .on_gcd(info, crate::spell_cast::history_now(h.app.world())),
        "TC defers the root GCD until empower release"
    );
    h.advance(69);
    assert!(h.app.world().get::<CastState>(caster).is_some());
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.spell_id == Some(FIRE_DAMAGE)
                && event.kind == CombatLogKind::Damage)
    );
    h.advance(1);
    assert!(h.app.world().get::<CastState>(caster).is_none());
}

#[test]
fn native_empower_fire_breath_max_release_damages_and_keeps_rank_three_dot() {
    let (mut h, caster) = prepare(21);
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIRE_BREATH, Some(caster), false)
        .unwrap();
    h.advance(90);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.spell_id == Some(FIRE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "max release must execute actual native357209 damage"
    );
    let dot = h
        .app
        .world()
        .get::<ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FIRE_DAMAGE && aura.caster == caster)
        .unwrap();
    // TC10 ticks minus3 per extra rank, native2s period; no Blast Furnace at21.
    assert_eq!(dot.duration_ms, 8_000);
}

fn spawn_surge_fixture(level: u8, enemies: usize) -> (Harness, Entity) {
    let (mut h, caster) = prepare(level);
    assert_eq!(
        crate::auras::has_aura(h.app.world(), caster, 375757),
        level >= 31,
        "fixture must retain the native level-gated Eternity's Span passive"
    );
    for index in 1..enemies {
        let enemy = spawn_npc(&mut h.app, 3.0 + index as f32);
        h.app.world_mut().entity_mut(enemy).insert(Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        });
    }
    (h, caster)
}

fn observe_surge_targets(level: u8, enemies: usize) -> std::collections::BTreeSet<u64> {
    let (mut h, caster) = spawn_surge_fixture(level, enemies);
    let target = h.npc;
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        caster,
        ETERNITY_SURGE,
        Some(target),
        false,
    )
    .unwrap();
    h.advance(100);
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.spell_id == Some(SURGE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .filter_map(|event| event.target)
        .collect()
}

#[test]
fn native_empower_eternity_surge_rank_three_hits_three_of_four_enemies() {
    assert_eq!(
        observe_surge_targets(21, 4).len(),
        3,
        "native rank3 limits359077 to3 targets without Eternity's Span"
    );
}

#[test]
fn native_empower_eternity_span_rank_three_hits_six_of_seven_enemies() {
    assert_eq!(
        observe_surge_targets(32, 7).len(),
        6,
        "native375757 doubles actual rank3 recipients without fabricating damage"
    );
}

fn spawn_scouring_fixture(scouring: bool) -> (Harness, Entity) {
    let (mut h, caster) = prepare(21);
    let target = h.npc;
    crate::spell_cast::creature_cast(h.app.world_mut(), target, 21562, Some(target), true).unwrap();
    assert!(
        crate::auras::has_aura(h.app.world(), target, 21562),
        "native Magic Fortitude must exist before the release"
    );
    // Controlled native passive application isolates TC's real Scouring gate;
    // this is not a claim that the default level21 starter loadout selected it.
    if scouring {
        crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 378438, Some(caster), 1);
    }
    assert_eq!(
        crate::auras::has_aura(h.app.world(), caster, 378438),
        scouring
    );
    (h, caster)
}

fn cast_fire_breath_and_check_magic_buff(scouring: bool) -> bool {
    let (mut h, caster) = spawn_scouring_fixture(scouring);
    let target = h.npc;
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIRE_BREATH, Some(caster), false)
        .unwrap();
    h.advance(90);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.spell_id == Some(FIRE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    crate::auras::has_aura(h.app.world(), target, 21562)
}

#[test]
fn native_empower_scouring_absent_preserves_enemy_magic_buff() {
    assert!(cast_fire_breath_and_check_magic_buff(false));
}

#[test]
fn native_empower_scouring_present_dispels_enemy_magic_buff() {
    assert!(!cast_fire_breath_and_check_magic_buff(true));
}

#[test]
fn native_empower_cancel_resets_surge_cooldown_without_release_damage() {
    let (mut h, caster) = prepare(21);
    let target = h.npc;
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        caster,
        ETERNITY_SURGE,
        Some(target),
        false,
    )
    .unwrap();
    h.advance(10);
    assert!(h.app.world().get::<CastState>(caster).is_some());
    crate::spell_cast::cancel_cast(h.app.world_mut(), caster);
    assert!(h.app.world().get::<CastState>(caster).is_none());
    let world = h.app.world();
    let store = world.resource::<crate::spell_info::SpellStore>();
    let history = world
        .get::<crate::spell_cast::SpellHistory>(caster)
        .unwrap();
    assert!(
        !history.on_cooldown(
            store.get(ETERNITY_SURGE).unwrap(),
            crate::spell_cast::history_now(world)
        ),
        "TC Spell::finish resets the cancelled empower's ordinary spell cooldown"
    );
    assert!(
        history.on_gcd(
            store.get(359115).unwrap(),
            crate::spell_cast::history_now(world)
        ),
        "cancel still applies native359115 finish GCD"
    );
    h.advance(100);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.spell_id == Some(SURGE_DAMAGE)
                && event.kind == CombatLogKind::Damage),
        "cancel must never be reinterpreted as empower release"
    );
}

#[test]
fn native_empower_follower_single_target_releases_fire_breath_at_rank_one() {
    let (mut h, caster) = prepare(21);
    let mut begun = false;
    for _ in 0..600 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
        if h.inbox
            .go
            .iter()
            .any(|event| event.caster == caster.to_bits() && event.spell_id == FIRE_BREATH)
        {
            begun = true;
            break;
        }
    }
    assert!(begun, "real follower must first admit native Fire Breath");
    for _ in 0..21 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
    }
    let dot = h.app.world().get::<ActiveAuras>(h.npc).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == FIRE_DAMAGE && aura.caster == caster)
    });
    assert_eq!(
        dot.map(|aura| aura.duration_ms),
        Some(20_000),
        "SimC fb_fs single-target rank1 releases after the first native stage"
    );
}

#[test]
fn native_empower_tip_scales_snapshots_max_rank_before_consuming_one_use() {
    let (mut h, caster) = prepare(90);
    let tip = 370553;
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, tip, Some(caster), false).unwrap();
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), caster, tip));
    let fire = super::kit_overrides::resolve_action(h.app.world(), caster, FIRE_BREATH).unwrap();
    crate::spell_cast::creature_cast_empowered(h.app.world_mut(), caster, fire, Some(caster), 1)
        .unwrap();
    assert!(
        !crate::auras::has_aura(h.app.world(), caster, tip),
        "native one-use duration modifier must be consumed by the admitted empower"
    );
    h.advance(3);
    assert!(
        h.app.world().get::<CastState>(caster).is_none(),
        "native duration reduction must be snapshotted before consuming Tip"
    );
    let dot = h
        .app
        .world()
        .get::<ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FIRE_DAMAGE && aura.caster == caster)
        .unwrap();
    assert_eq!(
        dot.duration_ms, 6_000,
        "Tip must release Font rank4 despite a stage1 request: native base2s plus Blast Furnace4s"
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.spell_id == Some(FIRE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );

    // Let the ordinary finish GCD expire, without restoring any cooldown or aura.
    h.advance(12);
    let surge =
        super::kit_overrides::resolve_action(h.app.world(), caster, ETERNITY_SURGE).unwrap();
    crate::spell_cast::creature_cast_empowered(h.app.world_mut(), caster, surge, Some(h.npc), 1)
        .unwrap();
    h.advance(3);
    assert!(
        h.app.world().get::<CastState>(caster).is_some(),
        "consumed Tip must not make the next empower instant"
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.spell_id == Some(SURGE_DAMAGE)
                && event.kind == CombatLogKind::Damage)
    );
    h.advance(20);
    assert!(h.app.world().get::<CastState>(caster).is_none());
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.spell_id == Some(SURGE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

fn complete_empower(h: &mut Harness, caster: Entity, base: u32) {
    let spell = super::kit_overrides::resolve_action(h.app.world(), caster, base).unwrap();
    let target = if base == FIRE_BREATH { caster } else { h.npc };
    super::kit_proc_fidelity_tests::complete_cast(h, caster, spell, target);
    for _ in 0..100 {
        h.advance(1);
        if h.app.world().get::<CastState>(caster).is_none() {
            return;
        }
    }
    panic!("native empower{spell} did not finish");
}

#[test]
fn native_empower_animosity_extends_dragonrage_with_native_diminishing_effectiveness() {
    let (mut h, caster) = prepare(90);
    let animosity = h
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 375797)
        .unwrap();
    let extension_ms = animosity
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let reduction = animosity
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    super::kit_proc_fidelity_tests::complete_cast(&mut h, caster, 375087, caster);
    let mut effectiveness = 1.0;
    for base in [FIRE_BREATH, ETERNITY_SURGE] {
        let before = super::kit_clearcasting_tests::own_aura(&h, caster, 375087).unwrap();
        complete_empower(&mut h, caster, base);
        let after = super::kit_clearcasting_tests::own_aura(&h, caster, 375087).unwrap();
        let extended_ms = (after.expires_at.unwrap() - before.expires_at.unwrap()) * 1000.0;
        let expected = extension_ms * effectiveness;
        assert!(
            (extended_ms - f64::from(expected)).abs() <= 1.0,
            "actual empower must extend the same Dragonrage by native{expected}ms, got{extended_ms}"
        );
        assert_eq!(before.instance_id, after.instance_id);
        effectiveness *= 1.0 - reduction / 100.0;
    }
}

#[test]
fn native_empower_animosity_new_dragonrage_instance_restores_full_effectiveness() {
    let (mut h, caster) = prepare(90);
    super::kit_proc_fidelity_tests::complete_cast(&mut h, caster, 375087, caster);
    let first = super::kit_clearcasting_tests::own_aura(&h, caster, 375087).unwrap();
    complete_empower(&mut h, caster, FIRE_BREATH);
    // Let the actual first Dragonrage expire and the native Fire Breath charge recover.
    h.advance(600);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 375087));
    // Controlled native buff application isolates per-instance script state;
    // this does not claim the ordinary Dragonrage cooldown is already ready.
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 375087, Some(caster), 1);
    let before = super::kit_clearcasting_tests::own_aura(&h, caster, 375087).unwrap();
    assert_ne!(before.instance_id, first.instance_id);
    let passive = super::kit_clearcasting_tests::own_aura(&h, caster, 375797).unwrap();
    let expected = passive
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    complete_empower(&mut h, caster, FIRE_BREATH);
    let after = super::kit_clearcasting_tests::own_aura(&h, caster, 375087).unwrap();
    let extended = (after.expires_at.unwrap() - before.expires_at.unwrap()) * 1000.0;
    assert!(
        (extended - f64::from(expected)).abs() <= 1.0,
        "new native Dragonrage instance must receive full Animosity again"
    );
}

#[test]
fn native_empower_blast_furnace_preserves_extra_dot_seconds_at_ranks_one_and_three() {
    for level in [43, 44] {
        for (stage, base_ms) in [(1, 20_000), (3, 8_000)] {
            let (mut h, caster) = prepare(level);
            let furnace = super::kit_clearcasting_tests::own_aura(&h, caster, 375510);
            assert_eq!(
                furnace.is_some(),
                level == 44,
                "fixture must use the actual native Blast Furnace learn boundary"
            );
            let extra_seconds = furnace.as_ref().map_or(0.0, |aura| {
                aura.effects
                    .iter()
                    .find(|effect| effect.index == 0)
                    .unwrap()
                    .amount
            });
            crate::spell_cast::creature_cast_empowered(
                h.app.world_mut(),
                caster,
                FIRE_BREATH,
                Some(caster),
                stage,
            )
            .unwrap();
            h.advance(100);
            let dot = h
                .app
                .world()
                .get::<ActiveAuras>(h.npc)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == FIRE_DAMAGE && aura.caster == caster)
                .unwrap();
            assert_eq!(
                dot.duration_ms,
                base_ms + (extra_seconds * 1000.0) as u32,
                "native Blast Furnace must extend the remaining rank{stage} DoT without changing its tick clock"
            );
        }
    }
}

fn observe_fire_breath_damage(stage: u8) -> (f64, f64, f64, f32) {
    let (mut h, caster) = prepare(21);
    let world = h.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(FIRE_DAMAGE)
        .unwrap();
    let variance = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .variance
        / 2.0;
    let spell_power = world
        .get::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .spell_power;
    assert_eq!(
        world
            .get::<crate::auras::AuraStatMods>(caster)
            .unwrap()
            .critical_damage(spell.school_mask()),
        1.0,
        "low-level native fixture must retain the ordinary200% critical-hit multiplier"
    );
    crate::spell_cast::creature_cast_empowered(
        h.app.world_mut(),
        caster,
        FIRE_BREATH,
        Some(caster),
        stage,
    )
    .unwrap();
    h.advance(120);
    let hits: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.spell_id == Some(FIRE_DAMAGE)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    let direct = hits.iter().find(|event| !event.periodic).unwrap();
    let tick = hits.iter().find(|event| event.periodic).unwrap();
    let normalize = |event: &shared::protocol::CombatLogEvent| {
        f64::from(event.amount) / if event.crit { 2.0 } else { 1.0 }
    };
    (
        normalize(direct),
        normalize(tick),
        f64::from(variance),
        spell_power,
    )
}

#[test]
fn native_empower_higher_fire_ranks_convert_three_periodic_ticks_per_extra_stage() {
    let (base_direct, tick, variance, power) = observe_fire_breath_damage(1);
    for rank in [2, 3] {
        let (direct, same_tick, same_variance, same_power) = observe_fire_breath_damage(rank);
        assert_eq!(same_power, power);
        assert_eq!(same_variance, variance);
        assert!(
            (same_tick - tick).abs() <= 1.0,
            "upranking must not change periodic tick magnitude"
        );
        let converted = f64::from((rank - 1) * 3);
        // Independent real tick oracle; native direct variance affects only its base hit.
        // Each integer combat receipt contributes at most one point of rounding error.
        let low =
            (base_direct - 1.0) * (1.0 - variance) / (1.0 + variance) + converted * (tick - 1.0);
        let high =
            (base_direct + 1.0) * (1.0 + variance) / (1.0 - variance) + converted * (tick + 1.0);
        assert!(
            (low..=high).contains(&direct),
            "rank{rank} actual{direct} must include{converted} native ticks in{low}..{high}"
        );
    }
}

#[test]
fn native_empower_font_rank_four_releases_real_short_dot_and_eight_target_surge() {
    for base in [FIRE_BREATH, ETERNITY_SURGE] {
        let (mut h, caster) = spawn_surge_fixture(65, 9);
        let spell = super::kit_overrides::resolve_action(h.app.world(), caster, base).unwrap();
        let target = if base == FIRE_BREATH { caster } else { h.npc };
        crate::spell_cast::creature_cast_empowered(
            h.app.world_mut(),
            caster,
            spell,
            Some(target),
            4,
        )
        .unwrap();
        h.advance(100);
        if base == FIRE_BREATH {
            let dot = h
                .app
                .world()
                .get::<ActiveAuras>(h.npc)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == FIRE_DAMAGE && aura.caster == caster)
                .unwrap();
            let furnace = super::kit_clearcasting_tests::own_aura(&h, caster, 375510).unwrap();
            let extra = furnace
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount;
            assert_eq!(
                dot.duration_ms,
                2_000 + (extra * 1000.0) as u32,
                "native rank4 leaves one periodic tick plus Blast Furnace"
            );
            assert!(
                h.inbox
                    .log
                    .iter()
                    .any(|event| event.source == Some(caster.to_bits())
                        && event.target == Some(h.npc.to_bits())
                        && event.spell_id == Some(FIRE_DAMAGE)
                        && event.kind == CombatLogKind::Damage
                        && !event.periodic
                        && event.amount > 0)
            );
        } else {
            let hits: std::collections::BTreeSet<_> = h
                .inbox
                .log
                .iter()
                .filter(|event| {
                    event.source == Some(caster.to_bits())
                        && event.spell_id == Some(SURGE_DAMAGE)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .filter_map(|event| event.target)
                .collect();
            assert_eq!(
                hits.len(),
                8,
                "native Font rank4 plus Eternity's Span hits8 of9 enemies"
            );
        }
    }
}

#[test]
fn native_empower_scouring_retains_native_two_dispels_at_rank_three() {
    let (mut h, caster) = prepare(21);
    let target = h.npc;
    let buffs = [1459, 21562, 1044]; // Three actual helpful Magic auras, no absorbs.
    for spell in buffs {
        crate::spell_cast::creature_cast(h.app.world_mut(), target, spell, Some(target), true)
            .unwrap();
        assert!(crate::auras::has_aura(h.app.world(), target, spell));
    }
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 378438, Some(caster), 1);
    let count = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(FIRE_DAMAGE)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 5 && effect.effect == 38)
        .unwrap()
        .base_points;
    assert_eq!(
        count, 2.0,
        "native70448 count differs from historical rank-driven TC script"
    );
    crate::spell_cast::creature_cast_empowered(
        h.app.world_mut(),
        caster,
        FIRE_BREATH,
        Some(caster),
        3,
    )
    .unwrap();
    h.advance(60);
    let remaining = buffs
        .iter()
        .filter(|spell| crate::auras::has_aura(h.app.world(), target, **spell))
        .count();
    assert_eq!(
        remaining,
        buffs.len() - count as usize,
        "actual rank3 Fire Breath must dispel exactly native2, not historicalrank3"
    );
}

#[test]
fn native_empower_font_kit_resolver_dispatches_both_real_four_stage_variants() {
    for (base, variant) in [(FIRE_BREATH, 382266), (ETERNITY_SURGE, 382411)] {
        let (mut h, caster) = prepare(65);
        assert!(crate::auras::has_aura(h.app.world(), caster, 411212));
        let resolved = super::kit_overrides::resolve_action(h.app.world(), caster, base).unwrap();
        assert_eq!(
            resolved, variant,
            "native Font must resolve before cast eligibility"
        );
        let target = if base == FIRE_BREATH { caster } else { h.npc };
        crate::spell_cast::creature_cast(h.app.world_mut(), caster, resolved, Some(target), false)
            .unwrap();
        h.advance(1);
        assert!(
            h.inbox
                .go
                .iter()
                .any(|event| event.caster == caster.to_bits() && event.spell_id == variant),
            "Font must cast actual native four-stage variant{variant}, not base{base}"
        );
        let stages = &h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(variant)
            .unwrap()
            .empower_stages;
        assert_eq!(stages.len(), 4);
        assert_eq!(
            stages
                .iter()
                .map(|stage| stage.duration_ms)
                .collect::<Vec<_>>(),
            vec![1000, 750, 750, 750]
        );
    }
}
