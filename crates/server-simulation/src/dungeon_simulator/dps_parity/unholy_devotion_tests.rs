//! Real Putrefy arrivals grant independent ghoul speed and Energy stacks.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn observe_devotion(arrivals: usize) -> (f64, f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    engage(&mut run);
    let owner = run.actor;
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        owner,
        191587,
        Some(run.targets[0]),
        1,
    ));
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), owner).unwrap();
    for _ in 0..arrivals {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            owner,
            1277098,
            Some(owner),
            1
        ));
    }
    if arrivals > 0 {
        for _ in 0..120 {
            run.app.update();
            let count = run
                .app
                .world()
                .get::<ActiveAuras>(pet)
                .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 1270491))
                .map_or(0, |aura| aura.stacks as usize);
            if count == arrivals {
                break;
            }
        }
        let aura = run
            .app
            .world()
            .get::<ActiveAuras>(pet)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1270491)
            .expect("real Putrefy arrivals grant devotion");
        assert_eq!(aura.stacks as usize, arrivals);
        assert_eq!(aura.stack_expirations().len(), arrivals);
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        owner,
        literal_rotation::execution::Execution::Spell(1233448, owner, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let mut swing_times = Vec::new();
    for step in 0..180 {
        run.app.update();
        let mut events = run.app.world_mut().resource_mut::<ObservedCombatEvents>();
        for event in events.0.drain(..) {
            if event.source != Some(pet.to_bits()) || event.kind != CombatLogKind::Damage {
                continue;
            }
            if event.spell_id.is_none() {
                swing_times.push(step as f64 * 0.05);
            }
        }
        if swing_times.len() >= 2 {
            break;
        }
    }
    assert!(
        swing_times.len() >= 2,
        "real ghoul must swing: {swing_times:?}"
    );
    let amount = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1270491)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .base_points;
    server_spell_scripts::test_support::set_ghoul_energy(run.app.world_mut(), pet, 0.0);
    let start = crate::auras::aura_now(run.app.world());
    let mut expected = 0.0;
    let mut previous = start;
    for _ in 0..20 {
        run.app.update();
        let world = run.app.world();
        let now = crate::auras::aura_now(world);
        let haste = world
            .get::<crate::stats::CharacterStats>(owner)
            .unwrap()
            .haste_pct;
        expected += (now - previous)
            * 10.0
            * (1.0 + f64::from(haste) / 100.0)
            * (1.0 + f64::from(amount) * arrivals as f64 / 100.0);
        previous = now;
    }
    let energy = server_spell_scripts::test_support::ghoul_energy(run.app.world(), pet);
    (swing_times[1] - swing_times[0], energy, expected)
}

#[test]
#[ignore = "requires native252 profile and permanent/lesser ghoul templates"]
fn unholy_devotion_putrefy_stacks_speed_real_ghoul_swings_and_energy() {
    let base = observe_devotion(0);
    let one = observe_devotion(1);
    let two = observe_devotion(2);
    assert!(
        one.0 < base.0 && two.0 < one.0,
        "each real stack speeds auto attacks: {base:?} {one:?} {two:?}"
    );
    for (stacks, (_, energy, expected)) in [base, one, two].into_iter().enumerate() {
        assert!(
            (energy - expected).abs() < 0.1,
            "{stacks} real stacks must regenerate native Energy: actual={energy}, expected={expected}"
        );
    }
}
