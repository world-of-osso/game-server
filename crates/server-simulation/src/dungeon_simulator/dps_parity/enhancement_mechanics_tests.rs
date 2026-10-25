//! Native Enhancement production boundaries, independent of APL selection.
use super::*;
use crate::auras::ActiveAuras;

const MAELSTROM_WEAPON: u32 = 344179;
const VOLTAIC_BLAZE: u32 = 470057;
const LIGHTNING_BOLT: u32 = 188196;

fn start_melee(run: &mut Run) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
}

fn cast(run: &mut Run, spell: u32) {
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => return,
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(error) => panic!("native spell{spell} rejected: {error:?}"),
        }
    }
    panic!("native spell{spell} did not become ready");
}

fn stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == MAELSTROM_WEAPON && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_native_melee_can_earn_maelstrom_weapon() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    start_melee(&mut run);
    let mut earned = false;
    for _ in 0..6000 {
        run.app.update();
        earned |= stacks(&run) > 0;
    }
    assert!(
        earned,
        "native melee events must be eligible for Maelstrom Weapon's native20% proc"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_voltaic_blaze_grants_native_maelstrom_weapon_instead_of_zero_power() {
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    assert_eq!(stacks(&run), 0);
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(VOLTAIC_BLAZE)
        .unwrap();
    let base = spell
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    let points = crate::auras::spell_mods::modify(
        world,
        run.actor,
        spell,
        crate::auras::SpellModOp::Points,
        base,
    );
    let native_amount = crate::auras::spell_mods::modify(
        world,
        run.actor,
        spell,
        crate::auras::SpellModOp::PointsIndex1,
        points,
    ) as u8;
    cast(&mut run, VOLTAIC_BLAZE);
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none(),
        "Voltaic Blaze grants its stacks on its native instant execution"
    );
    // Measure this execute before a later frame can earn unrelated real melee procs.
    assert_eq!(stacks(&run), native_amount);
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_real_maelstrom_weapon_makes_bolt_instant_and_is_consumed() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    start_melee(&mut run);
    for _ in 0..6000 {
        run.app.update();
        if stacks(&run) == 10 {
            break;
        }
    }
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    assert_eq!(
        stacks(&run),
        10,
        "ordinary melee must earn the native counter and its modifiers"
    );
    cast(&mut run, LIGHTNING_BOLT);
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none(),
        "at least five MW stacks must make the real Bolt instant"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        stacks(&run),
        0,
        "the native foreground Bolt must spend its counter"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_native_weapon_imbues_persist_and_deal_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(263, 1).unwrap();
    for spell in [319773, 319778] {
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, spell),
            "precombat must apply native weapon imbue {spell}"
        );
    }
    engage(&mut run);
    start_melee(&mut run);
    for _ in 0..1200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for spell in [25504, 10444] {
        assert!(
            events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0
            }),
            "ordinary melee must trigger native weapon damage {spell}"
        );
    }
}
