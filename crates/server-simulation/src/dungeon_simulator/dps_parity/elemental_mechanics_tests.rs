//! Native Elemental cast/flight/aura boundaries, not inferred damage coefficients.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const MOTE: u32 = 260734;
const BLAST_BUFFS: &[u32] = &[118522, 173183, 173184];
type DamageSample = (u64, f32, f32);

pub(super) fn fixture() -> Run {
    let mut run = prepare_literal(262, 1).unwrap();
    engage(&mut run);
    run
}

pub(super) fn settle(run: &mut Run) {
    for _ in 0..600 {
        run.app.update();
        let world = run.app.world();
        let busy = world.get::<shared::casting::CastState>(run.actor).is_some()
            || world
                .get::<crate::spell_cast::SpellHistory>(run.actor)
                .is_some_and(|history| {
                    history.has_active_gcd(crate::spell_cast::history_now(world))
                });
        if !busy {
            for _ in 0..10 {
                run.app.update();
            }
            return;
        }
    }
    panic!("native Elemental cast/GCD failed to finish");
}

pub(super) fn cast(run: &mut Run, spell: u32) {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        false,
    )
    .unwrap_or_else(|reason| panic!("native spell{spell} rejected: {reason:?}"));
    settle(run);
}

pub(super) fn own_aura(run: &Run, spell: u32) -> Option<&crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn distant_flame_shock_tick_still_grants_lava_surge() {
    let _seed = crate::random::seed_scope(818181);
    let mut run = fixture();
    assert!(own_aura(&run, 77756).is_some(), "native Lava Surge passive");
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        188389,
        Some(run.targets[0]),
        1,
    ));
    let target = run.targets[0];
    let at = *run
        .app
        .world()
        .get::<shared::components::Position>(target)
        .unwrap();
    run.app
        .world_mut()
        .get_mut::<shared::components::Position>(run.actor)
        .unwrap()
        .x = at.x + 300.0;
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..100 {
        run.app.update();
        let ticked = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(188389)
                    && event.kind == CombatLogKind::Damage
                    && event.periodic
            });
        if ticked {
            let actor_pos = run
                .app
                .world()
                .get::<shared::components::Position>(run.actor)
                .unwrap();
            let target_pos = run
                .app
                .world()
                .get::<shared::components::Position>(target)
                .unwrap();
            assert!((actor_pos.x - target_pos.x).abs() > 100.0);
            assert!(
                own_aura(&run, 77762).is_some(),
                "TC's infinite normalized count guarantees a proc at nonzero chance: actor={actor_pos:?} target={target_pos:?} passive={:?}",
                own_aura(&run, 77756)
            );
            return;
        }
    }
    panic!("the distant Flame Shock did not produce a real periodic tick");
}

fn flame_shock_hits(imbued: bool) -> [i32; 2] {
    let _seed = crate::random::seed_scope(818181);
    let mut run = fixture();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        318038,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    let percent = own_aura(&run, 382028)
        .expect("real main-hand imbue modifier")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(
        (percent - 6.5).abs() < 0.001,
        "native Enhanced Imbues modifies the native 5% amount"
    );
    if !imbued {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            RemoveReason::Cancelled,
            |aura| aura.spell_id == 319778,
        );
        assert!(
            own_aura(&run, 382028).is_none(),
            "modifier retires with its source"
        );
    }
    read_flame_shock_hits(&mut run)
}

fn read_flame_shock_hits(run: &mut Run) -> [i32; 2] {
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.targets[0],
        RemoveReason::Cancelled,
        |aura| aura.spell_id == 188389,
    );
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = -100.0;
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        188389,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..80 {
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .crit_pct = -100.0;
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    [false, true].map(|periodic| {
        let hit = events
            .iter()
            .find(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(188389)
                    && event.kind == CombatLogKind::Damage
                    && event.periodic == periodic
            })
            .expect("actual full Flame Shock hit");
        assert!(!hit.crit, "controlled hit must not crit");
        hit.amount
    })
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_flametongue_imbue_modifies_shock_until_removed() {
    let empowered = flame_shock_hits(true);
    let plain = flame_shock_hits(false);
    for (value, base) in empowered.into_iter().zip(plain) {
        let ratio = f64::from(value) / f64::from(base);
        let expected = 1.0 + 0.05 * 1.3;
        assert!(
            (ratio - expected).abs() < 0.02,
            "imbued {value}, plain {base}, expected {expected}"
        );
    }
}

fn unity_shock_hits(selected: bool, elementals: bool) -> [i32; 2] {
    let _seed = crate::random::seed_scope(818181);
    let mut run = fixture();
    assert!(own_aura(&run, 462866).is_some(), "selected Elemental Unity");
    if !selected {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            RemoveReason::Cancelled,
            |aura| aura.spell_id == 462866,
        );
    }
    if elementals {
        for spell in [118291, 157319] {
            assert!(crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                run.actor,
                spell,
                Some(run.actor),
                1
            ));
        }
        for _ in 0..5 {
            run.app.update();
        }
        assert!(own_aura(&run, 118291).is_some());
        assert!(own_aura(&run, 157319).is_some());
    }
    read_flame_shock_hits(&mut run)
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_unity_periodic_factors_apply_with_or_without_elementals() {
    for active in [false, true] {
        let with_unity = unity_shock_hits(true, active);
        let without_unity = unity_shock_hits(false, active);
        let periodic_ratio = f64::from(with_unity[1]) / f64::from(without_unity[1]);
        let expected = (1.0_f64 + 0.06).powi(2);
        assert!(
            (periodic_ratio - expected).abs() < 0.02,
            "elementals {active}: {periodic_ratio} expected {expected}"
        );
        if !active {
            let direct_ratio = f64::from(with_unity[0]) / f64::from(without_unity[0]);
            assert!(
                (direct_ratio - 1.0).abs() < 0.02,
                "no elemental means no direct bonus: {direct_ratio}"
            );
        }
    }
}

