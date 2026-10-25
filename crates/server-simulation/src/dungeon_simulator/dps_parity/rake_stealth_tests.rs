//! Actual native selected-profile Prowl→Rake; TC spell_dru_rake2040–2078.
use super::*;

fn wait_native_gcd(run: &mut Run, actor: Entity, spell_id: u32) {
    for _ in 0..100 {
        let world = run.app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(spell_id)
            .unwrap();
        if world
            .get::<crate::spell_cast::SpellHistory>(actor)
            .is_none_or(|history| !history.on_gcd(spell, crate::spell_cast::now(world)))
        {
            return;
        }
        run.app.update();
    }
    panic!("native GCD for{spell_id} did not expire in100frames");
}

fn rake_initial_damage(prowl: bool) -> (f64, f64) {
    let mut run = prepare(103, 1).unwrap();
    let actor = run.actor;
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), actor, encounter).unwrap();
    for legacy in [231052, 405834] {
        assert!(!crate::auras::has_aura(run.app.world(), actor, legacy));
    }
    for target in [actor, fixture_player(&run)] {
        if !crate::auras::has_aura(run.app.world(), target, 1126) {
            wait_native_gcd(&mut run, actor, 1126);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1126, Some(target), false)
                .expect("ordinary native Mark of the Wild before Prowl");
        }
    }
    wait_native_gcd(&mut run, actor, 1822);
    if prowl {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5215, Some(actor), false)
            .expect("ordinary native Prowl cast");
        assert!(crate::auras::has_aura(run.app.world(), actor, 5215));
    }
    wait_native_gcd(&mut run, actor, 1822);
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    let source = run.app.world().resource::<crate::spell_info::SpellStore>();
    let bonus = source
        .get(1822)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 3)
        .unwrap()
        .base_points;
    if prowl {
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 5215),
            "native Prowl survives the ordinary GCD wait"
        );
    }
    let actor_position = *run
        .app
        .world()
        .get::<shared::components::Position>(actor)
        .unwrap();
    {
        let mut target_position = run
            .app
            .world_mut()
            .get_mut::<shared::components::Position>(run.targets[0])
            .unwrap();
        target_position.x = actor_position.x + 1.5;
        target_position.y = actor_position.y;
        target_position.z = actor_position.z;
    }
    let _seed = crate::random::seed_scope(70448);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        actor,
        1822,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary native Rake cast");
    let combat = report::combat(run.app.world(), &run.targets, 1.0);
    let damage = combat["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| action["spell_id"] == 1822)
        .map(|action| action["damage"].as_f64().unwrap())
        .sum();
    (damage, 1.0 + f64::from(bonus) / 100.0)
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_literal_native_selected_profile_prowl_rake_uses_authored_stealth_bonus() {
    let (normal, multiplier) = rake_initial_damage(false);
    let (stealth, _) = rake_initial_damage(true);
    assert!(normal > 0.0 && stealth > 0.0, "both real casts must land");
    assert!(
        (stealth - normal * multiplier).abs() <= 2.0,
        "native selected-profile Rake: normal{normal}, Prowl{stealth}, authored factor{multiplier}"
    );
}
