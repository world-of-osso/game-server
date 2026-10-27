//! Ruby Essence Burst rolls once per actual Living Flame hit, including
//! Leaping Flames uses that heal the caster.
use super::*;

const LIVING_FLAME: u32 = 361469;
const LIVING_FLAME_HEAL: u32 = 361509;
const ESSENCE_BURST: u32 = 359618;

fn cast_living_flame_after_fire_breath(targets: usize) -> (Run, usize) {
    let mut run = prepare_literal(1467, targets).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    crate::spell_cast::creature_cast_empowered_with_targets(
        run.app.world_mut(),
        actor,
        357208,
        crate::spell_cast::CastTargets {
            unit: actor,
            primary: Some(primary),
            destination: None,
        },
        2,
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    // Dragonrage turns every Ruby Essence Burst roll into a grant (sc_evoker.cpp7012).
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        375087,
        Some(actor),
        1
    ));
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == ESSENCE_BURST,
    );
    let before = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        actor,
        LIVING_FLAME,
        crate::spell_cast::CastTargets {
            unit: primary,
            primary: Some(primary),
            destination: None,
        },
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    (run, before)
}

fn essence_burst_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == ESSENCE_BURST))
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires exact native Devastation profile and DB"]
fn evoker_ruby_burst_rolls_for_each_actual_living_flame_damage_hit() {
    let (run, _) = cast_living_flame_after_fire_breath(3);
    // SimC76f8118 sc_evoker.cpp7004–7016: one roll per damage target hit.
    // Rank-two Leaping Flames hits all three real targets: three granted rolls
    // fill the two-use Essence Burst, where one roll per cast grants one use.
    assert_eq!(
        essence_burst_stacks(&run),
        2,
        "three Living Flame hits under Dragonrage must fill Essence Burst"
    );
}

#[test]
#[ignore = "requires exact native Devastation profile and DB"]
fn evoker_ruby_burst_unused_leaping_flames_heal_the_caster() {
    let (run, before) = cast_living_flame_after_fire_breath(1);
    let heals = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[before..]
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.actor.to_bits())
                && event.spell_id == Some(LIVING_FLAME_HEAL)
                && !event.periodic
                && event.kind == shared::protocol::CombatLogKind::Heal
        })
        .count();
    // SimC76f8118 sc_evoker.cpp6936–6951: each Leaping Flames use left after the
    // single real enemy is hit executes a Living Flame heal on the caster.
    assert_eq!(
        heals, 2,
        "two unused Leaping Flames must each heal the caster"
    );
}