fn remove_mote(run: &mut Run) {
    let instance = own_aura(run, MOTE).expect("earned MOTE").instance_id;
    crate::auras::remove_instance(
        run.app.world_mut(),
        run.actor,
        instance,
        RemoveReason::Consumed,
    );
}

fn prepare_mote(keep_mote: bool) -> (Run, f32) {
    let mut run = fixture();
    cast(&mut run, 51505);
    let mote = own_aura(&run, MOTE).expect("native Lava Burst MOTE");
    let percent = mote
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(percent > 0.0, "native talent modifies MOTE's dummy amount");
    if !keep_mote {
        remove_mote(&mut run);
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    (run, percent)
}

fn damage(run: &Run, spell: u32, periodic: bool) -> u64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell)
                && event.periodic == periodic
        })
        .unwrap_or_else(|| panic!("actual native spell{spell} periodic{periodic} impact"))
        .amount as u64
}

fn variance(run: &Run, spell: u32, index: u8) -> f32 {
    run.app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .variance
}

fn assert_mote_damage(ordinary: DamageSample, empowered: DamageSample) {
    let (ordinary, _, spread) = ordinary;
    let (empowered, percent, _) = empowered;
    let expected = ordinary as f64 * (1.0 + f64::from(percent) / 100.0);
    // Independent native ±Variance/2 rolls; do not tune or disable them.
    let spread = f64::from(spread) / 2.0;
    let ratio = (1.0 + spread) / (1.0 - spread);
    assert!(
        empowered as f64 >= expected / ratio - 3.0 && empowered as f64 <= expected * ratio + 3.0,
        "native MOTE amount{percent}%: ordinary{ordinary}, empowered{empowered}, expected{expected}, variance{spread}"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_master_of_elements_does_not_proc_from_lightning() {
    let mut run = fixture();
    assert!(own_aura(&run, MOTE).is_none());
    cast(&mut run, 188196);
    assert!(
        own_aura(&run, MOTE).is_none(),
        "only normal Lava Burst earns MOTE, not Lightning Bolt"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_mote_is_consumed_by_foreground_lightning() {
    let mut run = fixture();
    assert!(own_aura(&run, MOTE).is_none());
    cast(&mut run, 51505);
    assert!(own_aura(&run, MOTE).is_some(), "real Lava Burst earns MOTE");
    cast(&mut run, 188196);
    assert!(
        own_aura(&run, MOTE).is_none(),
        "next affected foreground spell consumes MOTE"
    );
}

fn lightning_damage(keep_mote: bool) -> DamageSample {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, percent) = prepare_mote(keep_mote);
    cast(&mut run, 188196);
    (
        damage(&run, 188196, false),
        percent,
        variance(&run, 188196, 0),
    )
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_mote_native_damage_survives_consumption_and_missile_flight() {
    assert_mote_damage(lightning_damage(false), lightning_damage(true));
}

fn flame_tick(keep_mote: bool) -> DamageSample {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, percent) = prepare_mote(keep_mote);
    cast(&mut run, 188389);
    for _ in 0..50 {
        run.app.update();
    }
    (
        damage(&run, 188389, true),
        percent,
        variance(&run, 188389, 1),
    )
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_mote_flame_shock_retains_native_damage_on_later_ticks() {
    assert_mote_damage(flame_tick(false), flame_tick(true));
}

fn quake_tick(keep_mote: bool) -> DamageSample {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, percent) = prepare_mote(keep_mote);
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        462620,
        Some(run.targets[0]),
        1,
    );
    // Field damage must survive loss of the earned aura after field creation.
    if keep_mote {
        remove_mote(&mut run);
    }
    for _ in 0..40 {
        run.app.update();
    }
    (
        damage(&run, 77478, false),
        percent,
        variance(&run, 77478, 0),
    )
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_mote_earthquake_retains_native_damage_after_aura_removal() {
    assert_mote_damage(quake_tick(false), quake_tick(true));
}

fn blast_buff_count(run: &Run) -> usize {
    BLAST_BUFFS
        .iter()
        .filter(|&&spell| own_aura(run, spell).is_some())
        .count()
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_blast_background_casts_choose_missing_native_buffs_before_refreshing() {
    let mut run = fixture();
    assert_eq!(blast_buff_count(&run), 0);
    let _seed = crate::random::seed_scope(70448);
    for expected in [1, 2, 3, 3] {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            117014,
            Some(run.targets[0]),
            1,
        );
        assert_eq!(
            blast_buff_count(&run),
            expected,
            "one proc per root; no duplicate while a buff is missing"
        );
    }
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_blast_overload_grants_the_last_missing_native_buff() {
    let mut run = fixture();
    cast(&mut run, 51505);
    assert!(own_aura(&run, MOTE).is_some());
    for &spell in &BLAST_BUFFS[..2] {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.actor),
            1,
        );
    }
    assert_eq!(blast_buff_count(&run), 2);
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        120588,
        Some(run.targets[0]),
        1,
    );
    assert!(
        own_aura(&run, 173184).is_some(),
        "overload grants the only missing buff, too"
    );
    assert!(
        own_aura(&run, MOTE).is_some(),
        "background overload must not spend MOTE"
    );
}
