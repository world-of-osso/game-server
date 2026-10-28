//! Outnumber follows real lesser ghoul births and departures.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::summon::SummonedCreature;

#[test]
#[ignore = "requires native252 profile and lesser ghoul templates"]
fn unholy_outnumber_tracks_actual_lesser_ghoul_lifetimes() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1241705));
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let before = crate::auras::summon_damage::multiplier(run.app.world(), pet);
    for _ in 0..2 {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            275430,
            Some(run.actor),
            1
        ));
    }
    let actual = {
        let world = run.app.world_mut();
        world
            .query::<&SummonedCreature>()
            .iter(world)
            .filter(|summon| summon.owner == run.actor && summon.spell_id == 275430)
            .count()
    };
    assert_eq!(actual, 2, "two real lesser ghouls must exist");
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1242998 && aura.caster == run.actor)
        .unwrap();
    assert_eq!(usize::from(aura.stacks), actual);
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    assert!(
        amount > 0.0,
        "selected native Outnumber modifies the counter"
    );
    let after = crate::auras::summon_damage::multiplier(run.app.world(), pet);
    assert!(
        (after / before - (1.0 + actual as f32 * amount / 100.0)).abs() < 0.0001,
        "real ghoul count must change controlled-pet damage: {before} -> {after}"
    );
    for _ in 0..800 {
        run.app.update();
        if !crate::auras::has_aura(run.app.world(), run.actor, 1242998) {
            return;
        }
    }
    panic!("counter must end when both real lesser ghouls expire");
}
