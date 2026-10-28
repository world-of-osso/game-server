//! Gaze of the Alnseer must earn native stat stacks from actual equipped-item procs.
use super::*;
use crate::auras::ActiveAuras;

fn stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1266687 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

fn hit(run: &mut Run) {
    let actor = run.actor;
    let target = run.targets[0];
    let count = |run: &Run| run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter().filter(|event| event.source == Some(actor.to_bits()) && event.target == Some(target.to_bits()) && event.spell_id == Some(361500) && event.kind == shared::protocol::CombatLogKind::Damage && event.amount > 0).count();
    let before = count(run);
    for _ in 0..5 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 361500, Some(target), 1);
        for _ in 0..2 { run.app.update(); }
        if count(run) > before { return; }
    }
    panic!("five native Living Flame attempts produced no landed damage");
}

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_equipped_gaze_earns_stat_stacks_and_refresh_bypasses_one_icd() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(1467, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    let position = *run
        .app
        .world()
        .get::<shared::components::Position>(actor)
        .unwrap();
    *run.app
        .world_mut()
        .get_mut::<shared::components::Position>(target)
        .unwrap() = position;
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), actor, target);
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1256896),
        "the real profile equips Gaze"
    );
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .primary
        .intellect;
    for _ in 0..500 {
        hit(&mut run);
        if stacks(&run) > 0 {
            break;
        }
        for _ in 0..2 {
            run.app.update();
        }
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1266686),
        "actual equipped Gaze earns Alnsight"
    );
    assert!(
        stacks(&run) > 0,
        "actual damage while Alnsight is active earns native stat stacks"
    );
    let value = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1266687 && aura.caster == actor)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let world = run.app.world();
    let level = world
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1266686 && aura.caster == actor)
        .unwrap()
        .cast_item_level;
    let mut driver = world
        .resource::<crate::spell_info::SpellStore>()
        .get(1256896)
        .unwrap()
        .clone();
    driver.cast_item_level = level;
    let effect = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    let expected = crate::spell_cast::effects::modded_base_points(world, actor, &driver, effect);
    assert!(
        expected > 0.0 && (value - expected).abs() < 0.01,
        "earned stat uses the equipped native driver value: actual={value}, expected={expected}"
    );
    assert!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .primary
            .intellect
            > before
    );
    let first = stacks(&run);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 1266686,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    hit(&mut run);
    assert_eq!(stacks(&run), first, "the existing cooldown must finish after refresh");
    for _ in 0..20 { run.app.update(); }
    let aura = run.app.world().get::<ActiveAuras>(actor).unwrap().iter().find(|aura| aura.spell_id == 1266686 && aura.caster == actor).unwrap();
    let instance = aura.instance_id;
    let deadline = crate::auras::procs::proc_deadline(run.app.world(), actor, instance).unwrap();
    let now = crate::spell_cast::now(run.app.world());
    let level = aura.cast_item_level;
    hit(&mut run);
    let regular = stacks(&run);
    let after = crate::auras::procs::proc_deadline(run.app.world(), actor, instance);
    assert_eq!(regular, first + 1, "the first post-refresh proc starts its normal cooldown: now={now} deadline={deadline} after={after:?} level={level:?}");
    hit(&mut run);
    let bypassed = stacks(&run);
    assert_eq!(bypassed, regular + 1, "the following real hit bypasses that cooldown once");
    hit(&mut run);
    assert_eq!(stacks(&run), bypassed, "the normal cooldown resumes after the bypass");
}
