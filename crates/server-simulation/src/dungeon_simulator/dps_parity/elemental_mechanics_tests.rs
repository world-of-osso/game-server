//! Native Elemental cast/flight/aura boundaries, not inferred damage coefficients.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const MOTE: u32 = 260734;
const BLAST_BUFFS: &[u32] = &[118522, 173183, 173184];
type DamageSample = (u64, f32, f32);

fn fixture() -> Run {
    let mut run = prepare_literal(262, 1).unwrap();
    engage(&mut run);
    run
}

fn settle(run: &mut Run) {
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

fn cast(run: &mut Run, spell: u32) {
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

fn own_aura(run: &Run, spell: u32) -> Option<&crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
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
