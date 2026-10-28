//! Actual Magus conversion gives the Lord its selected native talent lifetime.
use super::*;
use crate::spell_cast::summon::SummonedCreature;

#[test]
#[ignore = "requires native252 profile and real Magus/Lord summons"]
fn unholy_lord_lifetime_follows_selected_talent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    let owner = run.actor;
    let mut lord = None;
    for _ in 0..2 {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            owner,
            literal_rotation::execution::Execution::Spell(
                1247378,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        for _ in 0..50 {
            run.app.update();
            lord = {
                let world = run.app.world_mut();
                world
                    .query::<(Entity, &SummonedCreature)>()
                    .iter(world)
                    .find_map(|(entity, summon)| {
                        (summon.owner == owner && summon.spell_id == 1292072).then_some(entity)
                    })
            };
            if lord.is_some() {
                break;
            }
        }
        if lord.is_some() {
            break;
        }
    }
    let lord = lord.expect("real Magi must combine into the Lord");
    let now = crate::auras::aura_now(run.app.world());
    let expiry = run
        .app
        .world()
        .get::<SummonedCreature>(lord)
        .unwrap()
        .expires_at
        .unwrap();
    let native_ms =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), owner, 1256813, 1);
    assert!(native_ms > 0.0);
    assert!(
        (expiry - now - f64::from(native_ms) / 1000.0).abs() < 0.1,
        "Lord lifetime must use selected talent effect2, not summon row duration"
    );
    while crate::auras::aura_now(run.app.world()) < expiry - 0.1 {
        run.app.update();
        assert!(
            run.app.world().get_entity(lord).is_ok(),
            "Lord expired before its talent duration"
        );
    }
    for _ in 0..10 {
        run.app.update();
        if run.app.world().get_entity(lord).is_err() {
            return;
        }
    }
    panic!("Lord must despawn at its talent deadline");
}
